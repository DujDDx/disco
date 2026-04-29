//! Interruptible search support with Ctrl+B
//!
//! This module provides search functionality that can be interrupted by Ctrl+B.

use crate::domain::entry::IndexEntry;
use crate::persistence::entry_repo::{EntryRepo, FolderMatch};
use crate::Result;
use colored::Colorize;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Default limit for search results when no limit is specified
pub const DEFAULT_SEARCH_LIMIT: usize = 10000;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Check if search was interrupted
pub fn is_interrupted() -> bool {
    INTERRUPTED.load(Ordering::Relaxed)
}

/// Reset interrupt flag
fn reset_interrupt() {
    INTERRUPTED.store(false, Ordering::Relaxed);
}

/// Set interrupt flag
fn set_interrupt() {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

/// Result of an interruptible search
#[derive(Debug)]
pub struct InterruptibleSearchResult {
    /// Entries found
    pub entries: Vec<IndexEntry>,
    /// Folder matches found
    pub folder_matches: Vec<FolderMatch>,
    /// Whether the search was interrupted
    pub was_interrupted: bool,
}

/// Run a search with Ctrl+B interrupt support
///
/// This function:
/// 1. Shows a progress spinner while searching
/// 2. Listens for Ctrl+B in a background thread
/// 3. Returns results (either complete or interrupted)
pub fn run_interruptible_search(
    entry_repo: &EntryRepo,
    keyword: &str,
    entry_limit: usize,
    folder_limit: usize,
) -> Result<InterruptibleSearchResult> {
    run_interruptible_search_with_limits(entry_repo, keyword, entry_limit, folder_limit)
}

/// Run a search with default limits (no artificial restrictions)
pub fn run_interruptible_search_unlimited(
    entry_repo: &EntryRepo,
    keyword: &str,
) -> Result<InterruptibleSearchResult> {
    run_interruptible_search_with_limits(entry_repo, keyword, DEFAULT_SEARCH_LIMIT, DEFAULT_SEARCH_LIMIT)
}

/// Internal implementation with configurable limits
fn run_interruptible_search_with_limits(
    entry_repo: &EntryRepo,
    keyword: &str,
    entry_limit: usize,
    folder_limit: usize,
) -> Result<InterruptibleSearchResult> {
    reset_interrupt();

    // Create progress bar (no steady tick to avoid blocking)
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Valid template"),
    );
    pb.set_message(format!("Searching for '{}'... (Ctrl+B to interrupt)", keyword));

    // Start key listener thread
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    
    let listener_handle = thread::spawn(move || {
        if enable_raw_mode().is_ok() {
            while running_clone.load(Ordering::Relaxed) {
                if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                    if let Event::Key(key) = event::read().unwrap_or(Event::Key(event::KeyEvent {
                        code: KeyCode::Char(' '),
                        modifiers: KeyModifiers::empty(),
                        kind: event::KeyEventKind::Press,
                        state: event::KeyEventState::empty(),
                    })) {
                        if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL) {
                            set_interrupt();
                            break;
                        }
                    }
                }
            }
            let _ = disable_raw_mode();
        }
    });

    // Perform first search (entries)
    pb.set_message(format!("Searching entries for '{}'...", keyword));
    pb.tick();
    
    let entries = if !is_interrupted() {
        match entry_repo.search_by_name(keyword, entry_limit) {
            Ok(e) => e,
            Err(e) => {
                running.store(false, Ordering::Relaxed);
                let _ = listener_handle.join();
                return Err(e);
            }
        }
    } else {
        Vec::new()
    };

    // Update progress
    pb.set_message(format!("Found {} entries, searching folders...", entries.len()));
    pb.tick();

    // Perform second search (folders) if not interrupted
    let folder_matches = if !is_interrupted() {
        match entry_repo.search_folder_names(keyword, folder_limit) {
            Ok(f) => f,
            Err(e) => {
                running.store(false, Ordering::Relaxed);
                let _ = listener_handle.join();
                return Err(e);
            }
        }
    } else {
        Vec::new()
    };

    // Stop listener thread
    running.store(false, Ordering::Relaxed);
    let _ = listener_handle.join();

    // Clean up progress bar
    pb.finish_and_clear();

    let was_interrupted = is_interrupted();

    Ok(InterruptibleSearchResult {
        entries,
        folder_matches,
        was_interrupted,
    })
}

/// Run a search with progress display (alias for run_interruptible_search)
pub fn run_interruptible_search_batched(
    entry_repo: &EntryRepo,
    keyword: &str,
    entry_limit: usize,
    folder_limit: usize,
) -> Result<InterruptibleSearchResult> {
    run_interruptible_search(entry_repo, keyword, entry_limit, folder_limit)
}

/// Display search results and let user select
pub fn display_results_and_select(
    entries: &[IndexEntry],
    folder_matches: &[FolderMatch],
    was_interrupted: bool,
) -> Result<Vec<usize>> {
    println!();

    if was_interrupted {
        println!(
            "  {} Search interrupted - showing {} results found so far",
            "⚠".yellow(),
            entries.len() + folder_matches.len()
        );
    }

    if entries.is_empty() && folder_matches.is_empty() {
        println!("  No results found.");
        return Ok(Vec::new());
    }

    println!();
    println!("{}", "  Search Results:".cyan().bold());
    println!();

    let mut result_index = 0usize;

    // Show folder matches
    if !folder_matches.is_empty() {
        println!("{}", "  Folders:".yellow().bold());
        println!();

        for folder in folder_matches {
            let split_indicator = if folder.is_split() {
                format!(" {} ", "[Split]".magenta())
            } else {
                String::new()
            };

            println!(
                "  {} {} {}{}[{} files, {}]",
                format!("[{}]", result_index + 1).bright_black(),
                "📁",
                folder.folder_name.white().bold(),
                split_indicator,
                folder.file_count.to_string().cyan(),
                format_size(folder.total_size).green()
            );

            let disk_names: Vec<&str> = folder.disk_names.split(',').collect();
            if disk_names.len() > 1 {
                println!(
                    "      {} {}",
                    "→".bright_black(),
                    folder.disk_names.yellow()
                );
            } else {
                println!(
                    "      {} {}",
                    "→".bright_black(),
                    folder.disk_names.bright_black()
                );
            }

            result_index += 1;
        }
        println!();
    }

    // Show files
    if !entries.is_empty() {
        println!("{}", "  Files:".cyan().bold());
        println!();

        let display_count = entries.len().min(20);
        for entry in entries.iter().take(display_count) {
            println!(
                "  {} {} {} [{}] {}",
                format!("[{}]", result_index + 1).bright_black(),
                "📄",
                entry.file_name.white(),
                format_size(entry.size).cyan(),
                entry.disk_name.bright_black()
            );
            result_index += 1;
        }

        if entries.len() > 20 {
            println!("  ... and {} more files", entries.len() - 20);
        }
    }

    println!();
    println!(
        "  Total: {} folders, {} files",
        folder_matches.len(),
        entries.len()
    );
    println!();

    // Ask for selection
    print!("Enter numbers to select (e.g., 1,3,5) or 'all': ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim();

    let total_items = folder_matches.len() + entries.len().min(20);
    let indices: Vec<usize> = if input.to_lowercase() == "all" {
        (0..total_items).collect()
    } else {
        input
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter(|&i| i > 0 && i <= total_items)
            .map(|i| i - 1)
            .collect()
    };

    Ok(indices)
}

/// Format file size in human-readable format
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }
}