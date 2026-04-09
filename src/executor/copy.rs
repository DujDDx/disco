//! File and directory copying with progress

use crate::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Copy progress callback type
pub type ProgressCallback = dyn Fn(u64, u64) + Send + Sync;

/// Copy a single file with progress reporting
pub fn copy_file(
    source: &Path,
    dest: &Path,
    progress_cb: Option<&ProgressCallback>,
) -> Result<()> {
    // Ensure destination directory exists
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let source_size = source.metadata()?.len();
    let source_file = File::open(source)?;
    let dest_file = File::create(dest)?;

    let mut reader = BufReader::with_capacity(64 * 1024, source_file);
    let mut writer = BufWriter::with_capacity(64 * 1024, dest_file);

    let mut copied = 0u64;
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        writer.write_all(&buffer[..bytes_read])?;
        copied += bytes_read as u64;

        if let Some(cb) = progress_cb {
            cb(copied, source_size);
        }
    }

    writer.flush()?;
    Ok(())
}

/// Copy a directory recursively with progress
pub fn copy_dir_recursive(
    source: &Path,
    dest: &Path,
    progress_cb: Option<&ProgressCallback>,
) -> Result<u64> {
    use walkdir::WalkDir;

    // Calculate total size first
    let total_size: u64 = WalkDir::new(source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum();

    // Create destination directory
    fs::create_dir_all(dest)?;

    let mut total_copied = 0u64;

    for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
        let relative = entry.path().strip_prefix(source)?;
        let dest_path = dest.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else if entry.file_type().is_file() {
            let file_size = entry.metadata()?.len();
            copy_file(entry.path(), &dest_path, None)?;
            total_copied += file_size;

            if let Some(cb) = progress_cb {
                cb(total_copied, total_size);
            }
        }
    }

    Ok(total_copied)
}

/// Create a progress bar for copying
pub fn create_copy_progress(total_bytes: u64) -> ProgressBar {
    let pb = ProgressBar::new(total_bytes);
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .expect("Valid template")
        .progress_chars("#>-");
    pb.set_style(style);
    pb
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::io::Write;

    #[test]
    fn test_copy_file() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let dest = temp.path().join("dest.txt");

        File::create(&source).unwrap().write_all(b"hello world").unwrap();
        copy_file(&source, &dest, None).unwrap();

        let content = fs::read_to_string(&dest).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_copy_dir() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.path().join("source");
        let dest_dir = temp.path().join("dest");

        fs::create_dir(&source_dir).unwrap();
        fs::create_dir(source_dir.join("sub")).unwrap();
        File::create(source_dir.join("a.txt")).unwrap().write_all(b"a").unwrap();
        File::create(source_dir.join("sub/b.txt")).unwrap().write_all(b"b").unwrap();

        let total = copy_dir_recursive(&source_dir, &dest_dir, None).unwrap();
        assert_eq!(total, 2); // a.txt (1) + b.txt (1)

        assert!(dest_dir.join("a.txt").exists());
        assert!(dest_dir.join("sub/b.txt").exists());
    }
}