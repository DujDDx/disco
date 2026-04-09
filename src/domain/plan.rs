//! Storage plan models

use serde::{Deserialize, Serialize};
use crate::domain::disk::DiskId;
use crate::domain::solid::AtomicUnit;

/// A single item in the storage plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItem {
    /// The atomic unit to store
    pub unit: AtomicUnit,
    /// Target disk ID
    pub target_disk: DiskId,
    /// Target disk name
    pub target_disk_name: String,
    /// Target relative path on disk
    pub target_relative_path: String,
}

/// Complete storage plan for a batch of inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorePlan {
    /// All items in the plan
    pub items: Vec<PlanItem>,
    /// Total size to be stored
    pub total_size: u64,
    /// Number of files to be stored
    pub total_files: usize,
    /// Whether deduplication was applied
    pub dedup_applied: bool,
    /// Files skipped due to deduplication
    pub skipped_files: usize,
    /// Skipped file descriptions
    pub skipped_descriptions: Vec<String>,
}

impl StorePlan {
    pub fn new(items: Vec<PlanItem>) -> Self {
        let total_size = items.iter().map(|i| i.unit.size).sum();
        let total_files = items.iter().map(|i| i.unit.file_count).sum();
        Self {
            items,
            total_size,
            total_files,
            dedup_applied: false,
            skipped_files: 0,
            skipped_descriptions: Vec::new(),
        }
    }

    pub fn with_dedup(mut self, skipped: Vec<String>) -> Self {
        self.dedup_applied = true;
        self.skipped_files = skipped.len();
        self.skipped_descriptions = skipped;
        self
    }

    /// Check if plan is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get items targeting a specific disk
    pub fn items_for_disk(&self, disk_id: &DiskId) -> Vec<&PlanItem> {
        self.items.iter().filter(|i| i.target_disk == *disk_id).collect()
    }

    /// Calculate space needed on each disk
    pub fn space_per_disk(&self) -> std::collections::HashMap<DiskId, u64> {
        use std::collections::HashMap;
        let mut map: HashMap<DiskId, u64> = HashMap::new();
        for item in &self.items {
            map.entry(item.target_disk.clone())
                .and_modify(|v| *v += item.unit.size)
                .or_insert(item.unit.size);
        }
        map
    }
}