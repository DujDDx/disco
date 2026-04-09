//! Disk management commands

use clap::Args;
use crate::Result;
use crate::cli::context::AppContext;
use crate::cli::display::{format_size, format_mount_status};
use crate::domain::disk::{Disk, DiskId, MountStatus};
use crate::storage::platform::DiskDetector;
use std::io::{self, Write};

/// Add a disk to the pool
#[derive(Args, Debug)]
pub struct DiskAddCmd {
    /// Mount point of the disk to add (e.g., /Volumes/MyDisk on macOS)
    #[arg(required = true)]
    mount_point: String,

    /// Custom name for the disk (will prompt if not provided)
    #[arg(short, long)]
    name: Option<String>,
}

/// List all registered disks
#[derive(Args, Debug)]
pub struct DiskListCmd {
    /// Show detailed information including identity details
    #[arg(short, long)]
    detailed: bool,
}

pub fn handle_add(cmd: DiskAddCmd) -> Result<()> {
    let ctx = AppContext::init()?;
    handle_add_with_ctx(&ctx, cmd.mount_point, cmd.name)
}

pub fn handle_add_with_ctx(ctx: &AppContext, mount_point: String, name: Option<String>) -> Result<()> {
    let detector = AppContext::disk_detector();
    let disk_repo = ctx.disk_repo();

    // Verify mount point exists
    let mount_path = std::path::Path::new(&mount_point);
    if !mount_path.exists() {
        return Err(crate::DiscoError::PathNotFound(mount_point.clone()));
    }

    // Detect disk identity
    println!("Detecting disk at {}...", mount_point);
    let identity = detector.detect_identity(&mount_point)?;

    // Check if disk is already registered
    if let Some(existing) = disk_repo.find_disk_by_identity(&identity)? {
        println!("Disk already registered as: {}", existing.name);
        println!("Disk ID: {}", existing.disk_id);
        return Ok(());
    }

    // Get disk name
    let disk_name = match name {
        Some(n) => n,
        None => {
            // Prompt for name
            let default_name = identity.volume_label.clone()
                .unwrap_or_else(|| "New Disk".to_string());

            print!("Enter disk name [{}]: ", default_name);
            io::stdout().flush().ok();

            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            let trimmed = input.trim();

            if trimmed.is_empty() {
                default_name
            } else {
                trimmed.to_string()
            }
        }
    };

    // Generate disk ID from identity
    let disk_id = DiskId::new(
        identity.serial.clone()
            .or(identity.volume_uuid.clone())
            .unwrap_or_else(|| identity.fingerprint.clone())
    );

    // Create disk entry
    let disk = Disk {
        disk_id: disk_id.clone(),
        name: disk_name.clone(),
        identity,
        first_registered: chrono::Utc::now(),
        last_mount_point: Some(mount_point.clone()),
        mount_status: MountStatus::Connected,
        current_mount_point: Some(mount_point.clone()),
    };

    // Insert into database
    disk_repo.insert_disk(&disk)?;

    println!("\n✓ Disk registered successfully!");
    println!("  Name: {}", disk_name);
    println!("  ID: {}", disk_id);
    println!("  Capacity: {}", format_size(disk.identity.capacity_bytes));

    Ok(())
}

pub fn handle_list(cmd: DiskListCmd) -> Result<()> {
    let ctx = AppContext::init()?;
    handle_list_with_ctx(&ctx, cmd.detailed)
}

pub fn handle_list_with_ctx(ctx: &AppContext, detailed: bool) -> Result<()> {
    let detector = AppContext::disk_detector();
    let disk_repo = ctx.disk_repo();

    // Get all registered disks
    let disks = disk_repo.list_disks()?;

    if disks.is_empty() {
        println!("No disks registered. Use 'disco disk add <mount-point>' to add a disk.");
        return Ok(());
    }

    // Refresh mount status
    let mount_points = detector.list_mount_points()?;

    println!("\nRegistered Disks ({}):\n", disks.len());

    for disk in disks {
        // Check if disk is currently mounted
        let mut is_mounted = false;
        let mut current_mount: Option<String> = None;

        for mount in &mount_points {
            if let Ok(identity) = detector.detect_identity(mount) {
                if disk.identity.matches(&identity) {
                    is_mounted = true;
                    current_mount = Some(mount.clone());
                    break;
                }
            }
        }

        let status_str = format_mount_status(is_mounted);

        println!("  {} [{}]", disk.name, disk.disk_id);
        println!("    Status: {}", status_str);
        println!("    Capacity: {}", format_size(disk.identity.capacity_bytes));

        if let Some(mount) = &current_mount {
            println!("    Mount point: {}", mount);
        } else if let Some(last) = &disk.last_mount_point {
            println!("    Last mount: {}", last);
        }

        if detailed {
            if let Some(ref serial) = disk.identity.serial {
                println!("    Serial: {}", serial);
            }
            if let Some(ref uuid) = disk.identity.volume_uuid {
                println!("    Volume UUID: {}", uuid);
            }
            if let Some(ref label) = disk.identity.volume_label {
                println!("    Volume Label: {}", label);
            }
            println!("    Registered: {}", disk.first_registered.format("%Y-%m-%d %H:%M"));
        }

        println!();
    }

    Ok(())
}