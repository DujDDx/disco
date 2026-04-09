//! Get command - Locate a file by entry ID

use clap::Args;
use crate::Result;
use crate::cli::context::AppContext;
use crate::cli::display::format_size;
use crate::storage::platform::DiskDetector;

/// Get file location by entry ID
#[derive(Args, Debug)]
pub struct GetCmd {
    /// Entry ID from search results
    #[arg(required = true)]
    entry_id: i64,

    /// Check if disk is mounted and provide access path
    #[arg(short, long)]
    locate: bool,
}

pub fn handle_get(cmd: GetCmd) -> Result<()> {
    let ctx = AppContext::init()?;
    handle_get_with_ctx(&ctx, cmd.entry_id, cmd.locate)
}

pub fn handle_get_with_ctx(ctx: &AppContext, entry_id: i64, locate: bool) -> Result<()> {
    let detector = AppContext::disk_detector();
    let entry_repo = ctx.entry_repo();
    let disk_repo = ctx.disk_repo();

    // Get the entry
    let entry = entry_repo.get_entry_by_id(entry_id)?;

    println!("\nFile Information:");
    println!("  Name: {}", entry.file_name);
    println!("  Size: {}", format_size(entry.size));
    println!("  Disk: {} [{}]", entry.disk_name, entry.disk_id);
    println!("  Path: {}", entry.relative_path);

    if let Some(ref hash) = entry.hash {
        println!("  Hash: {}...", &hash[..16]);
    }

    // Get disk info
    let disk = disk_repo.get_disk_by_id(&entry.disk_id)?;

    // Check if disk is mounted
    let mount_points = detector.list_mount_points()?;
    let mut current_mount: Option<String> = None;

    for mount in &mount_points {
        if let Ok(identity) = detector.detect_identity(&mount) {
            if disk.identity.matches(&identity) {
                current_mount = Some(mount.clone());
                break;
            }
        }
    }

    match current_mount {
        Some(mount) => {
            println!("\n✓ Disk is mounted at: {}", mount);
            let full_path = format!("{}/{}", mount.trim_end_matches('/'), entry.relative_path);
            println!("Full path: {}", full_path);

            // Verify file exists
            let path = std::path::Path::new(&full_path);
            if path.exists() {
                println!("✓ File verified");
            } else {
                println!("⚠ Warning: File not found at expected location");
            }
        }
        None => {
            println!("\n⚠ Disk '{}' is not currently mounted.", disk.name);
            if let Some(ref last_mount) = disk.last_mount_point {
                println!("  Last known mount point: {}", last_mount);
            }
            println!("\nPlease connect the disk to access this file.");
        }
    }

    Ok(())
}