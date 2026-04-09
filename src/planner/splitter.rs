//! Atomic unit splitting based on SolidLayer rules

use crate::domain::solid::{AtomicUnit, SolidLayerDepth, SolidChecker};
use crate::domain::disk::DiskId;
use crate::Result;
use std::path::Path;
use walkdir::WalkDir;

/// Split an input path into atomic units based on SolidLayer rules
pub fn split_into_atomic_units(
    root_path: &Path,
    solid_layer: SolidLayerDepth,
    solid_checker: Option<&dyn SolidChecker>,
    disk_id: Option<&DiskId>,
) -> Result<Vec<AtomicUnit>> {
    // Single file case
    if root_path.is_file() {
        let size = root_path.metadata()?.len();
        return Ok(vec![
            AtomicUnit::new(root_path.to_string_lossy().to_string(), root_path.file_name().unwrap().to_string_lossy().to_string())
                .with_size(size, 1)
                .with_depth(0)
        ]);
    }

    // Directory case
    let mut units = Vec::new();

    // If solid_layer = 0, entire directory is one unit
    if solid_layer == SolidLayerDepth::Zero {
        let (size, file_count) = calculate_dir_stats(root_path)?;
        return Ok(vec![
            AtomicUnit::new(root_path.to_string_lossy().to_string(), root_path.file_name().unwrap().to_string_lossy().to_string())
                .with_size(size, file_count)
                .with_depth(0)
        ]);
    }

    // Otherwise, split based on depth
    split_recursive(
        root_path,
        0,
        solid_layer.min_depth(),
        solid_checker,
        disk_id,
        &mut units,
    )?;

    Ok(units)
}

fn split_recursive(
    dir: &Path,
    current_depth: u32,
    target_depth: u32,
    solid_checker: Option<&dyn SolidChecker>,
    disk_id: Option<&DiskId>,
    units: &mut Vec<AtomicUnit>,
) -> Result<()> {
    for entry in WalkDir::new(dir).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        // Check if this entry is marked as Solid
        let is_solid = solid_checker.map_or(false, |checker| checker.is_solid(path, disk_id.unwrap()));

        if path.is_file() {
            // Files are always atomic units at target depth
            let size = path.metadata()?.len();
            units.push(
                AtomicUnit::new(path.to_string_lossy().to_string(), name)
                    .with_size(size, 1)
                    .with_depth(current_depth + 1)
                    .mark_solid()
            );
        } else if is_solid {
            // Solid directories cannot be split further
            let (size, file_count) = calculate_dir_stats(path)?;
            units.push(
                AtomicUnit::new(path.to_string_lossy().to_string(), name)
                    .with_size(size, file_count)
                    .with_depth(current_depth + 1)
                    .mark_solid()
            );
        } else if current_depth >= target_depth - 1 {
            // At target depth, directories become atomic units
            let (size, file_count) = calculate_dir_stats(path)?;
            units.push(
                AtomicUnit::new(path.to_string_lossy().to_string(), name)
                    .with_size(size, file_count)
                    .with_depth(current_depth + 1)
            );
        } else {
            // Continue recursion
            split_recursive(path, current_depth + 1, target_depth, solid_checker, disk_id, units)?;
        }
    }

    Ok(())
}

/// Calculate total size and file count of a directory
fn calculate_dir_stats(dir: &Path) -> Result<(u64, usize)> {
    let mut total_size = 0u64;
    let mut file_count = 0usize;

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            total_size += entry.metadata()?.len();
            file_count += 1;
        }
    }

    Ok((total_size, file_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};
    use std::io::Write;

    struct NoSolidChecker;
    impl SolidChecker for NoSolidChecker {
        fn is_solid(&self, _: &Path, _: &DiskId) -> bool { false }
    }

    #[test]
    fn test_split_single_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        File::create(&file_path).unwrap().write_all(b"hello").unwrap();

        let units = split_into_atomic_units(&file_path, SolidLayerDepth::Infinite, None, None).unwrap();
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].size, 5);
        assert_eq!(units[0].file_count, 1);
    }

    #[test]
    fn test_split_directory_zero() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("sub1")).unwrap();
        fs::create_dir(temp.path().join("sub2")).unwrap();
        File::create(temp.path().join("sub1/a.txt")).unwrap().write_all(b"a").unwrap();
        File::create(temp.path().join("sub2/b.txt")).unwrap().write_all(b"b").unwrap();

        let units = split_into_atomic_units(temp.path(), SolidLayerDepth::Zero, None, None).unwrap();
        assert_eq!(units.len(), 1); // Entire directory as one unit
    }

    #[test]
    fn test_split_directory_one() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("sub1")).unwrap();
        fs::create_dir(temp.path().join("sub2")).unwrap();
        File::create(temp.path().join("sub1/a.txt")).unwrap().write_all(b"aaa").unwrap();
        File::create(temp.path().join("sub2/b.txt")).unwrap().write_all(b"bb").unwrap();

        let checker = NoSolidChecker;
        let units = split_into_atomic_units(temp.path(), SolidLayerDepth::One, Some(&checker), Some(&DiskId::new("test"))).unwrap();
        assert_eq!(units.len(), 2); // Two subdirectories as separate units
    }
}