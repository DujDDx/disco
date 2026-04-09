//! Visualize command - Terminal UI interface

use clap::Args;
use crate::Result;
use crate::cli::context::AppContext;
use crate::storage::platform::DiskDetector;
use crate::cli::display::format_size;
use std::collections::HashMap;

/// Open terminal visualization interface
#[derive(Args, Debug)]
pub struct VisualizeCmd {
    /// Start in disk usage view (treemap)
    #[arg(short, long)]
    usage: bool,

    /// Start in tree view
    #[arg(short, long, conflicts_with = "usage")]
    tree: bool,

    /// Filter to specific disk
    #[arg(short, long)]
    disk: Option<String>,
}

pub fn handle_visualize(cmd: VisualizeCmd) -> Result<()> {
    let ctx = AppContext::init()?;
    handle_visualize_with_ctx(&ctx, cmd.disk)
}

pub fn handle_visualize_with_ctx(ctx: &AppContext, disk: Option<String>) -> Result<()> {
    let detector = AppContext::disk_detector();
    let disk_repo = ctx.disk_repo();

    // Get disks
    let disks = disk_repo.list_disks()?;
    let mount_points = detector.list_mount_points()?;

    if disks.is_empty() {
        println!("No disks registered. Use 'disco disk add <mount-point>' to add disks.");
        return Ok(());
    }

    // Run TUI
    run_tui(ctx, &disks, &mount_points, disk.as_deref())?;

    Ok(())
}

/// View mode for TUI
#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    DiskList,
    FolderTree,
    FolderUsage,
}

/// Folder info for tree view
#[derive(Debug, Clone)]
struct FolderInfo {
    name: String,
    path: String,        // Full relative path
    size: u64,
    file_count: usize,
    is_folder: bool,     // true if it's a subfolder, false if it's a file
}

/// Build folder tree from entries, optionally filtering by parent path
fn build_folder_tree(entries: &[crate::domain::entry::IndexEntry], parent_path: Option<&str>) -> Vec<FolderInfo> {
    let mut folders: HashMap<String, FolderInfo> = HashMap::new();
    let mut files: Vec<FolderInfo> = Vec::new();

    for entry in entries {
        let relative = &entry.relative_path;

        // If we have a parent path filter, check if this entry is under it
        let path_to_check = if let Some(parent) = parent_path {
            if !relative.starts_with(parent) || relative == parent {
                continue;
            }
            // Get the part after parent
            relative.strip_prefix(parent).unwrap_or(relative).trim_start_matches('/')
        } else {
            relative.as_str()
        };

        let path_parts: Vec<&str> = path_to_check.split('/').filter(|s| !s.is_empty()).collect();
        if path_parts.is_empty() {
            continue;
        }

        // Get first component (folder or file)
        let first_part = path_parts[0];

        if path_parts.len() == 1 {
            // This is a file at current level
            files.push(FolderInfo {
                name: first_part.to_string(),
                path: if let Some(p) = parent_path {
                    format!("{}/{}", p, first_part)
                } else {
                    first_part.to_string()
                },
                size: entry.size,
                file_count: 1,
                is_folder: false,
            });
        } else {
            // This is a subfolder - aggregate stats
            let full_path = if let Some(p) = parent_path {
                format!("{}/{}", p, first_part)
            } else {
                first_part.to_string()
            };

            let folder = folders.entry(first_part.to_string()).or_insert_with(|| FolderInfo {
                name: first_part.to_string(),
                path: full_path.clone(),
                size: 0,
                file_count: 0,
                is_folder: true,
            });

            folder.size += entry.size;
            folder.file_count += 1;
        }
    }

    // Convert folders to vector and sort by size
    let mut result: Vec<FolderInfo> = folders.into_values().collect();
    result.sort_by(|a, b| b.size.cmp(&a.size));

    // Add files (sort by size too)
    files.sort_by(|a, b| b.size.cmp(&a.size));
    result.extend(files);

    result
}

fn run_tui(
    ctx: &AppContext,
    disks: &[crate::domain::disk::Disk],
    mount_points: &[String],
    filter_disk: Option<&str>,
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
    use std::time::{Duration, Instant};

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut view_mode = ViewMode::DiskList;
    let mut selected_disk: usize = 0;
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    // Build disk list with mount status
    let entry_repo = ctx.entry_repo();
    let detector = crate::storage::platform::get_detector();

    // Disk info: (disk, is_mounted, total_size, file_count, current_mount)
    let mut disk_info: Vec<(crate::domain::disk::Disk, bool, u64, usize, Option<String>)> = Vec::new();
    for disk in disks {
        // Skip if filtering
        if let Some(filter) = filter_disk {
            if !disk.name.contains(filter) && !disk.disk_id.as_str().contains(filter) {
                continue;
            }
        }

        // Check mount status
        let mut is_mounted = false;
        let mut current_mount: Option<String> = None;

        for mount in mount_points {
            if let Ok(identity) = detector.detect_identity(mount) {
                if disk.identity.matches(&identity) {
                    is_mounted = true;
                    current_mount = Some(mount.clone());
                    break;
                }
            }
        }

        // Get entry count
        let entries = entry_repo.get_entries_by_disk(&disk.disk_id)?;
        let file_count = entries.iter().filter(|e| e.entry_type == crate::domain::entry::EntryType::File).count();
        let total_size: u64 = entries.iter().map(|e| e.size).sum();

        disk_info.push((disk.clone(), is_mounted, total_size, file_count, current_mount));
    }

    if disk_info.is_empty() {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        println!("No matching disks found.");
        return Ok(());
    }

    // Folder tree state
    let mut current_disk_entries: Vec<crate::domain::entry::IndexEntry> = Vec::new();
    let mut folder_tree: Vec<FolderInfo> = Vec::new();
    let mut selected_folder: usize = 0;
    let mut folder_list_state = ListState::default();
    let mut current_folder_path: Option<String> = None;  // Track current folder for navigation
    folder_list_state.select(Some(0));

    // Main loop
    let res: Result<()> = loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Min(10),
                    Constraint::Length(2),
                ])
                .split(f.area());

            // Title bar - show current path for folder views
            let view_name = match view_mode {
                ViewMode::DiskList => "硬盘列表 Disk List".to_string(),
                ViewMode::FolderTree => {
                    if let Some(ref path) = current_folder_path {
                        format!("文件夹: /{}", path)
                    } else {
                        "文件夹树 Folder Tree (根目录)".to_string()
                    }
                }
                ViewMode::FolderUsage => {
                    if let Some(ref path) = current_folder_path {
                        format!("空间占用: /{}", path)
                    } else {
                        "空间占用 Usage View (根目录)".to_string()
                    }
                }
            };
            let title = Paragraph::new(format!(" {} ", view_name))
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::BOTTOM));
            f.render_widget(title, chunks[0]);

            match view_mode {
                ViewMode::DiskList => {
                    let items: Vec<ListItem> = disk_info
                        .iter()
                        .enumerate()
                        .map(|(i, (disk, mounted, size, files, mount))| {
                            let status_icon = if *mounted { "●" } else { "○" };
                            let status_color = if *mounted { Color::Green } else { Color::Red };
                            let name_style = if i == selected_disk {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };

                            let mount_info = mount.as_deref().map(|m| format!(" ({})", m)).unwrap_or_default();

                            ListItem::new(Line::from(vec![
                                Span::styled(format!("{} ", status_icon), Style::default().fg(status_color)),
                                Span::styled(&disk.name, name_style),
                                Span::styled(format!(" [{}]", disk.disk_id), Style::default().fg(Color::DarkGray)),
                                Span::styled(mount_info, Style::default().fg(Color::Green)),
                                Span::raw("  "),
                                Span::styled(format!("{} 文件", files), Style::default().fg(Color::Cyan)),
                                Span::raw("  "),
                                Span::styled(format_size(*size), Style::default().fg(Color::Magenta)),
                            ]))
                        })
                        .collect();

                    let list = List::new(items)
                        .block(Block::default()
                            .title(" 硬盘 Disks (Enter: 文件夹 | U: 占用视图) ")
                            .borders(Borders::ALL))
                        .highlight_style(Style::default().bg(Color::DarkGray))
                        .highlight_symbol("▶ ");

                    f.render_stateful_widget(list, chunks[1], &mut list_state);
                }
                ViewMode::FolderTree => {
                    // Folder list with folder/file icons
                    let items: Vec<ListItem> = folder_tree
                        .iter()
                        .enumerate()
                        .map(|(i, folder)| {
                            let is_selected = folder_list_state.selected() == Some(i);
                            let style = if is_selected {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };

                            // Show folder icon for folders, file icon for files
                            let icon = if folder.is_folder { "📁 " } else { "📄 " };
                            let info_text = if folder.is_folder {
                                format!("({} 项)", folder.file_count)
                            } else {
                                format!("({})", format_size(folder.size))
                            };

                            ListItem::new(Line::from(vec![
                                Span::styled(icon, Style::default()),
                                Span::styled(&folder.name, style),
                                Span::raw("  "),
                                Span::styled(format_size(folder.size), Style::default().fg(Color::Cyan)),
                                Span::raw("  "),
                                Span::styled(info_text, Style::default().fg(Color::DarkGray)),
                            ]))
                        })
                        .collect();

                    let path_info = current_folder_path.as_ref()
                        .map(|p| format!("路径: /{}", p))
                        .unwrap_or_else(|| "根目录".to_string());

                    let list = List::new(items)
                        .block(Block::default()
                            .title(format!(" {} (Enter进入 | Backspace返回 | 共{}项) ", path_info, folder_tree.len()))
                            .borders(Borders::ALL))
                        .highlight_style(Style::default().bg(Color::DarkGray))
                        .highlight_symbol("▶ ");

                    f.render_stateful_widget(list, chunks[1], &mut folder_list_state);
                }
                ViewMode::FolderUsage => {
                    // Calculate max size for scaling
                    let max_size = folder_tree.iter().map(|f| f.size).max().unwrap_or(1);

                    // Folder list with bars
                    let items: Vec<ListItem> = folder_tree
                        .iter()
                        .enumerate()
                        .map(|(i, folder)| {
                            let is_selected = folder_list_state.selected() == Some(i);
                            let style = if is_selected {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };

                            let ratio = if max_size > 0 { folder.size as f64 / max_size as f64 } else { 0.0 };
                            let bar_count = (ratio * 20.0) as usize;
                            let bar: String = "▓".repeat(bar_count) + &"░".repeat(20 - bar_count);

                            ListItem::new(Line::from(vec![
                                Span::styled(&folder.name, style),
                                Span::raw(" "),
                                Span::styled(bar, Style::default().fg(Color::Cyan)),
                                Span::raw(" "),
                                Span::styled(format_size(folder.size), Style::default().fg(Color::Green)),
                            ]))
                        })
                        .collect();

                    let path_info = current_folder_path.as_ref()
                        .map(|p| format!("路径: /{}", p))
                        .unwrap_or_else(|| "根目录".to_string());

                    let list = List::new(items)
                        .block(Block::default()
                            .title(format!(" {} (共{}项) ", path_info, folder_tree.len()))
                            .borders(Borders::ALL))
                        .highlight_style(Style::default().bg(Color::DarkGray))
                        .highlight_symbol("▶ ");

                    f.render_stateful_widget(list, chunks[1], &mut folder_list_state);
                }
            }

            // Help bar
            let help_text = match view_mode {
                ViewMode::DiskList => "↑↓: Navigate │ Enter: Folders │ U: Usage │ Q: Quit",
                ViewMode::FolderTree => "↑↓: Navigate │ Enter: 进入文件夹 │ Backspace/Esc: 返回 │ Q: Quit",
                ViewMode::FolderUsage => "↑↓: Select │ ←→: 切换硬盘 │ Enter: 进入 │ Backspace: 返回 │ Q: Quit",
            };
            let help = Paragraph::new(help_text)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(help, chunks[2]);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match view_mode {
                    ViewMode::DiskList => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                            KeyCode::Up => {
                                if selected_disk > 0 {
                                    selected_disk -= 1;
                                    list_state.select(Some(selected_disk));
                                }
                            }
                            KeyCode::Down => {
                                if selected_disk < disk_info.len() - 1 {
                                    selected_disk += 1;
                                    list_state.select(Some(selected_disk));
                                }
                            }
                            KeyCode::Enter => {
                                if let Some((disk, mounted, _, _, _)) = disk_info.get(selected_disk) {
                                    if *mounted {
                                        current_disk_entries = entry_repo.get_entries_by_disk(&disk.disk_id)?;
                                        current_folder_path = None;  // Reset to root
                                        folder_tree = build_folder_tree(&current_disk_entries, None);
                                        selected_folder = 0;
                                        folder_list_state.select(Some(0));
                                        view_mode = ViewMode::FolderTree;
                                    }
                                }
                            }
                            KeyCode::Char('u') => {
                                if let Some((disk, mounted, _, _, _)) = disk_info.get(selected_disk) {
                                    if *mounted {
                                        current_disk_entries = entry_repo.get_entries_by_disk(&disk.disk_id)?;
                                        current_folder_path = None;  // Reset to root
                                        folder_tree = build_folder_tree(&current_disk_entries, None);
                                        selected_folder = 0;
                                        folder_list_state.select(Some(0));
                                        view_mode = ViewMode::FolderUsage;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    ViewMode::FolderTree => {
                        match key.code {
                            KeyCode::Char('q') => break Ok(()),
                            KeyCode::Esc => {
                                // If in subfolder, go back to parent; otherwise go to disk list
                                if current_folder_path.is_some() {
                                    // Go back to parent folder
                                    if let Some(path) = &current_folder_path {
                                        let parts: Vec<&str> = path.rsplitn(2, '/').collect();
                                        current_folder_path = if parts.len() > 1 {
                                            Some(parts[1].to_string())
                                        } else {
                                            None
                                        };
                                    }
                                    folder_tree = build_folder_tree(&current_disk_entries, current_folder_path.as_deref());
                                    selected_folder = 0;
                                    folder_list_state.select(Some(0));
                                } else {
                                    view_mode = ViewMode::DiskList;
                                }
                            }
                            KeyCode::Up => {
                                if selected_folder > 0 {
                                    selected_folder -= 1;
                                    folder_list_state.select(Some(selected_folder));
                                }
                            }
                            KeyCode::Down => {
                                if selected_folder < folder_tree.len().saturating_sub(1) {
                                    selected_folder += 1;
                                    folder_list_state.select(Some(selected_folder));
                                }
                            }
                            KeyCode::Enter => {
                                // Enter selected folder if it's a folder (not a file)
                                if let Some(folder) = folder_tree.get(selected_folder) {
                                    if folder.is_folder {
                                        current_folder_path = Some(folder.path.clone());
                                        folder_tree = build_folder_tree(&current_disk_entries, current_folder_path.as_deref());
                                        selected_folder = 0;
                                        folder_list_state.select(Some(0));
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                // Go back to parent folder
                                if let Some(path) = &current_folder_path {
                                    let parts: Vec<&str> = path.rsplitn(2, '/').collect();
                                    current_folder_path = if parts.len() > 1 {
                                        Some(parts[1].to_string())
                                    } else {
                                        None
                                    };
                                }
                                folder_tree = build_folder_tree(&current_disk_entries, current_folder_path.as_deref());
                                selected_folder = 0;
                                folder_list_state.select(Some(0));
                            }
                            _ => {}
                        }
                    }
                    ViewMode::FolderUsage => {
                        match key.code {
                            KeyCode::Char('q') => break Ok(()),
                            KeyCode::Esc => {
                                // If in subfolder, go back to parent; otherwise go to disk list
                                if current_folder_path.is_some() {
                                    if let Some(path) = &current_folder_path {
                                        let parts: Vec<&str> = path.rsplitn(2, '/').collect();
                                        current_folder_path = if parts.len() > 1 {
                                            Some(parts[1].to_string())
                                        } else {
                                            None
                                        };
                                    }
                                    folder_tree = build_folder_tree(&current_disk_entries, current_folder_path.as_deref());
                                    selected_folder = 0;
                                    folder_list_state.select(Some(0));
                                } else {
                                    view_mode = ViewMode::DiskList;
                                }
                            }
                            KeyCode::Up => {
                                if selected_folder > 0 {
                                    selected_folder -= 1;
                                    folder_list_state.select(Some(selected_folder));
                                }
                            }
                            KeyCode::Down => {
                                if selected_folder < folder_tree.len().saturating_sub(1) {
                                    selected_folder += 1;
                                    folder_list_state.select(Some(selected_folder));
                                }
                            }
                            KeyCode::Left => {
                                if selected_disk > 0 {
                                    selected_disk -= 1;
                                    if let Some((disk, mounted, _, _, _)) = disk_info.get(selected_disk) {
                                        if *mounted {
                                            current_disk_entries = entry_repo.get_entries_by_disk(&disk.disk_id)?;
                                            current_folder_path = None;
                                            folder_tree = build_folder_tree(&current_disk_entries, None);
                                            selected_folder = 0;
                                            folder_list_state.select(Some(0));
                                        }
                                    }
                                }
                            }
                            KeyCode::Right => {
                                if selected_disk < disk_info.len() - 1 {
                                    selected_disk += 1;
                                    if let Some((disk, mounted, _, _, _)) = disk_info.get(selected_disk) {
                                        if *mounted {
                                            current_disk_entries = entry_repo.get_entries_by_disk(&disk.disk_id)?;
                                            current_folder_path = None;
                                            folder_tree = build_folder_tree(&current_disk_entries, None);
                                            selected_folder = 0;
                                            folder_list_state.select(Some(0));
                                        }
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                // Enter selected folder if it's a folder
                                if let Some(folder) = folder_tree.get(selected_folder) {
                                    if folder.is_folder {
                                        current_folder_path = Some(folder.path.clone());
                                        folder_tree = build_folder_tree(&current_disk_entries, current_folder_path.as_deref());
                                        selected_folder = 0;
                                        folder_list_state.select(Some(0));
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                // Go back to parent folder
                                if let Some(path) = &current_folder_path {
                                    let parts: Vec<&str> = path.rsplitn(2, '/').collect();
                                    current_folder_path = if parts.len() > 1 {
                                        Some(parts[1].to_string())
                                    } else {
                                        None
                                    };
                                }
                                folder_tree = build_folder_tree(&current_disk_entries, current_folder_path.as_deref());
                                selected_folder = 0;
                                folder_list_state.select(Some(0));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    };

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    res
}