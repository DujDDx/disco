//! Solid marker commands

use clap::Args;
use crate::Result;
use crate::cli::context::AppContext;
use crate::domain::disk::DiskId;

/// Set Solid marker on a directory
#[derive(Args, Debug)]
pub struct SolidCmd {
    /// Indexed path of the directory to mark
    #[arg(required = true)]
    path: String,

    /// Disk ID or name (required for ambiguous paths)
    #[arg(short, long)]
    disk: Option<String>,
}

pub fn handle_set(cmd: SolidCmd) -> Result<()> {
    let ctx = AppContext::init()?;
    handle_set_with_ctx(&ctx, cmd.path, cmd.disk)
}

pub fn handle_set_with_ctx(ctx: &AppContext, path: String, disk: Option<String>) -> Result<()> {
    let disk_repo = ctx.disk_repo();
    let entry_repo = ctx.entry_repo();

    // Find the disk
    let disk_id = if let Some(ref disk_id_or_name) = disk {
        // Try as disk ID first
        if let Ok(disk) = disk_repo.get_disk_by_id(&DiskId::new(disk_id_or_name.clone())) {
            disk.disk_id
        } else {
            // Try as disk name
            let disks = disk_repo.list_disks()?;
            disks.iter()
                .find(|d| &d.name == disk_id_or_name)
                .map(|d| d.disk_id.clone())
                .ok_or_else(|| crate::DiscoError::DiskNotFound(disk_id_or_name.clone()))?
        }
    } else {
        // Try to find entry by path across all disks
        let disks = disk_repo.list_disks()?;
        let mut found_disk_id = None;

        for disk in disks {
            let entries = entry_repo.get_entries_by_disk(&disk.disk_id)?;
            if entries.iter().any(|e| e.relative_path == path) {
                found_disk_id = Some(disk.disk_id);
                break;
            }
        }

        found_disk_id.ok_or_else(|| crate::DiscoError::PathNotFound(path.clone()))?
    };

    // Set solid flag
    entry_repo.set_solid_flag(&disk_id, &path)?;

    println!("✓ Solid marker set on: {}", path);
    println!("  This directory will not be split during storage operations.");

    Ok(())
}

pub fn handle_unset(cmd: SolidCmd) -> Result<()> {
    let ctx = AppContext::init()?;
    handle_unset_with_ctx(&ctx, cmd.path, cmd.disk)
}

pub fn handle_unset_with_ctx(ctx: &AppContext, path: String, disk: Option<String>) -> Result<()> {
    let disk_repo = ctx.disk_repo();
    let entry_repo = ctx.entry_repo();

    // Find the disk
    let disk_id = if let Some(ref disk_id_or_name) = disk {
        // Try as disk ID first
        if let Ok(disk) = disk_repo.get_disk_by_id(&DiskId::new(disk_id_or_name.clone())) {
            disk.disk_id
        } else {
            // Try as disk name
            let disks = disk_repo.list_disks()?;
            disks.iter()
                .find(|d| &d.name == disk_id_or_name)
                .map(|d| d.disk_id.clone())
                .ok_or_else(|| crate::DiscoError::DiskNotFound(disk_id_or_name.clone()))?
        }
    } else {
        // Try to find entry by path across all disks
        let disks = disk_repo.list_disks()?;
        let mut found_disk_id = None;

        for disk in disks {
            let entries = entry_repo.get_entries_by_disk(&disk.disk_id)?;
            if entries.iter().any(|e| e.relative_path == path) {
                found_disk_id = Some(disk.disk_id);
                break;
            }
        }

        found_disk_id.ok_or_else(|| crate::DiscoError::PathNotFound(path.clone()))?
    };

    // Unset solid flag
    entry_repo.unset_solid_flag(&disk_id, &path)?;

    println!("✓ Solid marker removed from: {}", path);

    Ok(())
}