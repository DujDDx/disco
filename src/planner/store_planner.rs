//! Storage planner - Orchestrates splitting and strategy

use crate::domain::solid::{SolidLayerDepth, AtomicUnit, SolidChecker};
use crate::planner::{splitter::split_into_atomic_units, strategy::DiskSelectionStrategy};
use crate::storage::fs::FsAdapter;
use crate::persistence::{entry_repo::EntryRepo};
use crate::Result;
use std::collections::HashMap;
use std::path::Path;

/// Storage planner that orchestrates the planning process
pub struct StorePlanner<'a> {
    entry_repo: &'a EntryRepo<'a>,
    fs_adapter: &'a FsAdapter,
    strategy: &'a dyn DiskSelectionStrategy,
}

impl<'a> StorePlanner<'a> {
    pub fn new(
        entry_repo: &'a EntryRepo<'a>,
        fs_adapter: &'a FsAdapter,
        strategy: &'a dyn DiskSelectionStrategy,
    ) -> Self {
        Self {
            entry_repo,
            fs_adapter,
            strategy,
        }
    }

    /// Generate a storage plan for input paths
    pub fn plan(
        &self,
        input_paths: &[std::path::PathBuf],
        solid_layer: SolidLayerDepth,
        solid_checker: Option<&dyn SolidChecker>,
        mounted_disks: &[crate::domain::disk::Disk],
    ) -> Result<crate::domain::plan::StorePlan> {
        // Get current available space for each connected disk
        let disk_space: HashMap<String, u64> = mounted_disks
            .iter()
            .filter_map(|d| {
                let mount_point = d.current_mount_point.as_ref()?;
                let space = self.fs_adapter.available_space(Path::new(mount_point)).ok()?;
                Some((d.disk_id.as_str().to_string(), space))
            })
            .collect();

        // Split each input path into atomic units
        let mut all_units = Vec::new();
        for input_path in input_paths {
            let units = split_into_atomic_units(
                input_path,
                solid_layer,
                solid_checker,
                None, // disk_id not relevant for new storage
            )?;
            all_units.extend(units);
        }

        // Assign units to disks using strategy
        let items = self.strategy.assign(all_units, mounted_disks, disk_space)?;

        // Build the plan
        Ok(crate::domain::plan::StorePlan::new(items))
    }

    /// Check for duplicates by hash (if dedup enabled)
    pub fn check_duplicates(&self, units: &[AtomicUnit]) -> Result<HashMap<String, String>> {
        let mut duplicates = HashMap::new();

        for unit in units {
            // Calculate hash for files in this unit
            if unit.file_count == 1 {
                let hash = crate::index::hasher::hash_file(Path::new(&unit.root_path))?;
                // Check if hash exists in index
                if let Some(existing) = self.entry_repo.find_by_hash(&hash)? {
                    duplicates.insert(unit.root_path.clone(), format!(
                        "Duplicate of {} on disk {}",
                        existing.relative_path,
                        existing.disk_name
                    ));
                }
            }
        }

        Ok(duplicates)
    }
}