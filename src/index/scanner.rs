//! File scanning and indexing

use crate::domain::disk::Disk;
use crate::domain::entry::{IndexEntry, EntryType, EntryStatus};
use crate::persistence::entry_repo::EntryRepo;
use crate::persistence::disk_repo::DiskRepo;
use crate::Result;
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use walkdir::WalkDir;

/// Scan progress callback
pub struct ScanProgress {
    bar: ProgressBar,
    files_scanned: u64,
    dirs_scanned: u64,
    total_size: u64,
    has_total: bool,
}

impl ScanProgress {
    /// Create progress bar with unknown total (shows spinner)
    pub fn new_spinner() -> Self {
        let bar = ProgressBar::new_spinner();
        let style = ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Valid template");
        bar.set_style(style);
        bar.set_message("Scanning...");
        Self {
            bar,
            files_scanned: 0,
            dirs_scanned: 0,
            total_size: 0,
            has_total: false,
        }
    }

    /// Create progress bar with known total
    pub fn new(total_estimate: u64) -> Self {
        if total_estimate == 0 {
            return Self::new_spinner();
        }
        let bar = ProgressBar::new(total_estimate);
        let style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
            .expect("Valid template")
            .progress_chars("#>-");
        bar.set_style(style);
        Self {
            bar,
            files_scanned: 0,
            dirs_scanned: 0,
            total_size: 0,
            has_total: true,
        }
    }

    pub fn inc_file(&mut self, size: u64) {
        self.files_scanned += 1;
        self.total_size += size;
        if self.has_total {
            self.bar.inc(1);
        } else {
            self.bar.set_message(format!(
                "Scanned {} files, {} dirs, {}",
                self.files_scanned,
                self.dirs_scanned,
                format_size(self.total_size)
            ));
            self.bar.tick();
        }
    }

    pub fn inc_dir(&mut self) {
        self.dirs_scanned += 1;
        if !self.has_total {
            self.bar.set_message(format!(
                "Scanned {} files, {} dirs, {}",
                self.files_scanned,
                self.dirs_scanned,
                format_size(self.total_size)
            ));
            self.bar.tick();
        }
    }

    pub fn finish(&self) {
        if self.has_total {
            self.bar.finish_with_message(format!(
                "Done: {} files, {} dirs, {}",
                self.files_scanned,
                self.dirs_scanned,
                format_size(self.total_size)
            ));
        } else {
            self.bar.finish_with_message(format!(
                "Done: {} files, {} dirs, {}",
                self.files_scanned,
                self.dirs_scanned,
                format_size(self.total_size)
            ));
        }
    }

    pub fn report(&self) -> (u64, u64, u64) {
        (self.files_scanned, self.dirs_scanned, self.total_size)
    }
}

/// Format size for display
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// Scan result statistics
pub struct ScanReport {
    pub files_added: usize,
    pub files_updated: usize,
    pub files_marked_missing: usize,
    pub dirs_added: usize,
    pub dirs_updated: usize,
    pub errors: Vec<String>,
}

impl ScanReport {
    pub fn new() -> Self {
        Self {
            files_added: 0,
            files_updated: 0,
            files_marked_missing: 0,
            dirs_added: 0,
            dirs_updated: 0,
            errors: Vec::new(),
        }
    }

    pub fn total_entries(&self) -> usize {
        self.files_added + self.files_updated + self.dirs_added + self.dirs_updated
    }
}

/// Full scan of a disk
pub fn full_scan(
    disk: &Disk,
    mount_point: &Path,
    entry_repo: &EntryRepo,
    disk_repo: &DiskRepo,
    compute_hash: bool,
) -> Result<ScanReport> {
    let mut report = ScanReport::new();
    let mut progress = ScanProgress::new_spinner();

    let existing_entries = entry_repo.get_entries_by_disk(&disk.disk_id)?;
    let existing_paths: std::collections::HashSet<String> = existing_entries
        .iter()
        .map(|e| e.relative_path.clone())
        .collect();

    let mut seen_paths = std::collections::HashSet::new();

    for entry in WalkDir::new(mount_point).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let relative = path.strip_prefix(mount_point)
            .expect("Path should be under mount point")
            .to_string_lossy()
            .to_string();

        if relative.is_empty() {
            continue;
        }

        seen_paths.insert(relative.clone());

        let metadata = entry.metadata()?;
        let mtime = chrono::DateTime::from(metadata.modified()?);

        let (entry_type, size) = if metadata.is_dir() {
            progress.inc_dir();
            (EntryType::Dir, 0)
        } else {
            let size = metadata.len();
            progress.inc_file(size);
            (EntryType::File, size)
        };

        let hash = if compute_hash && metadata.is_file() {
            Some(crate::index::hasher::hash_file(path)?)
        } else {
            None
        };

        let index_entry = IndexEntry {
            entry_id: 0,
            disk_id: disk.disk_id.clone(),
            disk_name: disk.name.clone(),
            relative_path: relative.clone(),
            file_name: entry.file_name().to_string_lossy().to_string(),
            size,
            hash,
            mtime,
            entry_type,
            solid_flag: false,
            last_seen_mount_point: mount_point.to_string_lossy().to_string(),
            indexed_at: Utc::now(),
            status: EntryStatus::Normal,
        };

        if existing_paths.contains(&relative) {
            entry_repo.upsert_entry(&index_entry)?;
            if entry_type == EntryType::File {
                report.files_updated += 1;
            } else {
                report.dirs_updated += 1;
            }
        } else {
            entry_repo.upsert_entry(&index_entry)?;
            if entry_type == EntryType::File {
                report.files_added += 1;
            } else {
                report.dirs_added += 1;
            }
        }
    }

    for existing in &existing_entries {
        if !seen_paths.contains(&existing.relative_path) {
            entry_repo.mark_missing(&disk.disk_id, &existing.relative_path)?;
            report.files_marked_missing += 1;
        }
    }

    disk_repo.update_last_mount_point(&disk.disk_id, mount_point.to_string_lossy().to_string())?;

    progress.finish();

    Ok(report)
}

/// Scan a specific path under a disk (for indexing newly stored files)
pub fn scan_path(
    disk: &Disk,
    mount_point: &Path,
    target_path: &Path,
    entry_repo: &EntryRepo,
    compute_hash: bool,
) -> Result<ScanReport> {
    let mut report = ScanReport::new();

    // Get existing entries for this disk
    let existing_entries = entry_repo.get_entries_by_disk(&disk.disk_id)?;
    let existing_paths: std::collections::HashSet<String> = existing_entries
        .iter()
        .map(|e| e.relative_path.clone())
        .collect();

    let mut progress = ScanProgress::new_spinner();
    progress.bar.set_message(format!("Indexing {}...", target_path.display()));

    // Scan the target path
    for entry in WalkDir::new(target_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let relative = path.strip_prefix(mount_point)
            .expect("Path should be under mount point")
            .to_string_lossy()
            .to_string();

        if relative.is_empty() {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime = chrono::DateTime::from(metadata.modified()?);

        let (entry_type, size) = if metadata.is_dir() {
            progress.inc_dir();
            (EntryType::Dir, 0)
        } else {
            let size = metadata.len();
            progress.inc_file(size);
            (EntryType::File, size)
        };

        let hash = if compute_hash && metadata.is_file() {
            match crate::index::hasher::hash_file(path) {
                Ok(h) => Some(h),
                Err(_) => None,
            }
        } else {
            None
        };

        let index_entry = IndexEntry {
            entry_id: 0,
            disk_id: disk.disk_id.clone(),
            disk_name: disk.name.clone(),
            relative_path: relative.clone(),
            file_name: entry.file_name().to_string_lossy().to_string(),
            size,
            hash,
            mtime,
            entry_type,
            solid_flag: false,
            last_seen_mount_point: mount_point.to_string_lossy().to_string(),
            indexed_at: Utc::now(),
            status: EntryStatus::Normal,
        };

        if existing_paths.contains(&relative) {
            entry_repo.upsert_entry(&index_entry)?;
            if entry_type == EntryType::File {
                report.files_updated += 1;
            } else {
                report.dirs_updated += 1;
            }
        } else {
            entry_repo.upsert_entry(&index_entry)?;
            if entry_type == EntryType::File {
                report.files_added += 1;
            } else {
                report.dirs_added += 1;
            }
        }
    }

    progress.finish();

    Ok(report)
}

#[cfg(test)]
mod tests {
    // Integration tests require database setup
}
