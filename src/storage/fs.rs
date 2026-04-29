//! File system adapter abstraction

use crate::Result;
use std::path::Path;
use walkdir::WalkDir;
use crate::storage::platform::DiskDetector;

/// File system adapter
pub struct FsAdapter;

impl FsAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl FsAdapter {
    /// Walk a directory and return all file paths
    pub fn walk_directory(&self, dir: &Path) -> Result<Vec<std::path::PathBuf>> {
        let paths: Vec<std::path::PathBuf> = WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_path_buf())
            .collect();
        Ok(paths)
    }

    /// Copy a file
    pub fn copy_file(&self, source: &Path, dest: &Path) -> Result<()> {
        crate::executor::copy::copy_file(source, dest, None)
    }

    /// Copy a directory recursively
    pub fn copy_dir_recursive(&self, source: &Path, dest: &Path) -> Result<u64> {
        crate::executor::copy::copy_dir_recursive(source, dest, None)
    }

    /// Get file size
    pub fn file_size(&self, path: &Path) -> Result<u64> {
        Ok(path.metadata()?.len())
    }

    /// Get directory total size
    pub fn dir_total_size(&self, dir: &Path) -> Result<u64> {
        let total: u64 = WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok().map(|m| m.len()))
            .sum();
        Ok(total)
    }

    /// Get available space at mount point
    pub fn available_space(&self, path: &Path) -> Result<u64> {
        let detector = crate::storage::platform::get_detector();
        let path_str = path.to_string_lossy();
        detector.available_space(&path_str)
    }

    /// Check if path exists
    pub fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_file_size() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        File::create(&file).unwrap().write_all(b"hello").unwrap();

        let adapter = FsAdapter::new();
        let size = adapter.file_size(&file).unwrap();
        assert_eq!(size, 5);
    }

    #[test]
    fn test_dir_total_size() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("sub")).unwrap();
        File::create(temp.path().join("a.txt")).unwrap().write_all(b"aaa").unwrap();
        File::create(temp.path().join("sub/b.txt")).unwrap().write_all(b"bb").unwrap();

        let adapter = FsAdapter::new();
        let size = adapter.dir_total_size(temp.path()).unwrap();
        assert_eq!(size, 5);
    }
}
