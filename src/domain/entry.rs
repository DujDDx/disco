//! Index entry models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::domain::disk::DiskId;

/// Entry type in the index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    File,
    Dir,
}

impl std::fmt::Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::File => write!(f, "file"),
            EntryType::Dir => write!(f, "dir"),
        }
    }
}

impl std::str::FromStr for EntryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "file" => Ok(EntryType::File),
            "dir" => Ok(EntryType::Dir),
            _ => Err(format!("Invalid entry type: {}", s)),
        }
    }
}

/// Entry status in the index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryStatus {
    /// Entry is confirmed present on disk
    Normal,
    /// Entry was not found during last scan
    Missing,
    /// Disk is offline, entry status pending confirmation
    PendingConfirm,
}

impl std::fmt::Display for EntryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryStatus::Normal => write!(f, "normal"),
            EntryStatus::Missing => write!(f, "missing"),
            EntryStatus::PendingConfirm => write!(f, "pending_confirm"),
        }
    }
}

impl std::str::FromStr for EntryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(EntryStatus::Normal),
            "missing" => Ok(EntryStatus::Missing),
            "pending_confirm" => Ok(EntryStatus::PendingConfirm),
            _ => Err(format!("Invalid entry status: {}", s)),
        }
    }
}

/// File/directory entry in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Unique entry ID (auto-increment)
    pub entry_id: i64,
    /// Owning disk ID
    pub disk_id: DiskId,
    /// Disk name at time of indexing
    pub disk_name: String,
    /// Path relative to disk root
    pub relative_path: String,
    /// File/directory name
    pub file_name: String,
    /// Size in bytes (for files) or total size (for dirs)
    pub size: u64,
    /// BLAKE3 hash (optional)
    pub hash: Option<String>,
    /// Last modification time
    pub mtime: DateTime<Utc>,
    /// Entry type
    pub entry_type: EntryType,
    /// Whether directory is marked as Solid
    pub solid_flag: bool,
    /// Last known mount point
    pub last_seen_mount_point: String,
    /// When this entry was indexed
    pub indexed_at: DateTime<Utc>,
    /// Current status
    pub status: EntryStatus,
}

impl IndexEntry {
    /// Get full path if disk is mounted
    pub fn full_path(&self, mount_point: &str) -> String {
        format!("{}/{}", mount_point.trim_end_matches('/'), self.relative_path)
    }

    /// Check if this entry matches a search keyword
    pub fn matches_keyword(&self, keyword: &str) -> bool {
        self.file_name.to_lowercase().contains(&keyword.to_lowercase())
    }

    /// Get file extension if this is a file
    pub fn extension(&self) -> Option<&str> {
        if self.entry_type != EntryType::File {
            return None;
        }
        self.file_name.rsplit('.').next()
    }
}