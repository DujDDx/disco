//! Linux disk detection using lsblk/blkid

use crate::domain::disk::DiskIdentity;
use crate::{Result, DiscoError};
use crate::storage::platform::DiskDetector;

pub struct LinuxDiskDetector;

impl LinuxDiskDetector {
    pub fn new() -> Self {
        Self
    }
}

impl DiskDetector for LinuxDiskDetector {
    fn detect_identity(&self, mount_point: &str) -> Result<DiskIdentity> {
        let output = std::process::Command::new("findmnt")
            .arg("-n")
            .arg("-o")
            .arg("SOURCE")
            .arg(mount_point)
            .output()
            .map_err(|e| DiscoError::PlatformError(format!("Failed to run findmnt: {}", e)))?;

        let device = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if device.is_empty() {
            return Err(DiscoError::PlatformError(
                format!("Could not find device for mount point: {}", mount_point)
            ));
        }

        let lsblk_output = std::process::Command::new("lsblk")
            .arg("-b")
            .arg("-d")
            .arg("-o")
            .arg("SERIAL,UUID,LABEL,SIZE")
            .arg(&device)
            .output()
            .map_err(|e| DiscoError::PlatformError(format!("Failed to run lsblk: {}", e)))?;

        if !lsblk_output.status.success() {
            return Err(DiscoError::PlatformError(
                format!("lsblk failed: {}", String::from_utf8_lossy(&lsblk_output.stderr))
            ));
        }

        let stdout = String::from_utf8_lossy(&lsblk_output.stdout);
        parse_lsblk_info(&stdout)
    }

    fn available_space(&self, mount_point: &str) -> Result<u64> {
        let output = std::process::Command::new("df")
            .arg("-B1")
            .arg(mount_point)
            .output()
            .map_err(|e| DiscoError::PlatformError(format!("Failed to run df: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                return parts[3].parse::<u64>()
                    .map_err(|e| DiscoError::PlatformError(format!("Failed to parse df: {}", e)));
            }
        }

        Err(DiscoError::PlatformError("Could not parse df output".to_string()))
    }

    fn total_capacity(&self, mount_point: &str) -> Result<u64> {
        let output = std::process::Command::new("df")
            .arg("-B1")
            .arg(mount_point)
            .output()
            .map_err(|e| DiscoError::PlatformError(format!("Failed to run df: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<u64>()
                    .map_err(|e| DiscoError::PlatformError(format!("Failed to parse df: {}", e)));
            }
        }

        Err(DiscoError::PlatformError("Could not parse df output".to_string()))
    }

    fn list_mount_points(&self) -> Result<Vec<String>> {
        let output = std::process::Command::new("findmnt")
            .arg("-l")
            .arg("-n")
            .arg("-o")
            .arg("TARGET")
            .output()
            .map_err(|e| DiscoError::PlatformError(format!("Failed to run findmnt: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mounts: Vec<String> = stdout
            .lines()
            .filter(|l| !l.starts_with("/proc") && !l.starts_with("/sys") && !l.starts_with("/dev"))
            .map(|l| l.trim().to_string())
            .collect();

        Ok(mounts)
    }
}

fn parse_lsblk_info(output: &str) -> Result<DiskIdentity> {
    if let Some(line) = output.lines().skip(1).next() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        let serial = parts.first().filter(|s| !s.is_empty()).map(|s| s.to_string());
        let volume_uuid = parts.get(1).filter(|s| !s.is_empty()).map(|s| s.to_string());
        let volume_label = parts.get(2).filter(|s| !s.is_empty()).map(|s| s.to_string());
        let capacity = parts.get(3).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);

        let fingerprint = String::new();

        return Ok(DiskIdentity {
            serial,
            volume_uuid,
            volume_label,
            capacity_bytes: capacity,
            fingerprint,
        });
    }

    Err(DiscoError::PlatformError("Could not parse lsblk output".to_string()))
}
