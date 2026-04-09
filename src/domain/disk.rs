//! Disk identity and status models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a disk (hash-based)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiskId(String);

impl DiskId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for DiskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Physical identity information for disk recognition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskIdentity {
    /// Device serial number (highest priority)
    pub serial: Option<String>,
    /// Volume UUID (second priority)
    pub volume_uuid: Option<String>,
    /// Volume label shown in filesystem
    pub volume_label: Option<String>,
    /// Total capacity in bytes
    pub capacity_bytes: u64,
    /// Fingerprint: hash of (label + capacity + first_registered timestamp)
    /// Used as fallback identity when serial/uuid unavailable
    pub fingerprint: String,
}

impl DiskIdentity {
    /// Check if this identity matches another, using priority order:
    /// serial > volume_uuid > fingerprint
    pub fn matches(&self, other: &DiskIdentity) -> bool {
        // Serial match (highest confidence)
        if let (Some(s1), Some(s2)) = (&self.serial, &other.serial) {
            if s1 == s2 {
                return true;
            }
        }

        // Volume UUID match (second confidence)
        if let (Some(u1), Some(u2)) = (&self.volume_uuid, &other.volume_uuid) {
            if u1 == u2 {
                return true;
            }
        }

        // Fingerprint match (fallback): compare volume_label + capacity_bytes
        // This is what fingerprint encoded, but without timestamp dependency
        self.volume_label.is_some()
            && self.volume_label == other.volume_label
            && self.capacity_bytes == other.capacity_bytes
    }

    /// Check if this identity matches another with tolerance for minor differences
    /// Useful for cases where disk reformatting changed some attributes
    pub fn matches_with_tolerance(&self, other: &DiskIdentity) -> MatchResult {
        // Serial match (highest confidence)
        if let (Some(s1), Some(s2)) = (&self.serial, &other.serial) {
            if s1 == s2 {
                return MatchResult::Exact("Serial number match".to_string());
            }
        }

        // Volume UUID match (second confidence)
        if let (Some(u1), Some(u2)) = (&self.volume_uuid, &other.volume_uuid) {
            if u1 == u2 {
                return MatchResult::Exact("Volume UUID match".to_string());
            }
        }

        // Volume label match with capacity tolerance
        if let (Some(l1), Some(l2)) = (&self.volume_label, &other.volume_label) {
            if self.labels_match(l1, l2) {
                // Check capacity with tolerance (±5%)
                let capacity_tolerance = 0.05;
                let min_cap = self.capacity_bytes as f64 * (1.0 - capacity_tolerance);
                let max_cap = self.capacity_bytes as f64 * (1.0 + capacity_tolerance);

                if other.capacity_bytes as f64 >= min_cap && other.capacity_bytes as f64 <= max_cap {
                    return MatchResult::Tolerant(format!(
                        "Volume label '{}' match with capacity tolerance ({} vs {})",
                        l1, format_capacity(self.capacity_bytes), format_capacity(other.capacity_bytes)
                    ));
                }
            }
        }

        // Capacity-only match (lowest confidence, only if labels are similar)
        if self.volume_label.is_some() && other.volume_label.is_some() {
            if let (Some(l1), Some(l2)) = (&self.volume_label, &other.volume_label) {
                if self.labels_similar(l1, l2) {
                    let capacity_tolerance = 0.10; // 10% tolerance for similar labels
                    let min_cap = self.capacity_bytes as f64 * (1.0 - capacity_tolerance);
                    let max_cap = self.capacity_bytes as f64 * (1.0 + capacity_tolerance);

                    if other.capacity_bytes as f64 >= min_cap && other.capacity_bytes as f64 <= max_cap {
                        return MatchResult::Weak(format!(
                            "Similar labels '{}'/'{}' with matching capacity",
                            l1, l2
                        ));
                    }
                }
            }
        }

        MatchResult::None(self.diagnose_mismatch(other))
    }

    /// Check if labels match exactly (case-sensitive)
    fn labels_match(&self, l1: &str, l2: &str) -> bool {
        l1 == l2
    }

    /// Check if labels are similar (case-insensitive, ignoring minor differences)
    fn labels_similar(&self, l1: &str, l2: &str) -> bool {
        let l1_lower = l1.to_lowercase();
        let l2_lower = l2.to_lowercase();

        // Exact case-insensitive match
        if l1_lower == l2_lower {
            return true;
        }

        // Check if one contains the other (for partial matches)
        if l1_lower.contains(&l2_lower) || l2_lower.contains(&l1_lower) {
            return true;
        }

        // Check similarity with Levenshtein-like comparison (simple version)
        // Similar if first few characters match
        let min_len = std::cmp::min(l1_lower.len(), l2_lower.len());
        if min_len >= 3 {
            let matching_chars = l1_lower.chars().take(min_len)
                .zip(l2_lower.chars().take(min_len))
                .filter(|(a, b)| a == b)
                .count();
            if matching_chars as f64 / min_len as f64 >= 0.7 {
                return true;
            }
        }

        false
    }

    /// Generate diagnostic message for why match failed
    fn diagnose_mismatch(&self, other: &DiskIdentity) -> String {
        let mut reasons = Vec::new();

        // Serial mismatch
        if let (Some(s1), Some(s2)) = (&self.serial, &other.serial) {
            if s1 != s2 {
                reasons.push(format!("Serial differs: '{}' vs '{}'", s1, s2));
            }
        } else if self.serial.is_some() && other.serial.is_none() {
            reasons.push("Registered serial not detected on mount".to_string());
        } else if self.serial.is_none() && other.serial.is_some() {
            reasons.push("New serial detected on mount".to_string());
        }

        // UUID mismatch
        if let (Some(u1), Some(u2)) = (&self.volume_uuid, &other.volume_uuid) {
            if u1 != u2 {
                reasons.push(format!("UUID differs: '{}' vs '{}'", u1, u2));
            }
        }

        // Label mismatch
        if self.volume_label != other.volume_label {
            reasons.push(format!(
                "Label differs: '{}' vs '{}'",
                self.volume_label.as_deref().unwrap_or("none"),
                other.volume_label.as_deref().unwrap_or("none")
            ));
        }

        // Capacity mismatch
        if self.capacity_bytes != other.capacity_bytes {
            reasons.push(format!(
                "Capacity differs: {} vs {}",
                format_capacity(self.capacity_bytes),
                format_capacity(other.capacity_bytes)
            ));
        }

        if reasons.is_empty() {
            "No matching attributes found".to_string()
        } else {
            reasons.join("; ")
        }
    }

    /// Generate fingerprint from label, capacity, and timestamp
    pub fn generate_fingerprint(label: Option<&str>, capacity: u64, registered_at: &DateTime<Utc>) -> String {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(label.unwrap_or("").as_bytes());
        hasher.update(&capacity.to_le_bytes());
        hasher.update(registered_at.to_rfc3339().as_bytes());
        hasher.finalize().to_hex().to_string()
    }
}

/// Result of identity matching
#[derive(Debug, Clone)]
pub enum MatchResult {
    /// Exact match with reason
    Exact(String),
    /// Match with tolerance applied
    Tolerant(String),
    /// Weak match (low confidence)
    Weak(String),
    /// No match with diagnostic message
    None(String),
}

impl MatchResult {
    pub fn is_match(&self) -> bool {
        !matches!(self, MatchResult::None(_))
    }

    pub fn confidence(&self) -> f64 {
        match self {
            MatchResult::Exact(_) => 1.0,
            MatchResult::Tolerant(_) => 0.8,
            MatchResult::Weak(_) => 0.5,
            MatchResult::None(_) => 0.0,
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            MatchResult::Exact(s) | MatchResult::Tolerant(s) | MatchResult::Weak(s) | MatchResult::None(s) => s,
        }
    }
}

/// Format capacity for display in diagnostic messages
fn format_capacity(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Mount status of a disk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MountStatus {
    /// Disk is currently mounted and accessible
    Connected,
    /// Disk is registered but not currently mounted
    Offline,
    /// A disk with conflicting identity is mounted
    IdentityConflict,
}

impl std::fmt::Display for MountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MountStatus::Connected => write!(f, "Connected"),
            MountStatus::Offline => write!(f, "Offline"),
            MountStatus::IdentityConflict => write!(f, "Identity Conflict"),
        }
    }
}

/// Complete disk registration record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disk {
    pub disk_id: DiskId,
    /// User-friendly name (e.g., "素材盘-01")
    pub name: String,
    pub identity: DiskIdentity,
    /// Timestamp when disk was first registered
    pub first_registered: DateTime<Utc>,
    /// Last known mount point
    pub last_mount_point: Option<String>,
    /// Current mount status (computed, not stored)
    pub mount_status: MountStatus,
    /// Current mount point if connected
    pub current_mount_point: Option<String>,
}

impl Disk {
    /// Create a new disk registration
    pub fn new(
        disk_id: DiskId,
        name: String,
        identity: DiskIdentity,
    ) -> Self {
        Self {
            disk_id,
            name,
            identity,
            first_registered: Utc::now(),
            last_mount_point: None,
            mount_status: MountStatus::Offline,
            current_mount_point: None,
        }
    }

    /// Get remaining space at current mount point
    pub fn available_space(&self) -> Option<u64> {
        // TODO: Query filesystem for available space
        None
    }

    /// Get used space ratio
    pub fn usage_ratio(&self) -> Option<f64> {
        // TODO: Calculate from available/total
        None
    }
}