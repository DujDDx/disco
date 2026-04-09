//! Scan commands

use clap::Args;
use crate::Result;
use crate::cli::context::AppContext;
use crate::index::scanner::full_scan;
use crate::storage::platform::DiskDetector;

/// Scan target specification
#[derive(Debug, Clone)]
pub enum ScanTarget {
    All,
    Disk(String),
}

/// Scan disks to build/update the file index
#[derive(Args, Debug)]
pub struct ScanCmd {
    /// Scan all registered disks
    #[arg(short, long, conflicts_with = "disk")]
    all: bool,

    /// Scan a specific disk by ID or name
    #[arg(short, long, conflicts_with = "all")]
    disk: Option<String>,

    /// Enable hash calculation during scan
    #[arg(short, long)]
    hash: bool,

    /// Force full scan instead of incremental
    #[arg(short, long)]
    full: bool,
}

pub fn handle_scan(cmd: ScanCmd) -> Result<()> {
    let ctx = AppContext::init()?;
    handle_scan_with_ctx(&ctx, cmd.all, cmd.disk, cmd.hash, cmd.full)
}

pub fn handle_scan_with_ctx(ctx: &AppContext, all: bool, disk: Option<String>, hash: bool, full: bool) -> Result<()> {
    let detector = AppContext::disk_detector();
    let disk_repo = ctx.disk_repo();
    let entry_repo = ctx.entry_repo();

    // Determine scan target
    let target = if all || disk.is_none() {
        ScanTarget::All
    } else {
        ScanTarget::Disk(disk.clone().unwrap())
    };

    // Get disks to scan
    let disks_to_scan = match target {
        ScanTarget::All => {
            disk_repo.list_disks()?
        }
        ScanTarget::Disk(ref id_or_name) => {
            // Try to find disk by ID or name
            let all_disks = disk_repo.list_disks()?;
            let matching: Vec<_> = all_disks
                .into_iter()
                .filter(|d| d.disk_id.as_str() == id_or_name || d.name == *id_or_name)
                .collect();

            if matching.is_empty() {
                return Err(crate::DiscoError::DiskNotFound(id_or_name.clone()));
            }
            matching
        }
    };

    if disks_to_scan.is_empty() {
        println!("No disks to scan. Register disks first with 'disco disk add'.");
        return Ok(());
    }

    // Get current mount points
    let mount_points = detector.list_mount_points()?;

    let mut total_files = 0usize;
    let mut total_dirs = 0usize;

    for disk in &disks_to_scan {
        println!("\nScanning disk: {} [{}]", disk.name, disk.disk_id);

        // Find current mount point
        let mut current_mount: Option<String> = None;
        for mount in &mount_points {
            if let Ok(identity) = detector.detect_identity(&mount) {
                if disk.identity.matches(&identity) {
                    current_mount = Some(mount.clone());
                    break;
                }
            }
        }

        let mount_point = match current_mount {
            Some(mp) => mp,
            None => {
                println!("  ⚠ Disk not mounted, skipping...");
                continue;
            }
        };

        println!("  Mount point: {}", mount_point);

        if hash {
            println!("  Hash calculation: enabled");
        }

        // Run scan
        let mount_path = std::path::Path::new(&mount_point);
        let report = full_scan(
            disk,
            mount_path,
            &entry_repo,
            &disk_repo,
            hash,
        )?;

        println!("\n  Scan completed:");
        println!("    Files added: {}", report.files_added);
        println!("    Files updated: {}", report.files_updated);
        println!("    Dirs added: {}", report.dirs_added);
        println!("    Dirs updated: {}", report.dirs_updated);
        if report.files_marked_missing > 0 {
            println!("    Files marked missing: {}", report.files_marked_missing);
        }
        if !report.errors.is_empty() {
            println!("    Errors: {}", report.errors.len());
        }

        total_files += report.files_added + report.files_updated;
        total_dirs += report.dirs_added + report.dirs_updated;
    }

    println!("\n✓ Scan complete!");
    println!("  Total files indexed: {}", total_files);
    println!("  Total directories indexed: {}", total_dirs);

    Ok(())
}