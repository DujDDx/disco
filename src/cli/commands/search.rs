//! Search command

use clap::Args;
use crate::Result;
use crate::cli::context::AppContext;
use crate::cli::display::{format_size, format_mount_status};
use crate::index::query::{search, SearchOptions};
use crate::storage::platform::DiskDetector;

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

    // Build search options
    let options = SearchOptions {
        min_size,
        max_size,
        ext: ext.clone(),
        entry_type: None,
        limit,
    };

    // Perform search
    let results = search(&entry_repo, &keyword, options)?;

    if results.is_empty() {
        println!("No files found matching '{}'", keyword);
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

    println!("\nSearch results for '{}' ({} found):\n", keyword, results.len());
    println!("{:<8} {:<30} {:<15} {:<12} {:<10} {}",
        "ID", "Name", "Disk", "Size", "Status", "Path");
    println!("{}", "-".repeat(100));

    for result in results {
        let entry = &result.entry;
        let is_mounted = mounted_disks.contains(entry.disk_id.as_str());
        let status = format_mount_status(is_mounted);

        // Truncate name if too long (using character count, not bytes)
        let name = if entry.file_name.chars().count() > 28 {
            format!("{}...", entry.file_name.chars().take(25).collect::<String>())
        } else {
            entry.file_name.clone()
        };

        // Truncate path if too long (using character count, not bytes)
        let path = if entry.relative_path.chars().count() > 40 {
            let chars: Vec<char> = entry.relative_path.chars().collect();
            format!("...{}", chars[chars.len().saturating_sub(37)..].iter().collect::<String>())
        } else {
            entry.relative_path.clone()
        };

        println!("{:<8} {:<30} {:<15} {:<12} {:<10} {}",
            entry.entry_id,
            name,
            entry.disk_name,
            format_size(entry.size),
            status,
            path
        );
    }

    println!();
    println!("Use 'disco get <ID>' to locate a specific file.");

    Ok(())
}