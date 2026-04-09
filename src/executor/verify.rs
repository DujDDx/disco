//! Hash verification after copy

use crate::{Result, DiscoError};
use std::path::Path;

/// Verify that two files have identical content by comparing hashes
pub fn verify_copy(source: &Path, dest: &Path) -> Result<()> {
    let source_hash = crate::index::hasher::hash_file(source)?;
    let dest_hash = crate::index::hasher::hash_file(dest)?;

    if source_hash != dest_hash {
        return Err(DiscoError::TaskFailed(
            format!("Hash mismatch: source={}, dest={}", source_hash, dest_hash)
        ));
    }

    Ok(())
}

/// Verify all files in a copied directory
pub fn verify_dir_copy(source: &Path, dest: &Path) -> Result<usize> {
    use walkdir::WalkDir;

    let mut verified = 0usize;

    for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let relative = entry.path().strip_prefix(source)?;
            let dest_path = dest.join(relative);

            if !dest_path.exists() {
                return Err(DiscoError::TaskFailed(
                    format!("Missing destination file: {}", dest_path.display())
                ));
            }

            verify_copy(entry.path(), &dest_path)?;
            verified += 1;
        }
    }

    Ok(verified)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_verify_matching_files() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("a.txt");
        let file2 = temp.path().join("b.txt");

        File::create(&file1).unwrap().write_all(b"same content").unwrap();
        File::create(&file2).unwrap().write_all(b"same content").unwrap();

        verify_copy(&file1, &file2).unwrap();
    }

    #[test]
    fn test_verify_different_files() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("a.txt");
        let file2 = temp.path().join("b.txt");

        File::create(&file1).unwrap().write_all(b"content a").unwrap();
        File::create(&file2).unwrap().write_all(b"content b").unwrap();

        let result = verify_copy(&file1, &file2);
        assert!(result.is_err());
    }
}