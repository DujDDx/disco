//! Store command - Store files into the disk pool

use clap::Args;
use crate::Result;
use crate::cli::context::AppContext;
use crate::cli::display::{format_size, print_success, print_warning, print_header, print_separator};
use crate::domain::solid::SolidLayerDepth;
use crate::planner::splitter::split_into_atomic_units;
use crate::planner::strategy::BestFitStrategy;
use crate::planner::strategy::DiskSelectionStrategy;
use crate::storage::platform::DiskDetector;
use crate::domain::disk::MountStatus;
use crate::index::scanner::scan_path;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};

/// Store files/folders into the disk pool
#[derive(Args, Debug)]
pub struct StoreCmd {
    /// Paths to store (supports drag-and-drop paths)
    #[arg(required = true)]
    paths: Vec<String>,

    /// SolidLayer depth (0=no split, 1=split to first level, inf=split to files)
    #[arg(long, default_value = "0")]
    solid_layer: String,

    /// Enable hash-based deduplication
    #[arg(short, long)]
    dedup: bool,

    /// Preview the storage plan without executing
    #[arg(short, long)]
    preview: bool,

    /// Skip confirmation prompt
    #[arg(short, long)]
    yes: bool,
}

pub fn handle_store(cmd: StoreCmd) -> Result<()> {
    let ctx = AppContext::init()?;
    handle_store_with_ctx(&ctx, cmd.paths, cmd.solid_layer, cmd.dedup, cmd.preview, cmd.yes)
}

pub fn handle_store_with_ctx(ctx: &AppContext, paths: Vec<String>, solid_layer: String, dedup: bool, preview: bool, yes: bool) -> Result<()> {
    let detector = AppContext::disk_detector();
    let disk_repo = ctx.disk_repo();
    let fs_adapter = AppContext::fs_adapter();

    // Parse SolidLayer
    let solid_layer_depth = SolidLayerDepth::parse(&solid_layer)
        .map_err(|e| crate::DiscoError::InvalidPath(e))?;

    println!("SolidLayer: {}", solid_layer_depth);

    // Validate input paths
    let mut valid_paths = Vec::new();
    for path_str in &paths {
        // Handle escaped paths from drag-and-drop
        let path_str = path_str.trim_matches('"').trim_matches('\'');

        let path = std::path::Path::new(path_str);
        if path.exists() {
            valid_paths.push(path.to_path_buf());
        } else {
            println!("⚠ Path not found, skipping: {}", path_str);
        }
    }

    if valid_paths.is_empty() {
        println!("No valid paths to store.");
        return Ok(());
    }

    println!("\nInput paths:");
    for path in &valid_paths {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let size = if path.is_dir() {
            fs_adapter.dir_total_size(path)?
        } else {
            fs_adapter.file_size(path)?
        };
        println!("  - {} ({})", name, format_size(size));
    }

    // Get mounted disks
    let all_disks = disk_repo.list_disks()?;
    let mount_points = detector.list_mount_points()?;

    let mut mounted_disks: Vec<crate::domain::disk::Disk> = Vec::new();
    for disk in all_disks {
        for mount in &mount_points {
            if let Ok(identity) = detector.detect_identity(mount) {
                if disk.identity.matches(&identity) {
                    let mut d = disk.clone();
                    d.mount_status = MountStatus::Connected;
                    d.current_mount_point = Some(mount.clone());
                    mounted_disks.push(d);
                    break;
                }
            }
        }
    }

    if mounted_disks.is_empty() {
        println!("\nNo disks are currently mounted.");
        println!("Please connect at least one disk to the pool.");
        return Ok(());
    }

    println!("\nAvailable disks:");
    for disk in &mounted_disks {
        let available = disk.current_mount_point.as_ref()
            .and_then(|m| fs_adapter.available_space(std::path::Path::new(m)).ok())
            .unwrap_or(0);
        println!("  - {} [{}]: {} free", disk.name, disk.disk_id, format_size(available));
    }

    // Split into atomic units
    let mut all_units = Vec::new();
    for path in &valid_paths {
        let units = split_into_atomic_units(
            path,
            solid_layer_depth,
            None,
            None,
        )?;
        all_units.extend(units);
    }

    if all_units.is_empty() {
        println!("\nNo files to store.");
        return Ok(());
    }

    println!("\nAtomic units ({}):", all_units.len());
    for unit in &all_units {
        println!("  - {} ({}, {} files)", unit.name, format_size(unit.size), unit.file_count);
    }

    // Get disk space
    let mut disk_space: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for disk in &mounted_disks {
        if let Some(ref mount) = disk.current_mount_point {
            if let Ok(space) = fs_adapter.available_space(std::path::Path::new(mount)) {
                disk_space.insert(disk.disk_id.as_str().to_string(), space);
            }
        }
    }

    // Use BestFit strategy
    let strategy = BestFitStrategy::new();
    let plan = strategy.assign(all_units, &mounted_disks, disk_space)?;

    // Show plan
    println!("\nStorage Plan:");
    println!("{}", "-".repeat(80));

    let mut total_size = 0u64;
    let mut total_files = 0usize;

    for item in &plan {
        println!("  {} → {} [{}]",
            item.unit.name,
            item.target_disk_name,
            format_size(item.unit.size)
        );
        total_size += item.unit.size;
        total_files += item.unit.file_count;
    }

    println!("{}", "-".repeat(80));
    println!("Total: {} files, {}", total_files, format_size(total_size));

    if preview {
        println!("\nPreview mode - no files were copied.");
        return Ok(());
    }

    // Confirm
    if !yes {
        print!("\nProceed with storage? [y/N] ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();

        if !input.trim().to_lowercase().starts_with('y') {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Execute storage
    println!();
    print_header("Copying Files...");
    println!();

    let mut copied_files = 0usize;
    let mut copied_size = 0u64;
    let mut failed_count = 0usize;

    // Track copied paths for indexing
    let mut copied_paths: Vec<(String, std::path::PathBuf)> = Vec::new();

    for item in &plan {
        let disk = mounted_disks.iter()
            .find(|d| d.disk_id == item.target_disk)
            .expect("Disk should exist");

        let mount = disk.current_mount_point.as_ref().expect("Disk should be mounted");
        let dest_path = std::path::Path::new(mount).join(&item.target_relative_path);

        println!("  {} → {}", item.unit.name.cyan(), disk.name.green());
        println!("    Size: {}", format_size(item.unit.size));

        let source_path = std::path::Path::new(&item.unit.root_path);

        // Copy with progress
        let result = if source_path.is_dir() {
            copy_dir_with_progress(source_path, &dest_path)
        } else {
            copy_file_with_progress(source_path, &dest_path)
        };

        match result {
            Ok(size) => {
                println!("    {}", "✓ Copied successfully".green());
                copied_files += item.unit.file_count;
                copied_size += size;
                // Track for indexing
                copied_paths.push((disk.disk_id.as_str().to_string(), dest_path.clone()));
            }
            Err(e) => {
                println!("    {} Failed: {}", "✗".red(), e);
                failed_count += 1;
            }
        }
        println!();
    }

    print_separator();
    if copied_files > 0 {
        print_success(&format!("Stored {} files ({})", copied_files, format_size(copied_size)));
    }
    if failed_count > 0 {
        print_warning(&format!("Failed to store {} items", failed_count));
    }

    // Auto-index the newly stored files
    if !copied_paths.is_empty() {
        println!();
        print_header("Updating Index...");
        println!();

        let entry_repo = ctx.entry_repo();
        let hash_enabled = dedup; // Use dedup flag for hash calculation

        let mut total_indexed = 0usize;

        for (disk_id, dest_path) in &copied_paths {
            // Find the disk
            let disk = mounted_disks.iter()
                .find(|d| d.disk_id.as_str() == disk_id);

            if let Some(disk) = disk {
                if let Some(mount) = &disk.current_mount_point {
                    let mount_path = std::path::Path::new(mount);

                    println!("  Indexing {}...", dest_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("files"));

                    match scan_path(disk, mount_path, dest_path, &entry_repo, hash_enabled) {
                        Ok(report) => {
                            let indexed = report.files_added + report.dirs_added + report.files_updated + report.dirs_updated;
                            total_indexed += indexed;
                            println!("    {} Added {} files, {} dirs",
                                "✓".green(),
                                report.files_added,
                                report.dirs_added);
                            if report.files_updated > 0 || report.dirs_updated > 0 {
                                println!("    {} Updated {} files, {} dirs",
                                    "↻".yellow(),
                                    report.files_updated,
                                    report.dirs_updated);
                            }
                        }
                        Err(e) => {
                            println!("    {} Indexing failed: {}", "✗".red(), e);
                        }
                    }
                }
            }
        }

        println!();
        if total_indexed > 0 {
            print_success(&format!("Indexed {} entries", total_indexed));
        }
    }

    Ok(())
}

/// Copy a file with progress bar
fn copy_file_with_progress(source: &std::path::Path, dest: &std::path::Path) -> Result<u64> {
    use std::fs::File;
    use std::io::{BufReader, BufWriter, Read, Write};

    // Ensure destination directory exists
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let source_size = source.metadata()?.len();
    let source_file = File::open(source)?;
    let dest_file = File::create(dest)?;

    let mut reader = BufReader::with_capacity(64 * 1024, source_file);
    let mut writer = BufWriter::with_capacity(64 * 1024, dest_file);

    // Create progress bar
    let pb = ProgressBar::new(source_size);
    let style = ProgressStyle::default_bar()
        .template("    {spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
        .expect("Valid template")
        .progress_chars("#>-");
    pb.set_style(style);

    let mut copied = 0u64;
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        writer.write_all(&buffer[..bytes_read])?;
        copied += bytes_read as u64;
        pb.set_position(copied);
    }

    writer.flush()?;
    pb.finish_and_clear();

    Ok(copied)
}

/// Copy a directory recursively with progress
fn copy_dir_with_progress(source: &std::path::Path, dest: &std::path::Path) -> Result<u64> {
    use walkdir::WalkDir;
    use std::fs::File;
    use std::io::{BufReader, BufWriter, Read, Write};

    // Calculate total size first
    let total_size: u64 = WalkDir::new(source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum();

    // Create destination directory
    std::fs::create_dir_all(dest)?;

    // Create progress bar
    let pb = ProgressBar::new(total_size);
    let style = ProgressStyle::default_bar()
        .template("    {spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
        .expect("Valid template")
        .progress_chars("#>-");
    pb.set_style(style);

    let mut total_copied = 0u64;

    for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
        let relative = entry.path().strip_prefix(source)?;
        let dest_path = dest.join(relative);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest_path)?;
        } else if entry.file_type().is_file() {
            // Ensure parent directory exists
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file_size = entry.metadata()?.len();
            let source_file = File::open(entry.path())?;
            let dest_file = File::create(&dest_path)?;

            let mut reader = BufReader::with_capacity(64 * 1024, source_file);
            let mut writer = BufWriter::with_capacity(64 * 1024, dest_file);

            let mut buffer = [0u8; 64 * 1024];
            let mut file_copied = 0u64;

            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                writer.write_all(&buffer[..bytes_read])?;
                file_copied += bytes_read as u64;
                pb.set_position(total_copied + file_copied);
            }

            writer.flush()?;
            total_copied += file_size;
        }
    }

    pb.finish_and_clear();

    Ok(total_copied)
}