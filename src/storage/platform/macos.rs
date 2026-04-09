//! macOS disk detection using diskutil

use crate::domain::disk::DiskIdentity;
use crate::{Result, DiscoError};
use crate::storage::platform::DiskDetector;

pub struct MacDiskDetector;

impl MacDiskDetector {
    pub fn new() -> Self {
        Self
    }
}

impl DiskDetector for MacDiskDetector {
    fn detect_identity(&self, mount_point: &str) -> Result<DiskIdentity> {
        // First, get the device node for the mount point
        let device = self.get_device_for_mount(mount_point)?;

        // Use diskutil to get disk info using the device node
        let output = std::process::Command::new("diskutil")
            .arg("info")
            .arg(&device)
            .output()
            .map_err(|e| DiscoError::PlatformError(format!("Failed to run diskutil: {}", e)))?;

        if !output.status.success() {
            // Fallback: try with mount point directly (some external drives work this way)
            let fallback_output = std::process::Command::new("diskutil")
                .arg("info")
                .arg(mount_point)
                .output()
                .map_err(|e| DiscoError::PlatformError(format!("Failed to run diskutil fallback: {}", e)))?;

            if fallback_output.status.success() {
                let stdout = String::from_utf8_lossy(&fallback_output.stdout);
                return parse_diskutil_info(&stdout, mount_point);
            }

            return Err(DiscoError::PlatformError(
                format!("diskutil failed for device {} and mount {}: {}",
                    device, mount_point, String::from_utf8_lossy(&output.stderr))
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_diskutil_info(&stdout, mount_point)
    }

    fn available_space(&self, mount_point: &str) -> Result<u64> {
        // Use df command to get available space
        let output = std::process::Command::new("df")
            .arg("-k")
            .arg(mount_point)
            .output()
            .map_err(|e| DiscoError::PlatformError(format!("Failed to run df: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let available_kb: u64 = parts[3].parse()
                    .map_err(|e| DiscoError::PlatformError(format!("Failed to parse df: {}", e)))?;
                return Ok(available_kb * 1024);
            }
        }

        Err(DiscoError::PlatformError("Could not parse df output".to_string()))
    }

    fn total_capacity(&self, mount_point: &str) -> Result<u64> {
        let output = std::process::Command::new("df")
            .arg("-k")
            .arg(mount_point)
            .output()
            .map_err(|e| DiscoError::PlatformError(format!("Failed to run df: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let total_kb: u64 = parts[1].parse()
                    .map_err(|e| DiscoError::PlatformError(format!("Failed to parse df: {}", e)))?;
                return Ok(total_kb * 1024);
            }
        }

        Err(DiscoError::PlatformError("Could not parse df output".to_string()))
    }

    fn list_mount_points(&self) -> Result<Vec<String>> {
        let mut mounts = Vec::new();

        if std::path::Path::new("/Volumes").exists() {
            for entry in std::fs::read_dir("/Volumes")? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name != "Macintosh HD" {
                    mounts.push(entry.path().to_string_lossy().to_string());
                }
            }
        }

        // Also include home directory mounts and other common locations
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = std::path::PathBuf::from(home);
            // Check for mounted volumes in home directory
            if home_path.exists() {
                for entry in std::fs::read_dir(&home_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() && path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                        // Could be a symlink to a mounted volume
                        if let Ok(target) = std::fs::canonicalize(&path) {
                            if target.starts_with("/Volumes/") {
                                mounts.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(mounts)
    }
}

impl MacDiskDetector {
    /// Get the device node (e.g., disk2s1) for a mount point
    fn get_device_for_mount(&self, mount_point: &str) -> Result<String> {
        // Use df command to get the device
        let output = std::process::Command::new("df")
            .arg(mount_point)
            .output()
            .map_err(|e| DiscoError::PlatformError(format!("Failed to run df: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 1 {
                let device = parts[0];
                // Device should look like /dev/disk2s1 or similar
                if device.starts_with("/dev/") {
                    // Strip /dev/ prefix to get just disk2s1
                    return Ok(device.replace("/dev/", ""));
                }
            }
        }

        // Fallback: try mount command
        let mount_output = std::process::Command::new("mount")
            .output()
            .map_err(|e| DiscoError::PlatformError(format!("Failed to run mount: {}", e)))?;

        let mount_stdout = String::from_utf8_lossy(&mount_output.stdout);
        for line in mount_stdout.lines() {
            if line.contains(mount_point) {
                // Parse mount output: "/dev/disk2s1 on /Volumes/MyDisk (hfs, local, ...)"
                if let Some(device_part) = line.split(" on ").next() {
                    if device_part.starts_with("/dev/") {
                        return Ok(device_part.replace("/dev/", ""));
                    }
                }
            }
        }

        // Last fallback: return the mount point itself (some diskutil versions accept this)
        Ok(mount_point.to_string())
    }
}

fn parse_diskutil_info(output: &str, mount_point: &str) -> Result<DiskIdentity> {
    let mut serial: Option<String> = None;
    let mut volume_uuid: Option<String> = None;
    let mut volume_label: Option<String> = None;
    let mut capacity: Option<u64> = None;

    // Extract volume label from mount point as fallback
    let mount_label = std::path::Path::new(mount_point)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty());

    for line in output.lines() {
        let line = line.trim();

        // Volume Name parsing - handle multiple formats
        if line.starts_with("Volume Name:") {
            let name = line.split(':').nth(1).unwrap_or("").trim();
            if name != "Not applicable" && !name.is_empty() && name != "Not available" {
                volume_label = Some(name.to_string());
            }
        }
        // Also check for "Device / Media Name:" as alternative
        if line.starts_with("Device / Media Name:") && volume_label.is_none() {
            let name = line.split(':').nth(1).unwrap_or("").trim();
            if !name.is_empty() && name != "Not applicable" {
                volume_label = Some(name.to_string());
            }
        }

        // Disk Size parsing - handle multiple formats
        // Format 1: "Disk Size:               500.11 GB (500111175680 Bytes)"
        // Format 2: "Total Size:              500.11 GB"
        if line.starts_with("Disk Size:") || line.starts_with("Total Size:") {
            // Try to extract bytes from parentheses
            if let Some(bytes_part) = line.split('(').nth(1) {
                if let Some(bytes_str) = bytes_part.split(' ').next() {
                    if let Ok(bytes) = bytes_str.replace("Bytes", "").replace(",", "").parse::<u64>() {
                        capacity = Some(bytes);
                    }
                }
            }
            // Fallback: parse GB/TB value
            if capacity.is_none() {
                let size_str = line.split(':').nth(1).unwrap_or("").trim();
                capacity = parse_size_string(size_str);
            }
        }

        // Volume UUID parsing
        if line.contains("Volume UUID:") {
            let uuid = line.split(':').nth(1).unwrap_or("").trim();
            if uuid != "Not applicable" && !uuid.is_empty() && uuid != "Not available" {
                volume_uuid = Some(uuid.to_string());
            }
        }

        // Serial Number parsing - handle multiple field names
        if line.contains("Serial Number:") || line.contains("Device Identifier:") || line.contains("Disk / Partition UUID:") {
            let sn = line.split(':').nth(1).unwrap_or("").trim();
            if !sn.is_empty() && sn != "Not applicable" && sn != "Not available" {
                // For Device Identifier, we may get something like "disk2s1" which is not a real serial
                // Only use it if it looks like a real serial number
                if line.contains("Serial Number:") {
                    serial = Some(sn.to_string());
                } else if serial.is_none() {
                    // Use Device Identifier as fallback serial if nothing else available
                    serial = Some(sn.to_string());
                }
            }
        }

        // Also check for "Solid State:" to identify drive type (informational)
        // and "Device Location:" for connection info
    }

    // Use mount point label as fallback if no label found
    if volume_label.is_none() && mount_label.is_some() {
        volume_label = mount_label.map(|s| s.to_string());
    }

    let capacity_bytes = capacity.unwrap_or(0);
    let fingerprint = String::new();

    Ok(DiskIdentity {
        serial,
        volume_uuid,
        volume_label,
        capacity_bytes,
        fingerprint,
    })
}

/// Parse size string like "500.11 GB" or "1 TB" into bytes
fn parse_size_string(size_str: &str) -> Option<u64> {
    let size_str = size_str.trim();
    let parts: Vec<&str> = size_str.split_whitespace().collect();
    if parts.len() >= 2 {
        let value: f64 = parts[0].parse().ok()?;
        let unit = parts[1].to_uppercase();
        let bytes = match unit.as_str() {
            "TB" | "TIB" => value * 1024.0 * 1024.0 * 1024.0 * 1024.0,
            "GB" | "GIB" => value * 1024.0 * 1024.0 * 1024.0,
            "MB" | "MIB" => value * 1024.0 * 1024.0,
            "KB" | "KIB" => value * 1024.0,
            "B" | "BYTES" => value,
            _ => return None,
        };
        return Some(bytes as u64);
    }
    None
}
