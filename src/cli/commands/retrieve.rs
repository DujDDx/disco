//! Retrieve command - Retrieve files from the disk pool

use clap::Args;
use crate::Result;
use crate::t;
use crate::cli::context::AppContext;
use crate::cli::display::{format_size, print_success, print_warning, print_info, print_header, print_separator};
use crate::cli::interruptible::{run_interruptible_search, DEFAULT_SEARCH_LIMIT};
use crate::storage::platform::DiskDetector;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};

/// Items per page for display
const ITEMS_PER_PAGE: usize = 20;

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

    // Search for files with interruptible search - use large limits
    println!();
    println!("Searching for: {}", keyword.cyan());

    // Use large limits to remove the 50/50 restriction
    let search_result = run_interruptible_search(&entry_repo, &keyword, DEFAULT_SEARCH_LIMIT, DEFAULT_SEARCH_LIMIT)?;

    // Extract results
    let entries = search_result.entries;
    let folder_matches = search_result.folder_matches;
    let was_interrupted = search_result.was_interrupted;

    // Separate files and directories
    let files: Vec<_> = entries
        .iter()
        .filter(|e| e.entry_type == crate::domain::entry::EntryType::File)
        .collect();

    let dirs: Vec<_> = entries
        .iter()
        .filter(|e| e.entry_type == crate::domain::entry::EntryType::Dir)
        .collect();

    if files.is_empty() && dirs.is_empty() && folder_matches.is_empty() {
        println!();
        if was_interrupted {
            print_warning("Search was interrupted before any results were found.");
        } else {
            print_warning("No files or folders found matching the keyword.");
        }
        return Ok(());
    }

    // Get mount points for status checking
    let mount_points = detector.list_mount_points()?;

    // Show interruption message if applicable
    println!();
    if was_interrupted {
        println!(
            "  [!] Search interrupted - showing {} results found so far",
            files.len() + folder_matches.len()
        );
    }

    // Run interactive paginator
    let indices = run_interactive_paginator(
        &folder_matches,
        &files,
        &disk_repo,
        &detector,
        &mount_points,
    )?;

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
            println!("[DIR] Retrieving folder: {}", folder.folder_name);

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
                        println!("    [X] Failed: {}", e);
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
                        println!("  [X] Failed: {} - {}", entry.file_name, e);
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

    println!("  [*] Copying {}...", entry.file_name);

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
        .template("    [{elapsed_precise}] [{bar:30}] {bytes}/{total_bytes} ({bytes_per_sec})")
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

    println!("    [OK] Saved to {}", dest.display());

    Ok(copied)
}
}

/// Active region for display
#[derive(Debug, Clone, Copy, PartialEq)]
enum ActiveRegion {
    Folders,
    Files,
}

/// Run interactive paginator using ratatui for stable display
fn run_interactive_paginator(
    folder_matches: &[crate::persistence::entry_repo::FolderMatch],
    files: &[&crate::domain::entry::IndexEntry],
    disk_repo: &crate::persistence::disk_repo::DiskRepo,
    detector: &dyn DiskDetector,
    mount_points: &[String],
) -> Result<Vec<usize>> {
    use crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
        Terminal,
    };
    use std::io;
    use std::time::Duration;

    // Pre-build mount status map for files
    let mut mounted_disks: std::collections::HashSet<crate::domain::disk::DiskId> = std::collections::HashSet::new();
    for mount in mount_points {
        if let Ok(identity) = detector.detect_identity(mount) {
            let disks = disk_repo.list_disks()?;
            for disk in &disks {
                if disk.identity.matches(&identity) {
                    mounted_disks.insert(disk.disk_id.clone());
                }
            }
        }
    }

    let mut selected_indices: Vec<usize> = Vec::new();
    let folder_count = folder_matches.len();
    let file_count = files.len();
    let total_items = folder_count + file_count;

    if total_items == 0 {
        return Ok(Vec::new());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // State - separate pagination for each region
    let mut active_region = if folder_count > 0 { ActiveRegion::Folders } else { ActiveRegion::Files };
    let mut folder_page: usize = 0;
    let mut file_page: usize = 0;
    let folder_total_pages = if folder_count > 0 { (folder_count + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE } else { 0 };
    let file_total_pages = if file_count > 0 { (file_count + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE } else { 0 };

    let mut folder_list_state = ListState::default();
    folder_list_state.select(Some(0));
    let mut file_list_state = ListState::default();
    file_list_state.select(Some(0));

    // Main loop
    let res: Result<Vec<usize>> = loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(2),  // Header
                    Constraint::Percentage(50),  // Folders region
                    Constraint::Percentage(50),  // Files region
                    Constraint::Length(2),  // Help
                ])
                .split(f.area());

            // Header
            let header = Paragraph::new(Line::from(vec![
                Span::styled(" 搜索结果 ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(format!("文件夹: {}", folder_count), Style::default().fg(Color::Blue)),
                Span::raw("  "),
                Span::styled(format!("文件: {}", file_count), Style::default().fg(Color::Green)),
                Span::raw("  "),
                Span::styled(format!("已选: {}", selected_indices.len()), Style::default().fg(Color::Yellow)),
            ]))
            .block(Block::default().borders(Borders::BOTTOM));
            f.render_widget(header, chunks[0]);

            // Folders region
            let folder_is_active = active_region == ActiveRegion::Folders;
            let folder_border_style = if folder_is_active {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            if folder_count > 0 {
                let folder_start = folder_page * ITEMS_PER_PAGE;
                let folder_end = (folder_start + ITEMS_PER_PAGE).min(folder_count);

                let folder_items: Vec<ListItem> = (folder_start..folder_end)
                    .map(|idx| {
                        let is_selected = selected_indices.contains(&idx);
                        let is_highlighted = folder_is_active && folder_list_state.selected() == Some(idx - folder_start);

                        let folder = &folder_matches[idx];
                        let selector = if is_selected { "[*] " } else { "[ ] " };
                        let selector_style = if is_selected {
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };

                        let name_style = if is_highlighted {
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };

                        // Check if all disks for this folder are online
                        let folder_disk_ids: Vec<crate::domain::disk::DiskId> = folder.disk_id_list()
                            .into_iter()
                            .map(|s| crate::domain::disk::DiskId::new(&s))
                            .collect();
                        let all_disks_online = folder_disk_ids.iter().all(|did| mounted_disks.contains(did));
                        let status = if all_disks_online { "[在线]" } else { "[离线]" };
                        let status_style = if all_disks_online {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default().fg(Color::Red)
                        };

                        // Get disk names for display
                        let disk_names_display = folder.disk_names.clone();

                        ListItem::new(Line::from(vec![
                            Span::styled(selector, selector_style),
                            Span::styled(format!("[{:>3}] ", idx + 1), Style::default().fg(Color::Cyan)),
                            Span::styled(status, status_style),
                            Span::raw(" "),
                            Span::styled(&folder.folder_name, name_style),
                            Span::raw("  "),
                            Span::styled(format!("({} 文件, {})", folder.file_count, format_size(folder.total_size)), Style::default().fg(Color::DarkGray)),
                            Span::raw(" - "),
                            Span::styled(disk_names_display, Style::default().fg(Color::DarkGray)),
                        ]))
                    })
                    .collect();

                let folder_title = format!("[FOLDERS] 第 {}/{} 页 (共 {} 项)",
                    folder_page + 1, folder_total_pages, folder_count);

                let folder_list = List::new(folder_items)
                    .block(Block::default()
                        .title(Span::styled(folder_title, Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)))
                        .borders(Borders::ALL)
                        .border_style(folder_border_style))
                    .highlight_style(Style::default().bg(Color::DarkGray))
                    .highlight_symbol("▶ ");

                f.render_stateful_widget(folder_list, chunks[1], &mut folder_list_state);
            } else {
                let empty_folder = Paragraph::new("无文件夹结果")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default()
                        .title("[FOLDERS]")
                        .borders(Borders::ALL)
                        .border_style(folder_border_style));
                f.render_widget(empty_folder, chunks[1]);
            }

            // Files region
            let file_is_active = active_region == ActiveRegion::Files;
            let file_border_style = if file_is_active {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            if file_count > 0 {
                let file_start = file_page * ITEMS_PER_PAGE;
                let file_end = (file_start + ITEMS_PER_PAGE).min(file_count);

                let file_items: Vec<ListItem> = (file_start..file_end)
                    .map(|file_idx| {
                        let idx = folder_count + file_idx;  // Global index
                        let is_selected = selected_indices.contains(&idx);
                        let is_highlighted = file_is_active && file_list_state.selected() == Some(file_idx - file_start);

                        if let Some(entry) = files.get(file_idx) {
                            let selector = if is_selected { "[*] " } else { "[ ] " };
                            let selector_style = if is_selected {
                                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            };

                            let name_style = if is_highlighted {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };

                            let mounted = mounted_disks.contains(&entry.disk_id);
                            let status = if mounted { "[在线]" } else { "[离线]" };
                            let status_style = if mounted {
                                Style::default().fg(Color::Green)
                            } else {
                                Style::default().fg(Color::Red)
                            };

                            ListItem::new(Line::from(vec![
                                Span::styled(selector, selector_style),
                                Span::styled(format!("[{:>3}] ", idx + 1), Style::default().fg(Color::Cyan)),
                                Span::styled(status, status_style),
                                Span::raw(" "),
                                Span::styled(&entry.file_name, name_style),
                                Span::raw("  "),
                                Span::styled(format!("({} - {})", format_size(entry.size), entry.disk_name), Style::default().fg(Color::DarkGray)),
                            ]))
                        } else {
                            ListItem::new(Line::from(""))
                        }
                    })
                    .collect();

                let file_title = format!("[FILES] 第 {}/{} 页 (共 {} 项)",
                    file_page + 1, file_total_pages, file_count);

                let file_list = List::new(file_items)
                    .block(Block::default()
                        .title(Span::styled(file_title, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
                        .borders(Borders::ALL)
                        .border_style(file_border_style))
                    .highlight_style(Style::default().bg(Color::DarkGray))
                    .highlight_symbol("▶ ");

                f.render_stateful_widget(file_list, chunks[2], &mut file_list_state);
            } else {
                let empty_file = Paragraph::new("无文件结果")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default()
                        .title("[FILES]")
                        .borders(Borders::ALL)
                        .border_style(file_border_style));
                f.render_widget(empty_file, chunks[2]);
            }

            // Help bar
            let help = Paragraph::new(Line::from(vec![
                Span::styled("Tab", Style::default().fg(Color::Cyan)),
                Span::raw(": 切换区域  "),
                Span::styled("↑↓", Style::default().fg(Color::Cyan)),
                Span::raw(": 导航  "),
                Span::styled("←→", Style::default().fg(Color::Cyan)),
                Span::raw(": 翻页  "),
                Span::styled("Space", Style::default().fg(Color::Cyan)),
                Span::raw(": 选择  "),
                Span::styled("a", Style::default().fg(Color::Cyan)),
                Span::raw(": 全选当前区  "),
                Span::styled("c", Style::default().fg(Color::Cyan)),
                Span::raw(": 清空  "),
                Span::styled("Enter", Style::default().fg(Color::Cyan)),
                Span::raw(": 确认  "),
                Span::styled("q/Esc", Style::default().fg(Color::Cyan)),
                Span::raw(": 取消"),
            ]))
            .style(Style::default().fg(Color::DarkGray));
            f.render_widget(help, chunks[3]);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        // Cleanup and return empty
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        println!("{}", t!("action-cancelled"));
                        return Ok(Vec::new());
                    }
                    KeyCode::Tab => {
                        // Toggle between regions
                        if active_region == ActiveRegion::Folders && file_count > 0 {
                            active_region = ActiveRegion::Files;
                        } else if active_region == ActiveRegion::Files && folder_count > 0 {
                            active_region = ActiveRegion::Folders;
                        }
                    }
                    KeyCode::Up => {
                        match active_region {
                            ActiveRegion::Folders => {
                                if let Some(selected) = folder_list_state.selected() {
                                    if selected > 0 {
                                        folder_list_state.select(Some(selected - 1));
                                    }
                                }
                            }
                            ActiveRegion::Files => {
                                if let Some(selected) = file_list_state.selected() {
                                    if selected > 0 {
                                        file_list_state.select(Some(selected - 1));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Down => {
                        match active_region {
                            ActiveRegion::Folders => {
                                let page_item_count = ((folder_page + 1) * ITEMS_PER_PAGE).min(folder_count) - folder_page * ITEMS_PER_PAGE;
                                if let Some(selected) = folder_list_state.selected() {
                                    if selected < page_item_count.saturating_sub(1) {
                                        folder_list_state.select(Some(selected + 1));
                                    }
                                }
                            }
                            ActiveRegion::Files => {
                                let page_item_count = ((file_page + 1) * ITEMS_PER_PAGE).min(file_count) - file_page * ITEMS_PER_PAGE;
                                if let Some(selected) = file_list_state.selected() {
                                    if selected < page_item_count.saturating_sub(1) {
                                        file_list_state.select(Some(selected + 1));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Left => {
                        match active_region {
                            ActiveRegion::Folders => {
                                if folder_page > 0 {
                                    folder_page -= 1;
                                    folder_list_state.select(Some(0));
                                }
                            }
                            ActiveRegion::Files => {
                                if file_page > 0 {
                                    file_page -= 1;
                                    file_list_state.select(Some(0));
                                }
                            }
                        }
                    }
                    KeyCode::Right => {
                        match active_region {
                            ActiveRegion::Folders => {
                                if folder_page < folder_total_pages.saturating_sub(1) {
                                    folder_page += 1;
                                    folder_list_state.select(Some(0));
                                }
                            }
                            ActiveRegion::Files => {
                                if file_page < file_total_pages.saturating_sub(1) {
                                    file_page += 1;
                                    file_list_state.select(Some(0));
                                }
                            }
                        }
                    }
                    KeyCode::Char(' ') => {
                        // Toggle selection based on active region
                        match active_region {
                            ActiveRegion::Folders => {
                                if let Some(selected) = folder_list_state.selected() {
                                    let idx = folder_page * ITEMS_PER_PAGE + selected;
                                    if let Some(pos) = selected_indices.iter().position(|&i| i == idx) {
                                        selected_indices.remove(pos);
                                    } else {
                                        selected_indices.push(idx);
                                    }
                                }
                            }
                            ActiveRegion::Files => {
                                if let Some(selected) = file_list_state.selected() {
                                    let file_idx = file_page * ITEMS_PER_PAGE + selected;
                                    let idx = folder_count + file_idx;  // Global index
                                    if let Some(pos) = selected_indices.iter().position(|&i| i == idx) {
                                        selected_indices.remove(pos);
                                    } else {
                                        selected_indices.push(idx);
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if selected_indices.is_empty() {
                            // If nothing selected, select current item
                            match active_region {
                                ActiveRegion::Folders => {
                                    if let Some(selected) = folder_list_state.selected() {
                                        let idx = folder_page * ITEMS_PER_PAGE + selected;
                                        selected_indices.push(idx);
                                    }
                                }
                                ActiveRegion::Files => {
                                    if let Some(selected) = file_list_state.selected() {
                                        let file_idx = file_page * ITEMS_PER_PAGE + selected;
                                        let idx = folder_count + file_idx;
                                        selected_indices.push(idx);
                                    }
                                }
                            }
                        }
                        break Ok(selected_indices.clone());
                    }
                    KeyCode::Char('a') => {
                        // Select all in current region
                        match active_region {
                            ActiveRegion::Folders => {
                                for idx in 0..folder_count {
                                    if !selected_indices.contains(&idx) {
                                        selected_indices.push(idx);
                                    }
                                }
                            }
                            ActiveRegion::Files => {
                                for file_idx in 0..file_count {
                                    let idx = folder_count + file_idx;
                                    if !selected_indices.contains(&idx) {
                                        selected_indices.push(idx);
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('c') => {
                        // Clear selection
                        selected_indices.clear();
                    }
                    _ => {}
                }
            }
        }
    };

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    if !selected_indices.is_empty() {
        println!();
        println!("{}", t!("action-complete"));
    }

    res
}
