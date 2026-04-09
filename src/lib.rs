//! Disco library - Core modules for multi-disk storage management

pub mod cli;
pub mod domain;
pub mod executor;
pub mod index;
pub mod planner;
pub mod persistence;
pub mod storage;

/// Shared error types for the application
pub mod error {
    use thiserror::Error;

    /// Error severity levels for user-friendly display
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ErrorSeverity {
        Warning,
        Error,
        Critical,
    }

    #[derive(Error, Debug)]
    pub enum DiscoError {
        #[error("Disk not found: {0}")]
        DiskNotFound(String),

        #[error("Disk identity mismatch: expected {expected}, found {found}")]
        DiskIdentityMismatch { expected: String, found: String },

        #[error("Disk not mounted: {0}")]
        DiskNotMounted(String),

        #[error("Entry not found: {0}")]
        EntryNotFound(i64),

        #[error("Atomic unit too large: {size} bytes exceeds all available disk space")]
        AtomicUnitTooLarge { size: u64 },

        #[error("Path not found: {0}")]
        PathNotFound(String),

        #[error("Invalid path: {0}")]
        InvalidPath(String),

        #[error("Solid violation: cannot split directory marked as Solid")]
        SolidViolation,

        #[error("Task interrupted: {0}")]
        TaskInterrupted(String),

        #[error("Task failed: {0}")]
        TaskFailed(String),

        #[error("Database error: {0}")]
        DatabaseError(#[from] rusqlite::Error),

        #[error("IO error: {0}")]
        IoError(#[from] std::io::Error),

        #[error("Configuration error: {0}")]
        ConfigError(String),

        #[error("Platform detection error: {0}")]
        PlatformError(String),

        #[error("Path strip error: {0}")]
        StripPrefixError(#[from] std::path::StripPrefixError),

        #[error("Serialization error: {0}")]
        SerdeError(#[from] serde_json::Error),

        #[error("Walk directory error: {0}")]
        WalkDirError(String),

        #[error("Database migration error: {0}")]
        MigrationError(String),

        #[error("No disks available for storage")]
        NoDisksAvailable,

        #[error("File already exists: {0}")]
        FileAlreadyExists(String),

        #[error("Permission denied: {0}")]
        PermissionDenied(String),

        #[error("Operation cancelled by user")]
        OperationCancelled,
    }

    impl DiscoError {
        /// Get the severity level of this error
        pub fn severity(&self) -> ErrorSeverity {
            match self {
                DiscoError::DiskNotFound(_) |
                DiscoError::EntryNotFound(_) |
                DiscoError::PathNotFound(_) |
                DiscoError::OperationCancelled => ErrorSeverity::Warning,
                DiscoError::DiskNotMounted(_) |
                DiscoError::DiskIdentityMismatch { .. } |
                DiscoError::InvalidPath(_) |
                DiscoError::NoDisksAvailable |
                DiscoError::FileAlreadyExists(_) |
                DiscoError::PermissionDenied(_) |
                DiscoError::TaskInterrupted(_) => ErrorSeverity::Error,
                DiscoError::AtomicUnitTooLarge { .. } |
                DiscoError::SolidViolation |
                DiscoError::TaskFailed(_) |
                DiscoError::DatabaseError(_) |
                DiscoError::IoError(_) |
                DiscoError::ConfigError(_) |
                DiscoError::PlatformError(_) |
                DiscoError::MigrationError(_) => ErrorSeverity::Critical,
                DiscoError::StripPrefixError(_) |
                DiscoError::SerdeError(_) |
                DiscoError::WalkDirError(_) => ErrorSeverity::Error,
            }
        }

        /// Get a user-friendly description (avoiding technical jargon)
        pub fn user_description(&self) -> String {
            match self {
                DiscoError::DiskNotFound(id) => {
                    format!("The disk '{}' is not registered in your disk pool.", id)
                }
                DiscoError::DiskNotMounted(name) => {
                    format!("The disk '{}' is not connected to your computer. Please connect it first.", name)
                }
                DiscoError::DiskIdentityMismatch { expected, found } => {
                    format!("The connected disk appears to be different from the registered one. Expected '{}', but found '{}'.", expected, found)
                }
                DiscoError::EntryNotFound(id) => {
                    format!("Could not find the file with ID {} in the index.", id)
                }
                DiscoError::PathNotFound(path) => {
                    format!("The file or folder '{}' does not exist.", path)
                }
                DiscoError::InvalidPath(path) => {
                    format!("The path '{}' is not valid. Please check the spelling.", path)
                }
                DiscoError::AtomicUnitTooLarge { size } => {
                    let size_gb = *size as f64 / (1024.0 * 1024.0 * 1024.0);
                    format!("The file is too large ({:.2} GB) to fit on any of your connected disks.", size_gb)
                }
                DiscoError::NoDisksAvailable => {
                    "No disks are connected. Please connect at least one disk to your disk pool.".to_string()
                }
                DiscoError::FileAlreadyExists(path) => {
                    format!("A file already exists at '{}'. Choose a different location or use --force to overwrite.", path)
                }
                DiscoError::PermissionDenied(path) => {
                    format!("You don't have permission to access '{}'. Try running with elevated privileges or check the file permissions.", path)
                }
                DiscoError::SolidViolation => {
                    "This folder is marked as 'Solid' and cannot be split across multiple disks. Choose a single disk or remove the Solid marker.".to_string()
                }
                DiscoError::TaskInterrupted(task) => {
                    format!("The {} operation was interrupted. Some files may not have been completely processed.", task)
                }
                DiscoError::TaskFailed(task) => {
                    format!("The {} operation failed. Please check the error details and try again.", task)
                }
                DiscoError::OperationCancelled => {
                    "The operation was cancelled.".to_string()
                }
                DiscoError::DatabaseError(e) => {
                    format!("A database error occurred: {}. Try restarting the application.", e)
                }
                DiscoError::IoError(e) => {
                    format!("A file system error occurred: {}. Check if the disk is properly connected.", e)
                }
                DiscoError::ConfigError(e) => {
                    format!("A configuration error occurred: {}. Check your settings.", e)
                }
                DiscoError::PlatformError(e) => {
                    format!("Could not detect disk information: {}. Make sure the disk is properly mounted.", e)
                }
                DiscoError::MigrationError(e) => {
                    format!("Database upgrade failed: {}. The application may need to be reinstalled.", e)
                }
                _ => self.to_string(),
            }
        }

        /// Get a suggested fix for this error
        pub fn suggestion(&self) -> Option<String> {
            match self {
                DiscoError::DiskNotFound(_) => {
                    Some("Use 'disk list' to see registered disks, or 'disk add' to register a new one.".to_string())
                }
                DiscoError::DiskNotMounted(_) => {
                    Some("Connect the external disk to your computer, then use 'refresh' to update its status.".to_string())
                }
                DiscoError::DiskIdentityMismatch { .. } => {
                    Some("Use 'repair' to update the disk identity or reconnect it.".to_string())
                }
                DiscoError::NoDisksAvailable => {
                    Some("Use 'disk add <mount-point>' to register disks to your pool.".to_string())
                }
                DiscoError::PathNotFound(_) => {
                    Some("Check that the path is correct and the file/folder exists.".to_string())
                }
                DiscoError::InvalidPath(_) => {
                    Some("Make sure the path is absolute (starting with /) and properly formatted.".to_string())
                }
                DiscoError::AtomicUnitTooLarge { .. } => {
                    Some("Add more disks to your pool, or use a different disk with more free space.".to_string())
                }
                DiscoError::SolidViolation => {
                    Some("Use 'solid unset <path>' to remove the Solid marker, or choose a single disk.".to_string())
                }
                DiscoError::PermissionDenied(_) => {
                    Some("Try running with administrator privileges, or check file ownership.".to_string())
                }
                DiscoError::PlatformError(_) => {
                    Some("Make sure the disk is properly connected and mounted in Finder/File Manager.".to_string())
                }
                DiscoError::EntryNotFound(_) => {
                    Some("Use 'search' to find the file, or 'scan' to update the index.".to_string())
                }
                _ => None,
            }
        }
    }

    impl From<rusqlite_migration::Error> for DiscoError {
        fn from(e: rusqlite_migration::Error) -> Self {
            DiscoError::MigrationError(e.to_string())
        }
    }

    impl From<walkdir::Error> for DiscoError {
        fn from(e: walkdir::Error) -> Self {
            DiscoError::WalkDirError(e.to_string())
        }
    }

    pub type Result<T> = std::result::Result<T, DiscoError>;
}

pub use error::{DiscoError, Result, ErrorSeverity};