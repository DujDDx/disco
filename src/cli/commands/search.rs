//! Search command

use clap::Args;
use crate::Result;
use crate::cli::context::AppContext;
use crate::cli::display::format_size;
use crate::cli::interruptible::run_interruptible_search;
use crate::index::query::SearchOptions;
use crate::storage::platform::DiskDetector;
use crate::t;
use colored::Colorize;

/// Items per page for display
const ITEMS_PER_PAGE: usize = 20;

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
    let disk_repo = ctx.disk_repo();

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

    // Extract file entries
    let files: Vec<_> = results.iter().map(|r| &r.entry).collect();

    // Run interactive paginator
    run_interactive_paginator(
        &folder_matches,
        &files,
        &disk_repo,
        &detector,
        &mount_points,
        &keyword,
    )?;

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
    keyword: &str,
) -> Result<()> {
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

    let folder_count = folder_matches.len();
    let file_count = files.len();
    let total_items = folder_count + file_count;

    if total_items == 0 {
        return Ok(());
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
    loop {
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
                Span::styled(" 搜索结果: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(keyword, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(format!("文件夹: {}", folder_count), Style::default().fg(Color::Blue)),
                Span::raw("  "),
                Span::styled(format!("文件: {}", file_count), Style::default().fg(Color::Green)),
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
                        let is_highlighted = folder_is_active && folder_list_state.selected() == Some(idx - folder_start);

                        let folder = &folder_matches[idx];

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
                        let is_highlighted = file_is_active && file_list_state.selected() == Some(file_idx - file_start);

                        if let Some(entry) = files.get(file_idx) {
                            let name_style = if is_highlighted {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };

                            let mounted = mounted_disks.contains(&entry.disk_id);
                            let status = if mounted { "[ONLINE]" } else { "[OFFLINE]" };
                            let status_style = if mounted {
                                Style::default().fg(Color::Green)
                            } else {
                                Style::default().fg(Color::Red)
                            };

                            ListItem::new(Line::from(vec![
                                Span::styled(format!("[{:>3}] ", file_idx + 1), Style::default().fg(Color::Cyan)),
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
                Span::styled("q/Esc", Style::default().fg(Color::Cyan)),
                Span::raw(": 退出"),
            ]))
            .style(Style::default().fg(Color::DarkGray));
            f.render_widget(help, chunks[3]);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        break;
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
                    _ => {}
                }
            }
        }
    };

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    println!("{}", t!("action-goodbye"));

    Ok(())
}
