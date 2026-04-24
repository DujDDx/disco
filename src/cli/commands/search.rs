//! Search command

use clap::Args;
use crate::Result;
use crate::cli::context::AppContext;
use crate::cli::display::format_size;
use crate::cli::interruptible::run_interruptible_search;
use crate::index::query::SearchOptions;
use crate::storage::platform::DiskDetector;
use colored::Colorize;

/// Search for files in the index
#[derive(Args, Debug)]
pub struct SearchCmd {
    /// Search keyword (matches file name)
    #[arg(required = true)]
    keyword: String,

    /// Filter by minimum file size (in bytes)
    #[arg(long)]
    min_size: Option<u64>,

    /// Filter by maximum file size (in bytes)
    #[arg(long)]
    max_size: Option<u64>,

    /// Filter by file extension (e.g., ".pdf")
    #[arg(long)]
    ext: Option<String>,

    /// Limit number of results
    #[arg(short, long, default_value = "50")]
    limit: usize,
}

pub fn handle_search(cmd: SearchCmd) -> Result<()> {
    let ctx = AppContext::init()?;
    handle_search_with_ctx(&ctx, cmd.keyword, cmd.min_size, cmd.max_size, cmd.ext, cmd.limit)
}

pub fn handle_search_with_ctx(ctx: &AppContext, keyword: String, min_size: Option<u64>, max_size: Option<u64>, ext: Option<String>, limit: usize) -> Result<()> {
    let detector = AppContext::disk_detector();
    let entry_repo = ctx.entry_repo();

    // Perform search with interruptible search
    println!();
    println!("Searching for: {}", keyword.cyan());

    let search_result = run_interruptible_search(&entry_repo, &keyword, limit * 2, 50)?;
    let entries = search_result.entries;
    let folder_matches = search_result.folder_matches;
    let was_interrupted = search_result.was_interrupted;

    // Build search options for additional filtering
    let options = SearchOptions {
        min_size,
        max_size,
        ext: ext.clone(),
        entry_type: None,
        limit,
    };

    // Apply additional filters and calculate scores
    let mut results: Vec<crate::index::query::SearchResult> = entries
        .into_iter()
        .filter(|e| {
            // Size filter
            if let Some(min) = options.min_size {
                if e.size < min {
                    return false;
                }
            }
            if let Some(max) = options.max_size {
                if e.size > max {
                    return false;
                }
            }

            // Extension filter
            if let Some(ref ext_filter) = options.ext {
                if e.extension() != Some(ext_filter.trim_start_matches('.')) {
                    return false;
                }
            }

            true
        })
        .filter_map(|entry| {
            let score = calculate_search_score(&entry.file_name, &entry.relative_path, &keyword);
            if score > 0 {
                Some(crate::index::query::SearchResult { entry, score })
            } else {
                None
            }
        })
        .collect();

    // Sort by score
    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(limit);

    // Show interruption message if applicable
    if was_interrupted {
        println!();
        println!(
            "  [!] Search interrupted - showing results found so far"
        );
    }

    if results.is_empty() && folder_matches.is_empty() {
        println!();
        if was_interrupted {
            println!("  No results found before interruption.");
        } else {
            println!("No files found matching '{}'", keyword);
        }
        return Ok(());
    }

    // Get mount points for status check
    let mount_points = detector.list_mount_points()?;
    let disk_repo = ctx.disk_repo();
    let disks = disk_repo.list_disks()?;

    // Build mount status map
    let mut mounted_disks: std::collections::HashSet<String> = std::collections::HashSet::new();
    for mount in &mount_points {
        if let Ok(identity) = detector.detect_identity(mount) {
            for disk in &disks {
                if disk.identity.matches(&identity) {
                    mounted_disks.insert(disk.disk_id.as_str().to_string());
                }
            }
        }
    }

    println!();
    println!("{}", "  Search Results:".cyan().bold());
    println!();

    let mut result_index = 0usize;

    // Show folder matches first
    if !folder_matches.is_empty() {
        println!("{}", "  Folders (aggregated across disks):".yellow().bold());
        println!();

        for folder in &folder_matches {
            let split_indicator = if folder.is_split() {
                " [SPLIT]"
            } else {
                ""
            };

            // Simple format: [N] [DIR] foldername (X files, size) - disk1,disk2
            println!(
                "  [{}] [DIR] {}{} ({} files, {}) - {}",
                result_index + 1,
                folder.folder_name,
                split_indicator,
                folder.file_count,
                format_size(folder.total_size),
                folder.disk_names
            );

            result_index += 1;
        }
        println!();
    }

    // Show files
    if !results.is_empty() {
        println!("{}", "  Files:".cyan().bold());
        println!();

        for result in &results {
            let entry = &result.entry;
            let is_mounted = mounted_disks.contains(entry.disk_id.as_str());
            let status = if is_mounted { "[ONLINE]" } else { "[OFFLINE]" };

            // Simple format: [N] [FILE] filename (size) - disk - path
            println!(
                "  [{}] {} {} ({}) - {}",
                result_index + 1,
                status,
                entry.file_name,
                format_size(entry.size),
                entry.disk_name
            );

            result_index += 1;
        }
    }

    println!();
    println!(
        "  Total: {} folders, {} files",
        folder_matches.len(),
        results.len()
    );
    println!();
    println!("Use 'disco get <ID>' to locate a specific file.");

    Ok(())
}

/// Calculate a simple match score based on how well the keyword matches
fn calculate_search_score(file_name: &str, relative_path: &str, keyword: &str) -> u32 {
    let file_lower = file_name.to_lowercase();
    let path_lower = relative_path.to_lowercase();
    let keyword_lower = keyword.to_lowercase();

    // Exact match on file_name gets highest score
    if file_lower == keyword_lower {
        return 1000;
    }

    // Starts with keyword on file_name gets high score
    if file_lower.starts_with(&keyword_lower) {
        return 800;
    }

    // Contains keyword in file_name gets medium score
    if file_lower.contains(&keyword_lower) {
        let pos = file_lower.find(&keyword_lower).unwrap_or(0);
        return 500 + (100 - pos.min(100) as u32);
    }

    // Match in relative_path (folder name match) gets lower score
    if path_lower.contains(&keyword_lower) {
        let segments: Vec<&str> = path_lower.split('/').collect();
        for (i, segment) in segments.iter().enumerate() {
            if segment.contains(&keyword_lower) {
                if *segment == keyword_lower {
                    return 600;
                }
                if segment.starts_with(&keyword_lower) {
                    return 450 + (100 - i.min(100) as u32);
                }
                let pos = segment.find(&keyword_lower).unwrap_or(0);
                return 300 + (50 - pos.min(50) as u32) + (20 - i.min(20) as u32);
            }
        }
    }

    0
}
