//! Retrieve command - Retrieve files from the disk pool

use clap::Args;
use crate::Result;
use crate::cli::context::AppContext;
use crate::cli::display::{format_size, print_success, print_warning, print_info, print_header, print_separator};
use crate::storage::platform::DiskDetector;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};

/// Retrieve files from the disk pool
#[derive(Args, Debug)]
pub struct RetrieveCmd {
    /// Search keyword to find files
    #[arg(required = true)]
    keyword: String,

    /// Destination directory to retrieve files to
    #[arg(short, long)]
    dest: Option<String>,

    /// File extension filter
    #[arg(long)]
    ext: Option<String>,

    /// Maximum number of results to show
    #[arg(long, default_value = "20")]
    limit: usize,

    /// Retrieve all matching files without confirmation
    #[arg(short, long)]
    all: bool,

    /// Search for folders instead of files
    #[arg(short, long)]
    folder: bool,
}

pub fn handle_retrieve(cmd: RetrieveCmd) -> Result<()> {
    let ctx = AppContext::init()?;
    handle_retrieve_with_ctx(&ctx, cmd.keyword)
}

pub fn handle_retrieve_with_ctx(ctx: &AppContext, keyword: String) -> Result<()> {
    let detector = AppContext::disk_detector();
    let entry_repo = ctx.entry_repo();
    let disk_repo = ctx.disk_repo();

    // Search for files
    println!();
    println!("Searching for: {}", keyword.cyan());

    let entries = entry_repo.search_by_name(&keyword, 100)?;

    // Separate files and directories
    let files: Vec<_> = entries
        .iter()
        .filter(|e| e.entry_type == crate::domain::entry::EntryType::File)
        .collect();

    let dirs: Vec<_> = entries
        .iter()
        .filter(|e| e.entry_type == crate::domain::entry::EntryType::Dir)
        .collect();

    // Also search for folder names (aggregated across disks)
    let folder_matches = entry_repo.search_folder_names(&keyword, 50)?;

    if files.is_empty() && dirs.is_empty() && folder_matches.is_empty() {
        println!();
        print_warning("No files or folders found matching the keyword.");
        return Ok(());
    }

    // Get mount points for status checking
    let mount_points = detector.list_mount_points()?;

    // Display results
    println!();
    print_header("Search Results:");
    println!();

    let mut result_index = 0usize;

    // Show aggregated folder matches first
    if !folder_matches.is_empty() {
        println!("{}", "  Folders (aggregated across disks):".yellow().bold());
        println!();

        for folder in &folder_matches {
            let split_indicator = if folder.is_split() {
                format!(" {} ", "[拆分/Split]".magenta())
            } else {
                String::new()
            };

            println!(
                "  {} {} {}{}[{} files, {}]",
                format!("[{}]", result_index + 1).bright_black(),
                "📁".to_string(),
                folder.folder_name.white().bold(),
                split_indicator,
                folder.file_count.to_string().cyan(),
                format_size(folder.total_size).green()
            );

            // Show which disks this folder is on
            let disk_names: Vec<&str> = folder.disk_names.split(',').collect();
            if disk_names.len() > 1 {
                println!("      {} {}", "→".bright_black(), folder.disk_names.yellow());
            } else {
                println!("      {} {}", "→".bright_black(), folder.disk_names.bright_black());
            }

            result_index += 1;
        }
        println!();
    }

    // Show individual files
    if !files.is_empty() {
        println!("{}", "  Files:".cyan().bold());
        println!();

        for entry in &files {
            // Check if disk is mounted
            let disk = disk_repo.get_disk_by_id(&entry.disk_id);
            let mounted = match disk {
                Ok(ref d) => {
                    let mut found = false;
                    for mount in &mount_points {
                        if let Ok(identity) = detector.detect_identity(mount) {
                            if d.identity.matches(&identity) {
                                found = true;
                                break;
                            }
                        }
                    }
                    found
                }
                Err(_) => false,
            };

            let status = if mounted {
                "●".green()
            } else {
                "○".red()
            };

            println!(
                "  {} {} {} {} [{}] {}",
                format!("[{}]", result_index + 1).bright_black(),
                status,
                "📄".to_string(),
                entry.file_name.white(),
                format_size(entry.size).cyan(),
                entry.disk_name.bright_black()
            );

            result_index += 1;
        }
    }

    println!();
    println!("  Total: {} folders, {} files", folder_matches.len(), files.len());
    println!();

    // Ask which items to retrieve
    print!("Enter numbers to retrieve (e.g., 1,3,5) or 'all': ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim();

    let total_items = folder_matches.len() + files.len();
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

    if indices.is_empty() {
        print_info("No items selected.");
        return Ok(());
    }

    // Ask for destination
    print!("Enter destination directory [default: ./]: ");
    io::stdout().flush().ok();
    let mut dest_input = String::new();
    io::stdin().read_line(&mut dest_input).ok();
    let dest_dir = if dest_input.trim().is_empty() {
        std::env::current_dir()?
    } else {
        std::path::PathBuf::from(dest_input.trim())
    };

    // Create destination if needed
    if !dest_dir.exists() {
        std::fs::create_dir_all(&dest_dir)?;
        println!("Created directory: {}", dest_dir.display());
    }

    println!();
    print_header("Retrieving Files...");
    println!();

    let mut success_count = 0;
    let mut failed_count = 0;
    let mut total_size = 0u64;

    // Process selected indices
    for &idx in &indices {
        let folder_count = folder_matches.len();

        if idx < folder_count {
            // This is a folder - retrieve all files in it
            let folder = &folder_matches[idx];
            println!("{}", format!("📁 Retrieving folder: {}", folder.folder_name).cyan().bold());

            // Get all entries for this folder
            let folder_entries = entry_repo.search_by_path_prefix(&folder.folder_name, 10000)?;

            // Filter to files only and group by disk
            let folder_files: Vec<_> = folder_entries
                .iter()
                .filter(|e| e.entry_type == crate::domain::entry::EntryType::File)
                .collect();

            println!("  Found {} files across {} disk(s)", folder_files.len(), folder.disk_id_list().len());

            // Create subfolder in destination
            let folder_dest = dest_dir.join(&folder.folder_name);
            if !folder_dest.exists() {
                std::fs::create_dir_all(&folder_dest)?;
            }

            // Retrieve each file
            for file_entry in &folder_files {
                match retrieve_file(file_entry, &disk_repo, &detector, &mount_points, &folder_dest) {
                    Ok(size) => {
                        success_count += 1;
                        total_size += size;
                    }
                    Err(e) => {
                        println!("    {} Failed: {}", "✗".red(), e);
                        failed_count += 1;
                    }
                }
            }
            println!();
        } else {
            // This is a file
            let file_idx = idx - folder_count;
            if let Some(entry) = files.get(file_idx) {
                match retrieve_file(entry, &disk_repo, &detector, &mount_points, &dest_dir) {
                    Ok(size) => {
                        success_count += 1;
                        total_size += size;
                    }
                    Err(e) => {
                        println!("  {} Failed: {}", entry.file_name.red(), e);
                        failed_count += 1;
                    }
                }
            }
        }
    }

    println!();
    print_separator();
    if success_count > 0 {
        print_success(&format!("Retrieved {} files ({})", success_count, format_size(total_size)));
    }
    if failed_count > 0 {
        print_warning(&format!("Failed to retrieve {} files", failed_count));
    }

    Ok(())
}

/// Retrieve a single file
fn retrieve_file(
    entry: &crate::domain::entry::IndexEntry,
    disk_repo: &crate::persistence::disk_repo::DiskRepo,
    detector: &dyn DiskDetector,
    mount_points: &[String],
    dest_dir: &std::path::Path,
) -> Result<u64> {
    // Get disk info
    let disk = disk_repo.get_disk_by_id(&entry.disk_id)?;

    // Find mount point
    let mut current_mount: Option<String> = None;
    for mount in mount_points {
        if let Ok(identity) = detector.detect_identity(mount) {
            if disk.identity.matches(&identity) {
                current_mount = Some(mount.clone());
                break;
            }
        }
    }

    let mount = match current_mount {
        Some(m) => m,
        None => {
            return Err(crate::DiscoError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Disk '{}' not mounted", disk.name),
            )));
        }
    };

    // Build source and destination paths
    let source_path = std::path::Path::new(&mount).join(&entry.relative_path);

    // Preserve folder structure in destination
    let dest_path = if entry.relative_path.contains('/') {
        // Has subfolders - preserve structure
        dest_dir.join(&entry.relative_path)
    } else {
        dest_dir.join(&entry.file_name)
    };

    // Handle duplicate filenames
    let dest_path = if dest_path.exists() {
        let mut counter = 1;
        let (stem, ext): (String, String) = entry.file_name.rsplit_once('.')
            .map(|(name, ext)| (name.to_string(), format!(".{}", ext)))
            .unwrap_or((entry.file_name.clone(), String::new()));
        let mut new_path = dest_dir.join(format!("{}_{}{}", stem, counter, ext));
        while new_path.exists() {
            counter += 1;
            new_path = dest_dir.join(format!("{}_{}{}", stem, counter, ext));
        }
        new_path
    } else {
        dest_path
    };

    println!("  {} Copying {}...", "→".bright_black(), entry.file_name.cyan());

    // Copy with progress
    copy_file_with_progress(&source_path, &dest_path)
}

/// Copy a file with progress bar
fn copy_file_with_progress(source: &std::path::Path, dest: &std::path::Path) -> Result<u64> {
    use std::fs::File;
    use std::io::{BufReader, BufWriter, Read, Write};

    // Ensure parent directory exists
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

    println!("    {} Saved to {}", "✓".green(), dest.display().to_string().green());

    Ok(copied)
}