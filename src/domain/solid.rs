//! Solid and SolidLayer rules

use serde::{Deserialize, Serialize};
use std::path::Path;

/// SolidLayer depth specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolidLayerDepth {
    /// No splitting allowed (entire input as one unit)
    Zero,
    /// Split to first level subdirectories
    One,
    /// Split to second level subdirectories
    Two,
    /// Split to N level subdirectories
    N(u32),
    /// Split all the way to individual files
    Infinite,
}

impl SolidLayerDepth {
    /// Parse from string (0, 1, 2, n, inf)
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "0" => Ok(SolidLayerDepth::Zero),
            "1" => Ok(SolidLayerDepth::One),
            "2" => Ok(SolidLayerDepth::Two),
            "inf" | "infinite" => Ok(SolidLayerDepth::Infinite),
            n => {
                let num: u32 = n.parse()
                    .map_err(|_| format!("Invalid SolidLayer value: {}", s))?;
                if num > 2 {
                    Ok(SolidLayerDepth::N(num))
                } else {
                    Err(format!("Use '0', '1', '2', 'n', or 'inf' instead of {}", s))
                }
            }
        }
    }

    /// Get the minimum atomic unit depth
    pub fn min_depth(&self) -> u32 {
        match self {
            SolidLayerDepth::Zero => 0,
            SolidLayerDepth::One => 1,
            SolidLayerDepth::Two => 2,
            SolidLayerDepth::N(n) => *n,
            SolidLayerDepth::Infinite => u32::MAX,
        }
    }

    /// Check if splitting is allowed at a given depth
    pub fn can_split_at(&self, depth: u32) -> bool {
        self.min_depth() <= depth
    }
}

impl std::fmt::Display for SolidLayerDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolidLayerDepth::Zero => write!(f, "0"),
            SolidLayerDepth::One => write!(f, "1"),
            SolidLayerDepth::Two => write!(f, "2"),
            SolidLayerDepth::N(n) => write!(f, "{}", n),
            SolidLayerDepth::Infinite => write!(f, "inf"),
        }
    }
}

impl Default for SolidLayerDepth {
    fn default() -> Self {
        SolidLayerDepth::Zero
    }
}

/// An atomic storage unit that cannot be split across disks
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AtomicUnit {
    /// Root path of this unit (absolute path on source)
    pub root_path: String,
    /// Display name (just the file/dir name)
    pub name: String,
    /// Path relative to the input root directory (preserves directory structure)
    /// When solid=inf, this contains the full relative path like "parent/child/file.txt"
    pub relative_path: String,
    /// Total size in bytes
    pub size: u64,
    /// Depth from original input root
    pub depth: u32,
    /// Whether this unit was created because of a Solid marker
    pub is_solid_marked: bool,
    /// List of files in this unit (for size calculation)
    pub file_count: usize,
}

impl AtomicUnit {
    pub fn new(path: impl Into<String>, name: impl Into<String>) -> Self {
        let path = path.into();
        let name = name.into();
        let relative_path = name.clone(); // Default: same as name
        Self {
            root_path: path,
            name,
            relative_path,
            size: 0,
            depth: 0,
            is_solid_marked: false,
            file_count: 0,
        }
    }

    /// Create with explicit relative path (preserves directory structure)
    pub fn with_relative_path(mut self, relative_path: impl Into<String>) -> Self {
        self.relative_path = relative_path.into();
        self
    }

    pub fn with_size(mut self, size: u64, file_count: usize) -> Self {
        self.size = size;
        self.file_count = file_count;
        self
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    pub fn mark_solid(mut self) -> Self {
        self.is_solid_marked = true;
        self
    }
}

/// Trait for checking if a path is marked as Solid
pub trait SolidChecker {
    fn is_solid(&self, path: &Path, disk_id: &crate::domain::disk::DiskId) -> bool;
}