//! Platform disk detection implementations

mod macos;
mod linux;

pub use macos::MacDiskDetector;
pub use linux::LinuxDiskDetector;

use crate::domain::disk::DiskIdentity;
use crate::Result;

/// Platform-specific disk detector trait
pub trait DiskDetector {
    /// Get disk identity from a mount point
    fn detect_identity(&self, mount_point: &str) -> Result<DiskIdentity>;

    /// Get available space at a mount point
    fn available_space(&self, mount_point: &str) -> Result<u64>;

    /// Get total capacity at a mount point
    fn total_capacity(&self, mount_point: &str) -> Result<u64>;

    /// Get device mount points
    fn list_mount_points(&self) -> Result<Vec<String>>;
}

/// Get the platform-specific disk detector
#[cfg(target_os = "macos")]
pub fn get_detector() -> MacDiskDetector {
    MacDiskDetector::new()
}

#[cfg(target_os = "linux")]
pub fn get_detector() -> LinuxDiskDetector {
    LinuxDiskDetector::new()
}
