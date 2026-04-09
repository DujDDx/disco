//! Disk selection strategies

use crate::domain::disk::{Disk, MountStatus};
use crate::domain::solid::AtomicUnit;
use crate::domain::plan::PlanItem;
use crate::{Result, DiscoError};
use std::collections::HashMap;

/// Disk selection strategy trait
pub trait DiskSelectionStrategy {
    /// Assign atomic units to disks
    /// Returns PlanItems or error if no valid assignment exists
    fn assign(&self, units: Vec<AtomicUnit>, disks: &[Disk], disk_space: HashMap<String, u64>) -> Result<Vec<PlanItem>>;
}

/// Best Fit Decreasing strategy
/// Sort units by size descending, assign each to the disk with least remaining space that can fit it
pub struct BestFitStrategy;

impl BestFitStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BestFitStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl DiskSelectionStrategy for BestFitStrategy {
    fn assign(&self, units: Vec<AtomicUnit>, disks: &[Disk], disk_space: HashMap<String, u64>) -> Result<Vec<PlanItem>> {
        // Filter to only connected disks
        let available_disks: Vec<&Disk> = disks
            .iter()
            .filter(|d| d.mount_status == MountStatus::Connected)
            .collect();

        if available_disks.is_empty() {
            return Err(DiscoError::DiskNotMounted("No disks are currently connected".to_string()));
        }

        // Sort units by size descending (largest first)
        let mut sorted_units = units;
        sorted_units.sort_by(|a, b| b.size.cmp(&a.size));

        // Track remaining space per disk
        let mut remaining_space: HashMap<String, u64> = disk_space.clone();

        let mut plan_items = Vec::new();

        for unit in sorted_units {
            let unit_size = unit.size;

            // Find candidate disks with enough space
            let candidates: Vec<&&Disk> = available_disks
                .iter()
                .filter(|d| {
                    let space = remaining_space.get(d.disk_id.as_str()).unwrap_or(&0);
                    *space >= unit_size
                })
                .collect();

            if candidates.is_empty() {
                return Err(DiscoError::AtomicUnitTooLarge { size: unit_size });
            }

            // Select disk with least remaining space after placement (Best Fit)
            let best_disk = candidates
                .iter()
                .min_by_key(|d| {
                    let space = remaining_space.get(d.disk_id.as_str()).unwrap_or(&0);
                    space - unit_size
                })
                .unwrap();

            // Create plan item
            let target_relative_path = unit.name.clone();
            plan_items.push(PlanItem {
                unit,
                target_disk: best_disk.disk_id.clone(),
                target_disk_name: best_disk.name.clone(),
                target_relative_path,
            });

            // Update remaining space
            let current = remaining_space.get_mut(best_disk.disk_id.as_str()).unwrap();
            *current -= unit_size;
        }

        Ok(plan_items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::disk::{DiskId, DiskIdentity, MountStatus};

    fn create_test_disk(id: &str, name: &str, space: u64) -> Disk {
        Disk {
            disk_id: DiskId::new(id),
            name: name.to_string(),
            identity: DiskIdentity {
                serial: Some(id.to_string()),
                volume_uuid: None,
                volume_label: None,
                capacity_bytes: space * 2,
                fingerprint: "test".to_string(),
            },
            first_registered: chrono::Utc::now(),
            last_mount_point: None,
            mount_status: MountStatus::Connected,
            current_mount_point: Some("/mnt".to_string()),
        }
    }

    #[test]
    fn test_best_fit_simple() {
        let strategy = BestFitStrategy::new();
        let disks = vec![
            create_test_disk("d1", "Disk1", 100),
            create_test_disk("d2", "Disk2", 200),
        ];

        let units = vec![
            AtomicUnit::new("/src/a", "a").with_size(50, 1),
            AtomicUnit::new("/src/b", "b").with_size(80, 1),
        ];

        let disk_space: HashMap<String, u64> = vec![
            ("d1".to_string(), 100u64),
            ("d2".to_string(), 200u64),
        ].into_iter().collect();

        let items = strategy.assign(units, &disks, disk_space).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_best_fit_no_space() {
        let strategy = BestFitStrategy::new();
        let disks = vec![create_test_disk("d1", "Disk1", 100)];
        let units = vec![AtomicUnit::new("/src/a", "a").with_size(200, 1)];
        let disk_space: HashMap<String, u64> = vec![("d1".to_string(), 100u64)].into_iter().collect();

        let result = strategy.assign(units, &disks, disk_space);
        assert!(result.is_err());
    }
}