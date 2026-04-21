//! Interactive shell mode for Disco CLI

use crate::cli::context::AppContext;
use crate::cli::commands::{
    disk::{handle_add_with_ctx, handle_list_with_ctx},
    scan::handle_scan_with_ctx,
    search::handle_search_with_ctx,
    get::handle_get_with_ctx,
    store::handle_store_with_ctx,
    solid::{handle_set_with_ctx, handle_unset_with_ctx},
    visualize::handle_visualize_with_ctx,
    retrieve::handle_retrieve_with_ctx,
};
use crate::cli::display::{format_size, format_mount_status_colored, print_success, print_error, print_warning, print_info, print_header, print_separator, format_disk_name, format_disk_id, print_disk_error};
use crate::domain::disk::DiskId;
use crate::storage::platform::{DiskDetector, get_detector};
use crate::Result;
use crate::t;
use rustyline::{Config, Editor, error::ReadlineError, history::DefaultHistory};
use std::io::{self, Write};
use colored::Colorize;

/// Run the interactive shell
pub fn run_interactive() -> Result<()> {
    // Initialize context (shared across all commands)
    let ctx = AppContext::init()?;

    // Create editor with history
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(rustyline::CompletionType::List)
        .build();
    let mut editor: Editor<(), DefaultHistory> = Editor::with_config(config)
        .map_err(|e: ReadlineError| crate::DiscoError::ConfigError(e.to_string()))?;

    // Load history
    let history_path = ctx.data_dir.join("history.txt");
    if history_path.exists() {
        let _ = editor.load_history(&history_path);
    }

    // Print welcome message
    println!();
    print_header(&t!("shell-welcome-title"));
    print_info(&t!("shell-welcome-help"));
    print_info(&t!("shell-welcome-menu"));
    println!();

    // REPL loop
    loop {
        let readline = editor.readline(&format!("{}", "disco> ".green().bold()));
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let _ = editor.add_history_entry(line);

                match dispatch(&ctx, line) {
                    Ok(should_exit) => {
                        if should_exit {
                            break;
                        }
                    }
                    Err(e) => {
                        print_disk_error(&e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", t!("shell-interrupted").yellow());
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", t!("shell-eof").yellow());
                break;
            }
            Err(e) => {
                print_error(&crate::t!("shell-input-error", "error" => e.to_string()));
                break;
            }
        }
    }

    // Save history
    let _ = editor.save_history(&history_path);
    print_success(&t!("action-goodbye"));

    Ok(())
}

/// Run menu mode directly (for `disco menu` command)
pub fn run_menu_direct() -> Result<()> {
    let ctx = AppContext::init()?;
    run_menu_mode(&ctx)?;
    Ok(())
}

/// Parse a shell line into tokens (handling quotes and escapes)
fn parse_shell_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';
    let mut escape_next = false;

    for ch in line.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        if ch == '\\' {
            escape_next = true;
            continue;
        }

        if in_quotes {
            if ch == quote_char {
                in_quotes = false;
            } else {
                current.push(ch);
            }
        } else if ch == '"' || ch == '\'' {
            in_quotes = true;
            quote_char = ch;
        } else if ch == ' ' || ch == '\t' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Dispatch a command line to the appropriate handler
fn dispatch(ctx: &AppContext, line: &str) -> Result<bool> {
    let tokens = parse_shell_line(line);
    if tokens.is_empty() {
        return Ok(false);
    }

    let disk_repo = ctx.disk_repo();

    match tokens[0].as_str() {
        "help" | "?" => {
            show_help(tokens.get(1).map(|s| s.as_str()));
            Ok(false)
        }
        "exit" | "quit" | "q" => {
            Ok(true)
        }
        "menu" | "m" => {
            // Launch menu mode
            run_menu_mode(ctx)?;
            Ok(false)
        }
        "status" => {
            handle_status(ctx)?;
            Ok(false)
        }
        "refresh" => {
            handle_refresh(ctx)?;
            Ok(false)
        }
        "repair" => {
            handle_repair(ctx)?;
            Ok(false)
        }
        "disk" => {
            if tokens.len() < 2 {
                print_warning(&t!("usage-disk"));
                return Ok(false);
            }
            match tokens[1].as_str() {
                "add" => {
                    if tokens.len() < 3 {
                        print_warning(&t!("usage-disk-add"));
                        return Ok(false);
                    }
                    let mount_point = tokens[2].clone();
                    let name = parse_option(&tokens, "--name");
                    handle_add_with_ctx(ctx, mount_point, name)?;
                    Ok(false)
                }
                "list" => {
                    let detailed = has_flag(&tokens, "-d") || has_flag(&tokens, "--detailed");
                    handle_list_with_ctx(ctx, detailed)?;
                    Ok(false)
                }
                "rename" => {
                    if tokens.len() < 4 {
                        print_warning(&t!("usage-disk-rename"));
                        return Ok(false);
                    }
                    let disk_id = DiskId::new(tokens[2].clone());
                    let new_name = tokens[3].clone();
                    disk_repo.update_disk_name(&disk_id, &new_name)?;
                    print_success(&crate::t!("disk-rename-success", "name" => new_name));
                    Ok(false)
                }
                "remove" => {
                    if tokens.len() < 3 {
                        print_warning(&t!("usage-disk-remove"));
                        return Ok(false);
                    }
                    let disk_id = DiskId::new(tokens[2].clone());
                    // Confirm deletion
                    print!("{} {}", "⚠".yellow(), crate::t!("confirm-remove-disk", "disk" => disk_id.as_str()));
                    io::stdout().flush().ok();
                    let mut input = String::new();
                    io::stdin().read_line(&mut input).ok();
                    if input.trim().to_lowercase().starts_with('y') {
                        disk_repo.delete_disk(&disk_id)?;
                        print_success(&t!("disk-removed-success"));
                    } else {
                        print_info(&t!("action-cancelled"));
                    }
                    Ok(false)
                }
                _ => {
                    print_warning(&crate::t!("unknown-disk-subcommand", "command" => tokens[1].clone()));
                    println!("  {}", format!("{}: {}", t!("available-disk-commands"), "add, list, rename, remove".bright_black()));
                    Ok(false)
                }
            }
        }
        "scan" => {
            let all = has_flag(&tokens, "--all") || has_flag(&tokens, "-a");
            let disk = parse_option(&tokens, "--disk").or_else(|| parse_option(&tokens, "-d"));
            let hash = has_flag(&tokens, "--hash") || has_flag(&tokens, "-h");
            let full = has_flag(&tokens, "--full") || has_flag(&tokens, "-f");
            handle_scan_with_ctx(ctx, all, disk, hash, full)?;
            Ok(false)
        }
        "search" => {
            if tokens.len() < 2 {
                print_warning(&t!("usage-search"));
                return Ok(false);
            }
            let keyword = tokens[1].clone();
            let ext = parse_option(&tokens, "--ext");
            let limit = parse_option(&tokens, "--limit")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(50);
            handle_search_with_ctx(ctx, keyword, None, None, ext, limit)?;
            Ok(false)
        }
        "get" => {
            if tokens.len() < 2 {
                print_warning(&t!("usage-get"));
                return Ok(false);
            }
            let entry_id = tokens[1].parse::<i64>()
                .map_err(|_| crate::DiscoError::InvalidPath(t!("get-invalid-id")))?;
            let locate = has_flag(&tokens, "--locate") || has_flag(&tokens, "-l");
            handle_get_with_ctx(ctx, entry_id, locate)?;
            Ok(false)
        }
        "store" => {
            if tokens.len() < 2 {
                print_warning(&t!("usage-store"));
                return Ok(false);
            }
            let paths: Vec<String> = tokens[1..].iter()
                .filter(|t| !t.starts_with("-"))
                .cloned()
                .collect();
            let solid_layer = parse_option(&tokens, "--solid-layer").unwrap_or_else(|| "0".to_string());
            let preview = has_flag(&tokens, "--preview") || has_flag(&tokens, "-p");
            let yes = has_flag(&tokens, "--yes") || has_flag(&tokens, "-y");
            handle_store_with_ctx(ctx, paths, solid_layer, false, preview, yes)?;
            Ok(false)
        }
        "retrieve" => {
            if tokens.len() < 2 {
                print_warning(&t!("usage-retrieve"));
                return Ok(false);
            }
            let keyword = tokens[1].clone();
            handle_retrieve_with_ctx(ctx, keyword)?;
            Ok(false)
        }
        "solid" => {
            if tokens.len() < 3 {
                print_warning(&t!("usage-solid"));
                return Ok(false);
            }
            match tokens[1].as_str() {
                "set" => {
                    let path = tokens[2].clone();
                    let disk = parse_option(&tokens, "--disk");
                    handle_set_with_ctx(ctx, path, disk)?;
                    Ok(false)
                }
                "unset" => {
                    let path = tokens[2].clone();
                    let disk = parse_option(&tokens, "--disk");
                    handle_unset_with_ctx(ctx, path, disk)?;
                    Ok(false)
                }
                _ => {
                    print_warning(&crate::t!("unknown-solid-subcommand", "command" => tokens[1].clone()));
                    Ok(false)
                }
            }
        }
        "visualize" | "viz" => {
            let disk = parse_option(&tokens, "--disk").or_else(|| parse_option(&tokens, "-d"));
            handle_visualize_with_ctx(ctx, disk)?;
            Ok(false)
        }
        _ => {
            print_warning(&crate::t!("unknown-command", "command" => tokens[0].clone()));
            print_info(&format!("{} {}", t!("shell-welcome-help"), t!("shell-welcome-menu")));
            Ok(false)
        }
    }
}

/// Check if a flag is present in tokens
fn has_flag(tokens: &[String], flag: &str) -> bool {
    tokens.iter().any(|t| t == flag)
}

/// Parse an option value from tokens (e.g., --name Foo)
fn parse_option(tokens: &[String], option: &str) -> Option<String> {
    for i in 0..tokens.len() - 1 {
        if tokens[i] == option {
            return Some(tokens[i + 1].clone());
        }
    }
    None
}

/// Show help message
fn show_help(command: Option<&str>) {
    match command {
        None => {
            println!();
            print_header(&t!("help-available-commands"));
            println!();
            println!("  {}  {}", "disk add <mount> [--name N]".bright_black(), t!("help-disk-add").white());
            println!("  {}  {}", "disk list [-d]".bright_black(), t!("help-disk-list").white());
            println!("  {}  {}", "disk rename <id> <name>".bright_black(), t!("help-disk-rename").white());
            println!("  {}  {}", "disk remove <id>".bright_black(), t!("help-disk-remove").white());
            println!();
            println!("  {}  {}", "scan [--all] [--disk D] [--hash]".bright_black(), t!("help-scan").white());
            println!("  {}  {}", "search <keyword> [--ext E]".bright_black(), t!("help-search").white());
            println!("  {}  {}", "get <id> [--locate]".bright_black(), t!("help-get").white());
            println!("  {}  {}", "store <paths...> [--solid-layer S]".bright_black(), t!("help-store").white());
            println!("  {}  {}", "retrieve <keyword>".bright_black(), t!("help-retrieve").white());
            println!();
            println!("  {}  {}", "solid set <path> [--disk D]".bright_black(), t!("help-solid-set").white());
            println!("  {}  {}", "solid unset <path>".bright_black(), t!("help-solid-unset").white());
            println!();
            println!("  {}  {}", "visualize [--disk D]".bright_black(), t!("help-visualize").white());
            println!("  {}  {}", "status".bright_black(), t!("help-status").white());
            println!("  {}  {}", "refresh".bright_black(), t!("help-refresh").white());
            println!("  {}  {}", "repair".bright_black(), t!("help-repair").white());
            println!("  {}  {}", "menu".bright_black(), t!("help-menu").white());
            println!();
            println!("  {}  {}", "help [command]".bright_black(), t!("help-detailed").white());
            println!("  {}  {}", "exit / quit".bright_black(), t!("help-exit").white());
            println!();
        }
        Some("disk") => {
            println!();
            print_header(&t!("help-disk-commands"));
            println!("  {}", "disk add <mount-point> [--name N]".cyan());
            println!("    {}", t!("help-disk-add-desc").white());
            println!();
            println!("  {}", "disk list [-d|--detailed]".cyan());
            println!("    {}", t!("help-disk-list-desc").white());
            println!();
            println!("  {}", "disk rename <disk-id> <new-name>".cyan());
            println!("    {}", t!("help-disk-rename-desc").white());
            println!();
            println!("  {}", "disk remove <disk-id>".cyan());
            println!("    {}", t!("help-disk-remove-desc").white());
            println!();
        }
        Some("scan") => {
            println!();
            print_header(&t!("help-scan-commands"));
            println!("  {}", "scan [--all] [--disk D] [--hash] [--full]".cyan());
            println!("    {}    {}", "--all".yellow(), t!("help-scan-all-desc").white());
            println!("    {}     {}", "--disk D".yellow(), t!("help-scan-disk-desc").white());
            println!("    {}       {}", "--hash".yellow(), t!("help-scan-hash-desc").white());
            println!("    {}       {}", "--full".yellow(), t!("help-scan-full-desc").white());
            println!();
        }
        Some("status") => {
            println!();
            print_header(&t!("help-status-commands"));
            println!("  {}", t!("help-status-desc").white());
            println!("    {}", format!("- {}", t!("help-status-detail1")).bright_black());
            println!("    {}", format!("- {}", t!("help-status-detail2")).bright_black());
            println!("    {}", format!("- {}", t!("help-status-detail3")).bright_black());
            println!();
        }
        Some("refresh") => {
            println!();
            print_header(&t!("help-refresh-commands"));
            println!("  {}", t!("help-refresh-desc").white());
            println!("  {}", t!("help-refresh-desc2").white());
            println!();
        }
        Some("repair") => {
            println!();
            print_header(&t!("help-repair-commands"));
            println!("  {}", t!("help-repair-desc").white());
            println!("  {}", t!("help-repair-desc2").white());
            println!("  {}", t!("help-repair-desc3").white());
            println!();
        }
        Some(cmd) => {
            print_warning(&crate::t!("help-no-detail", "command" => cmd.to_string()));
            print_info(&t!("help-general"));
        }
    }
}

/// Handle status command - show disk overview
fn handle_status(ctx: &AppContext) -> Result<()> {
    let disk_repo = ctx.disk_repo();
    let entry_repo = ctx.entry_repo();
    let detector = get_detector();
    let disks = disk_repo.list_disks()?;
    let mount_points = detector.list_mount_points()?;

    if disks.is_empty() {
        print_warning(&t!("no-disks-registered"));
        return Ok(());
    }

    println!();
    print_header(&t!("status-title"));
    print_separator();

    let mut total_files = 0usize;
    let mut online_count = 0;
    let mut offline_count = 0;

    for disk in &disks {
        // Check mount status
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

        let status = if is_mounted {
            online_count += 1;
            crate::domain::disk::MountStatus::Connected
        } else {
            offline_count += 1;
            crate::domain::disk::MountStatus::Offline
        };

        // Get file count
        let entries = entry_repo.get_entries_by_disk(&disk.disk_id)?;
        let file_count = entries.iter().filter(|e| e.entry_type == crate::domain::entry::EntryType::File).count();
        total_files += file_count;

        println!("  {} {}", format_disk_name(&disk.name), format_disk_id(disk.disk_id.as_str()));
        println!("    {}: {}", t!("label-status"), format_mount_status_colored(status));
        println!("    {}: {}", t!("label-capacity"), format_size(disk.identity.capacity_bytes).cyan());
        println!("    {}: {}", t!("label-files"), crate::t!("count-files", "count" => file_count.to_string()));
        if let Some(mount) = current_mount {
            println!("    {}: {}", t!("disk-mount-point"), crate::t!("status-mounted", "path" => mount.clone()).green());
        }
        println!();
    }

    print_separator();
    println!("{}: {}, {}, {}",
        t!("status-summary").white().bold(),
        crate::t!("count-disks", "count" => disks.len().to_string()),
        crate::t!("status-online-count", "count" => online_count.to_string()).green(),
        crate::t!("status-offline-count", "count" => offline_count.to_string()).red()
    );
    println!("{}: {}", t!("status-total-files").white().bold(), crate::t!("count-files", "count" => total_files.to_string()).cyan());
    println!();

    Ok(())
}

/// Handle refresh command - force refresh mount status with diagnostics
fn handle_refresh(ctx: &AppContext) -> Result<()> {
    let disk_repo = ctx.disk_repo();
    let detector = crate::storage::platform::get_detector();
    let mount_checker = crate::storage::mount::MountChecker::new(
        &disk_repo,
        &detector
    );

    println!();
    print_header(&t!("refresh-title"));
    println!();

    let report = mount_checker.force_refresh()?;

    print_separator();
    print_header(&t!("refresh-mount-title"));
    for detail in &report.mount_points {
        println!("  {} ({}, {})",
            detail.mount_point.green(),
            detail.identity.volume_label.as_deref().unwrap_or("unknown").white(),
            format_size(detail.identity.capacity_bytes).cyan()
        );
    }
    println!();

    print_separator();
    print_header(&t!("refresh-disk-title"));
    for disk_report in &report.disk_reports {
        println!("  {} {}", format_disk_name(&disk_report.name), format_disk_id(disk_report.disk_id.as_str()));
        println!("    {}: {}", t!("label-status"), format_mount_status_colored(disk_report.status));

        if let Some(mount) = &disk_report.mount_point {
            println!("    {}: {}", t!("disk-mount-point"), mount.green());
        }

        // Show diagnostic for offline disks
        if disk_report.status == crate::domain::disk::MountStatus::Offline {
            println!("    {}: {}", t!("refresh-diagnostic").yellow(), t!("refresh-no-match").yellow());
            if !disk_report.potential_matches.is_empty() {
                println!("    {}:", t!("refresh-potential").bright_black());
                for match_detail in &disk_report.potential_matches {
                    println!("      {} - {}", match_detail.mount_point.white(), match_detail.match_result.reason().yellow());
                }
            }
        }
        println!();
    }

    Ok(())
}

/// Handle repair command - fix offline disk identities
fn handle_repair(ctx: &AppContext) -> Result<()> {
    let disk_repo = ctx.disk_repo();
    let detector = get_detector();
    let disks = disk_repo.list_disks()?;
    let mount_points = detector.list_mount_points()?;

    // Find offline disks
    let offline_disks: Vec<_> = disks.iter()
        .filter(|disk| {
            for mount in &mount_points {
                if let Ok(identity) = detector.detect_identity(mount) {
                    if disk.identity.matches(&identity) {
                        return false;
                    }
                }
            }
            true
        })
        .collect();

    if offline_disks.is_empty() {
        print_success(&t!("repair-all-online"));
        return Ok(());
    }

    println!();
    print_warning(&crate::t!("repair-found-offline", "count" => offline_disks.len().to_string()));
    println!();

    for disk in offline_disks {
        println!("{} \"{}\" [{}] {}",
            t!("repair-disk-label").white(),
            disk.name.cyan(),
            disk.disk_id.as_str().bright_black(),
            t!("status-offline").red().bold()
        );
        println!("  {}: {:?}", t!("repair-volume-label").bright_black(), disk.identity.volume_label);
        println!("  {}: {}", t!("label-capacity").bright_black(), format_size(disk.identity.capacity_bytes).cyan());

        // Try to find matching mount point by label
        let mut candidates: Vec<(String, crate::domain::disk::DiskIdentity)> = Vec::new();

        for mount in &mount_points {
            if let Ok(identity) = detector.detect_identity(mount) {
                // Check if label matches
                if disk.identity.volume_label.is_some()
                    && identity.volume_label == disk.identity.volume_label {
                    candidates.push((mount.clone(), identity));
                }
            }
        }

        if candidates.is_empty() {
            print_warning(&format!("  {}", t!("repair-no-candidates")));
            println!("  {} {}", "[1]".bright_black(), t!("repair-skip"));
            println!("  {} {}", "[2]".red(), t!("repair-remove"));
            println!();
            print!("{} ", t!("repair-select").yellow());
            io::stdout().flush().ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            match input.trim() {
                "2" => {
                    disk_repo.delete_disk(&disk.disk_id)?;
                    print_success(&format!("  {}", t!("repair-removed")));
                }
                _ => {
                    print_info(&format!("  {}", t!("repair-skipped")));
                }
            }
        } else {
            print_info(&crate::t!("repair-candidates", "count" => candidates.len().to_string()));
            for (i, (mount, identity)) in candidates.iter().enumerate() {
                println!("    {} {} (label: {:?})",
                    format!("[{}]", i + 1).bright_black(),
                    mount.green(),
                    identity.volume_label
                );
            }
            println!();
            println!("  {} {}", "[R]".green(), t!("repair-reconnect"));
            println!("  {} {}", "[S]".bright_black(), t!("repair-skip-disk"));
            println!("  {} {}", "[D]".red(), t!("repair-delete"));
            println!();
            print!("{} ", t!("repair-select").yellow());
            io::stdout().flush().ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();

            match input.trim().to_uppercase().as_str() {
                "R" => {
                    // Use first candidate
                    if let Some((mount, new_identity)) = candidates.first() {
                        disk_repo.update_disk_identity(&disk.disk_id, new_identity)?;
                        disk_repo.update_last_mount_point(&disk.disk_id, mount.clone())?;
                        print_success(&format!("  {}", t!("repair-identity-updated")));
                        println!("  {}: {}", t!("repair-new-mount").bright_black(), mount.green());
                    }
                }
                "D" => {
                    disk_repo.delete_disk(&disk.disk_id)?;
                    print_success(&format!("  {}", t!("repair-removed")));
                }
                _ => {
                    print_info(&format!("  {}", t!("repair-skipped")));
                }
            }
        }
        println!();
    }

    print_success(&t!("repair-complete"));
    Ok(())
}

/// Run menu mode with arrow key navigation
fn run_menu_mode(ctx: &AppContext) -> Result<()> {
    use crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
        cursor::{MoveTo, Hide, Show},
        style::{Print, ResetColor, SetForegroundColor, Color as CColor},
    };
    use std::io;

    // Menu items: (number_key, label_key, desc_key)
    let menu_items: Vec<(&str, &str, &str)> = vec![
        ("1", "menu-disk-management", "menu-desc-disk"),
        ("2", "menu-scan-files", "menu-desc-scan"),
        ("3", "menu-search-files", "menu-desc-search"),
        ("4", "menu-store-files", "menu-desc-store"),
        ("5", "menu-retrieve-files", "menu-desc-retrieve"),
        ("6", "menu-view-status", "menu-desc-status"),
        ("7", "menu-refresh-status", "menu-desc-refresh"),
        ("8", "menu-repair-offline", "menu-desc-repair"),
        ("9", "menu-visualize", "menu-desc-visualize"),
        ("0", "menu-settings", "menu-desc-settings"),
        ("q", "menu-exit", "menu-desc-exit"),
    ];

    let mut selected = 0;
    let mut needs_redraw = true;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    loop {
        // Only redraw when needed
        if needs_redraw {
            // Clear screen and draw menu
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

            // Header with colorful border and DISCO ASCII art
            execute!(
                stdout,
                SetForegroundColor(CColor::Magenta),
                Print("\r\n"),
                Print("  ╔══════════════════════════════════════════════╗\r\n"),
                SetForegroundColor(CColor::Cyan),
                Print("  ║                                              ║\r\n"),
                Print("  ║    "), SetForegroundColor(CColor::Yellow),
                Print("██████╗ ██╗███████╗ ██████╗ ████████╗"), SetForegroundColor(CColor::Cyan),
                Print("     ║\r\n"),
                Print("  ║    "), SetForegroundColor(CColor::Yellow),
                Print("██╔══██╗██║██╔════╝██╔════╝ ██╔═══██║"), SetForegroundColor(CColor::Cyan),
                Print("     ║\r\n"),
                Print("  ║    "), SetForegroundColor(CColor::Yellow),
                Print("██║  ██║██║███████╗██║      ██║   ██║"), SetForegroundColor(CColor::Cyan),
                Print("     ║\r\n"),
                Print("  ║    "), SetForegroundColor(CColor::Yellow),
                Print("██║  ██║██║╚════██║██║      ██║   ██║"), SetForegroundColor(CColor::Cyan),
                Print("     ║\r\n"),
                Print("  ║    "), SetForegroundColor(CColor::Yellow),
                Print("██████╔╝██║███████║╚██████╗ ████████║"), SetForegroundColor(CColor::Cyan),
                Print("     ║\r\n"),
                Print("  ║    "), SetForegroundColor(CColor::Yellow),
                Print("╚═════╝ ╚═╝╚══════╝ ╚═════╝ ╚═══════╝"), SetForegroundColor(CColor::Cyan),
                Print("     ║\r\n"),
                Print("  ║                                              ║\r\n"),
                SetForegroundColor(CColor::Magenta),
                Print("  ╚══════════════════════════════════════════════╝\r\n"),
                ResetColor,
                Print("\r\n")
            )?;

            // Menu items with numbers and arrows
            for (i, (key, label_key, desc_key)) in menu_items.iter().enumerate() {
                let arrow = if i == selected { "  ▶ " } else { "    " };
                let key_color = if i == selected { CColor::Green } else { CColor::Yellow };
                let label_color = if i == selected { CColor::White } else { CColor::Grey };
                let desc_color = CColor::DarkGrey;

                execute!(
                    stdout,
                    Print(arrow),
                    SetForegroundColor(key_color),
                    Print(format!("[{}]", key)),
                    ResetColor,
                    Print(" "),
                    SetForegroundColor(label_color),
                    Print(t!(label_key)),
                    ResetColor,
                    SetForegroundColor(desc_color),
                    Print(format!(" - {}", t!(desc_key))),
                    ResetColor,
                    Print("\r\n")
                )?;
            }

            // Footer
            execute!(
                stdout,
                Print("\r\n"),
                SetForegroundColor(CColor::DarkGrey),
                Print("  ══════════════════════════════════════════════════════════\r\n"),
                ResetColor,
                Print("  "),
                SetForegroundColor(CColor::Green),
                Print("↑/↓"),
                ResetColor,
                Print(" Navigate  │  "),
                SetForegroundColor(CColor::Green),
                Print("Enter"),
                ResetColor,
                Print(" Select  │  "),
                SetForegroundColor(CColor::Green),
                Print("1-9,0,q"),
                ResetColor,
                Print(" Quick  │  "),
                SetForegroundColor(CColor::Green),
                Print("Esc"),
                ResetColor,
                Print(" Exit\r\n")
            )?;

            needs_redraw = false;
        }

        // Handle input - arrow keys, enter, and quick select
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Down => {
                        if selected < menu_items.len() - 1 {
                            selected += 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Enter => {
                        // Check if exit
                        if selected == menu_items.len() - 1 {
                            break;
                        }
                        // Execute selected action
                        disable_raw_mode()?;
                        execute!(stdout, LeaveAlternateScreen, Show)?;
                        // Handle errors gracefully
                        if let Err(e) = execute_menu_action(ctx, selected) {
                            println!("\n{} {}", "Error:".red(), e);
                            println!("{} ", "Press Enter to continue...".cyan());
                            let mut _input = String::new();
                            io::stdin().read_line(&mut _input).ok();
                        }
                        // Return to menu
                        enable_raw_mode()?;
                        execute!(stdout, EnterAlternateScreen, Hide)?;
                        needs_redraw = true;
                    }
                    KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Char(c) => {
                        // Quick select by number or 'q'
                        if let Some(idx) = menu_items.iter().position(|(k, _, _)| *k == c.to_string()) {
                            if idx == menu_items.len() - 1 {
                                break; // Exit
                            }
                            // Execute selected action
                            disable_raw_mode()?;
                            execute!(stdout, LeaveAlternateScreen, Show)?;
                            // Handle errors gracefully
                            if let Err(e) = execute_menu_action(ctx, idx) {
                                println!("\n{} {}", "Error:".red(), e);
                                println!("{} ", "Press Enter to continue...".cyan());
                                let mut _input = String::new();
                                io::stdin().read_line(&mut _input).ok();
                            }
                            // Return to menu
                            enable_raw_mode()?;
                            execute!(stdout, EnterAlternateScreen, Hide)?;
                            needs_redraw = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, Show)?;

    print_info(&t!("menu-returned"));
    Ok(())
}

/// Execute menu action
fn execute_menu_action(ctx: &AppContext, index: usize) -> Result<()> {
    match index {
        0 => { // Disk management - with arrow navigation
            run_submenu("磁盘管理 Disk Management", &[
                ("添加硬盘", "Add a new disk to the pool"),
                ("列出硬盘", "List all registered disks"),
            ], &[
                Box::new(|ctx| {
                    print!("{} ", "输入挂载点 Enter mount point:".cyan());
                    io::stdout().flush().ok();
                    let mut mount = String::new();
                    io::stdin().read_line(&mut mount).ok();
                    let mount = mount.trim();
                    if !mount.is_empty() {
                        handle_add_with_ctx(ctx, mount.to_string(), None)?;
                    }
                    Ok(())
                }),
                Box::new(|ctx| {
                    handle_list_with_ctx(ctx, false)?;
                    wait_for_enter_or_esc()
                }),
            ], ctx)?;
        }
        1 => { // Scan files - with arrow navigation and disk picker
            run_submenu("扫描文件 Scan Files", &[
                ("扫描所有硬盘", "Scan all registered disks"),
                ("扫描指定硬盘", "Select disk with arrow keys"),
            ], &[
                Box::new(|ctx| {
                    handle_scan_with_ctx(ctx, true, None, is_hash_enabled(ctx), false)?;
                    wait_for_enter_or_esc()
                }),
                Box::new(|ctx| {
                    // Use disk picker instead of text input
                    if let Some(disk_id) = run_disk_picker(ctx)? {
                        handle_scan_with_ctx(ctx, false, Some(disk_id), is_hash_enabled(ctx), false)?;
                        wait_for_enter_or_esc()?;
                    }
                    Ok(())
                }),
            ], ctx)?;
        }
        2 => { // Search files
            println!();
            print_header("搜索文件 Search Files");
            println!();
            print!("{} ", "输入关键词 Enter keyword:".cyan());
            io::stdout().flush().ok();
            let mut keyword = String::new();
            io::stdin().read_line(&mut keyword).ok();
            if !keyword.trim().is_empty() {
                handle_search_with_ctx(ctx, keyword.trim().to_string(), None, None, None, 50)?;
                wait_for_enter_or_esc()?;
            }
        }
        3 => { // Store files
            println!();
            print_header("存储文件 Store Files");
            let hash_enabled = is_hash_enabled(ctx);
            if hash_enabled {
                println!("  {} 哈希校验已开启", "✓".green());
            } else {
                println!("  {} 哈希校验已关闭", "○".yellow());
            }
            println!();
            print!("{} ", "输入文件路径（多个用逗号分隔）Enter paths (comma separated):".cyan());
            io::stdout().flush().ok();
            let mut paths = String::new();
            io::stdin().read_line(&mut paths).ok();
            let paths: Vec<String> = paths.trim().split(',').map(|s| s.trim().to_string()).collect();

            if !paths.is_empty() {
                println!();
                print!("{} ", "输入SolidLayer深度（默认0不分割）Enter solid layer depth [0]:".cyan());
                io::stdout().flush().ok();
                let mut solid_layer = String::new();
                io::stdin().read_line(&mut solid_layer).ok();
                let solid_layer = if solid_layer.trim().is_empty() { "0".to_string() } else { solid_layer.trim().to_string() };

                // Execute real storage (not preview)
                handle_store_with_ctx(ctx, paths, solid_layer, hash_enabled, false, false)?;
                wait_for_enter_or_esc()?;
            }
        }
        4 => { // Retrieve files
            println!();
            print_header("检索文件 Retrieve Files");
            println!();
            print!("{} ", "输入搜索关键词 Enter search keyword:".cyan());
            io::stdout().flush().ok();
            let mut keyword = String::new();
            io::stdin().read_line(&mut keyword).ok();
            if !keyword.trim().is_empty() {
                crate::cli::commands::retrieve::handle_retrieve_with_ctx(ctx, keyword.trim().to_string())?;
                wait_for_enter_or_esc()?;
            }
        }
        5 => { // View status - add pause
            handle_status(ctx)?;
            wait_for_enter_or_esc()?;
        }
        6 => { // Refresh status - add pause
            handle_refresh(ctx)?;
            wait_for_enter_or_esc()?;
        }
        7 => { // Repair offline - add pause
            handle_repair(ctx)?;
            wait_for_enter_or_esc()?;
        }
        8 => { // Visualize - no pause needed, it's a full TUI
            handle_visualize_with_ctx(ctx, None)?;
        }
        9 => { // Settings
            run_settings_menu(ctx)?;
        }
        _ => {}
    }

    Ok(())
}

/// Wait for Enter or Esc key before continuing (for display actions)
fn wait_for_enter_or_esc() -> Result<()> {
    println!();
    println!("  {}", "─────────────────────────────────────────────".bright_black());
    print!("  {} ", "按 Enter 或 Esc 继续 Press Enter or Esc to continue...".cyan());
    io::stdout().flush().ok();

    // Use crossterm to detect Enter or Esc
    use crossterm::{
        event::{self, Event, KeyCode},
        terminal::{enable_raw_mode, disable_raw_mode},
    };

    enable_raw_mode()?;
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter | KeyCode::Esc => break,
                    _ => {}
                }
            }
        }
    }
    disable_raw_mode()?;
    println!(); // Add newline for cleaner transition back to menu

    Ok(())
}

/// Run a submenu with arrow navigation
fn run_submenu(title: &str, items: &[(&str, &str)], actions: &[Box<dyn Fn(&AppContext) -> Result<()>>], ctx: &AppContext) -> Result<()> {
    use crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
        cursor::{MoveTo, Hide, Show},
        style::{Print, ResetColor, SetForegroundColor, Color as CColor},
    };

    let mut selected = 0;
    let mut needs_redraw = true;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    loop {
        if needs_redraw {
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

            // Header - use execute! for proper alignment
            execute!(
                stdout,
                Print("\r\n"),
                SetForegroundColor(CColor::Cyan),
                Print(format!("  {} ", title)),
                ResetColor,
                Print("\r\n"),
                Print("  ─────────────────────────────────────────────\r\n"),
                Print("\r\n")
            )?;

            // Items - aligned with fixed-width label column
            for (i, (label, desc)) in items.iter().enumerate() {
                let arrow = if i == selected { "▶ " } else { "  " };
                let label_color = if i == selected { CColor::Yellow } else { CColor::White };
                let desc_color = CColor::DarkGrey;

                execute!(
                    stdout,
                    Print(format!("  {} ", arrow)),
                    SetForegroundColor(label_color),
                    Print(format!("{:<12}", label)),  // Fixed 12-char width for alignment
                    ResetColor,
                    SetForegroundColor(desc_color),
                    Print(format!("  {}", desc)),
                    ResetColor,
                    Print("\r\n")
                )?;
            }

            // Back option
            execute!(stdout, Print("\r\n"))?;
            let back_arrow = if selected == items.len() { "▶ " } else { "  " };
            let back_style = if selected == items.len() { CColor::Yellow } else { CColor::Grey };
            execute!(
                stdout,
                Print(format!("  {} ", back_arrow)),
                SetForegroundColor(back_style),
                Print("返回 Back"),
                ResetColor,
                Print("\r\n")
            )?;

            // Footer
            execute!(
                stdout,
                Print("\r\n"),
                SetForegroundColor(CColor::DarkGrey),
                Print("  ↑/↓ Navigate │ Enter Select │ Esc Back\r\n"),
                ResetColor
            )?;

            needs_redraw = false;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Down => {
                        if selected < items.len() {
                            selected += 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Enter => {
                        if selected < items.len() {
                            disable_raw_mode()?;
                            execute!(stdout, LeaveAlternateScreen, Show)?;
                            // Run action and handle any errors gracefully
                            if let Err(e) = actions[selected](ctx) {
                                println!("\n{} {}", "Error:".red(), e);
                                println!("{} ", "Press Enter to continue...".cyan());
                                let mut _input = String::new();
                                io::stdin().read_line(&mut _input).ok();
                            }
                            enable_raw_mode()?;
                            execute!(stdout, EnterAlternateScreen, Hide)?;
                            needs_redraw = true;
                        } else {
                            break;
                        }
                    }
                    KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, Show)?;

    Ok(())
}

/// Run disk picker for arrow key selection
/// Returns the selected disk_id or None if cancelled
fn run_disk_picker(ctx: &AppContext) -> Result<Option<String>> {
    use crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
        cursor::{MoveTo, Hide, Show},
        style::{Print, ResetColor, SetForegroundColor, Color as CColor},
    };

    let disk_repo = ctx.disk_repo();
    let detector = crate::storage::platform::get_detector();
    let disks = disk_repo.list_disks()?;
    let mount_points = detector.list_mount_points()?;

    if disks.is_empty() {
        println!("No disks registered.");
        return Ok(None);
    }

    // Build disk info with mount status
    let disk_info: Vec<(crate::domain::disk::Disk, bool, Option<String>)> = disks.iter().map(|disk| {
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
        (disk.clone(), is_mounted, current_mount)
    }).collect();

    let mut selected = 0;
    let mut needs_redraw = true;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let result: Option<String> = loop {
        if needs_redraw {
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

            // Header
            execute!(
                stdout,
                Print("\r\n"),
                SetForegroundColor(CColor::Cyan),
                Print("  选择硬盘 Select Disk"),
                ResetColor,
                Print("\r\n"),
                Print("  ─────────────────────────────────────────────\r\n"),
                Print("\r\n")
            )?;

            // Disk list
            for (i, (disk, mounted, mount)) in disk_info.iter().enumerate() {
                let arrow = if i == selected { "▶ " } else { "  " };
                let status_icon = if *mounted { "●" } else { "○" };
                let status_color = if *mounted { CColor::Green } else { CColor::Red };
                let name_color = if i == selected { CColor::Yellow } else { CColor::White };

                execute!(
                    stdout,
                    Print(format!("  {} ", arrow)),
                    SetForegroundColor(status_color),
                    Print(status_icon),
                    ResetColor,
                    Print(" "),
                    SetForegroundColor(name_color),
                    Print(&disk.name),
                    ResetColor,
                    SetForegroundColor(CColor::DarkGrey),
                    Print(format!(" [{}]", disk.disk_id.as_str())),
                    ResetColor
                )?;

                if let Some(m) = mount {
                    execute!(
                        stdout,
                        SetForegroundColor(CColor::Green),
                        Print(format!(" ({})", m)),
                        ResetColor
                    )?;
                }

                execute!(stdout, Print("\r\n"))?;
            }

            // Back option
            execute!(stdout, Print("\r\n"))?;
            let back_arrow = if selected == disk_info.len() { "▶ " } else { "  " };
            let back_style = if selected == disk_info.len() { CColor::Yellow } else { CColor::Grey };
            execute!(
                stdout,
                Print(format!("  {} ", back_arrow)),
                SetForegroundColor(back_style),
                Print("返回 Back"),
                ResetColor,
                Print("\r\n")
            )?;

            // Footer
            execute!(
                stdout,
                Print("\r\n"),
                SetForegroundColor(CColor::DarkGrey),
                Print("  ↑/↓ Navigate │ Enter Select │ Esc Back\r\n"),
                ResetColor
            )?;

            needs_redraw = false;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Down => {
                        if selected < disk_info.len() {
                            selected += 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Enter => {
                        if selected < disk_info.len() {
                            break Some(disk_info[selected].0.disk_id.as_str().to_string());
                        } else {
                            break None;
                        }
                    }
                    KeyCode::Esc => {
                        break None;
                    }
                    _ => {}
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, Show)?;

    Ok(result)
}

/// Check if hash verification is enabled
fn is_hash_enabled(ctx: &AppContext) -> bool {
    let config = ctx.config();
    let db = ctx.db();
    config.get_value("hash_enabled", db)
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true) // Default to enabled
}

/// Set hash verification enabled
fn set_hash_enabled(ctx: &AppContext, enabled: bool) -> Result<()> {
    let config = ctx.config();
    let db = ctx.db();
    config.set_value("hash_enabled", if enabled { "true" } else { "false" }, db)
}

/// Run settings menu
fn run_settings_menu(ctx: &AppContext) -> Result<()> {
    use crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
        cursor::{MoveTo, Hide, Show},
        style::{Print, ResetColor, SetForegroundColor, Color as CColor},
    };

    let mut selected = 0;
    let mut hash_enabled = is_hash_enabled(ctx);
    let current_lang = crate::i18n::current_language();
    let mut lang_index = crate::i18n::SUPPORTED_LANGUAGES.iter()
        .position(|(code, _)| *code == current_lang)
        .unwrap_or(0);
    let mut needs_redraw = true;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    loop {
        if needs_redraw {
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

            // Header
            execute!(
                stdout,
                Print("\r\n"),
                SetForegroundColor(CColor::Cyan),
                Print(format!("  {}", t!("submenu-settings-title"))),
                ResetColor,
                Print("\r\n"),
                Print("  ─────────────────────────────────────────────\r\n"),
                Print("\r\n")
            )?;

            // Language option (index 0)
            let (_, lang_name) = crate::i18n::SUPPORTED_LANGUAGES[lang_index];
            let arrow = if selected == 0 { "▶ " } else { "  " };
            execute!(
                stdout,
                Print(format!("  {} ", arrow)),
                Print(format!("{}: ", t!("submenu-settings-lang"))),
                SetForegroundColor(CColor::Green),
                Print(lang_name),
                ResetColor,
                Print("\r\n")
            )?;
            execute!(
                stdout,
                Print("      "),
                SetForegroundColor(CColor::DarkGrey),
                Print(t!("submenu-settings-lang-desc")),
                ResetColor,
                Print("\r\n")
            )?;
            execute!(stdout, Print("\r\n"))?;

            // Hash verification option (index 1)
            let hash_status = if hash_enabled { t!("submenu-settings-hash-on") } else { t!("submenu-settings-hash-off") };
            let hash_color = if hash_enabled { CColor::Green } else { CColor::Red };
            let arrow = if selected == 1 { "▶ " } else { "  " };

            execute!(
                stdout,
                Print(format!("  {} ", arrow)),
                Print(format!("{}: ", t!("submenu-settings-hash"))),
                SetForegroundColor(hash_color),
                Print(hash_status),
                ResetColor,
                Print("\r\n")
            )?;
            execute!(
                stdout,
                Print("      "),
                SetForegroundColor(CColor::DarkGrey),
                Print(t!("submenu-settings-hash-desc")),
                ResetColor,
                Print("\r\n")
            )?;

            // Back option (index 2)
            execute!(stdout, Print("\r\n"))?;
            let back_arrow = if selected == 2 { "▶ " } else { "  " };
            let back_style = if selected == 2 { CColor::Yellow } else { CColor::Grey };
            execute!(
                stdout,
                Print(format!("  {} ", back_arrow)),
                SetForegroundColor(back_style),
                Print(t!("menu-back")),
                ResetColor,
                Print("\r\n")
            )?;

            // Footer
            execute!(
                stdout,
                Print("\r\n"),
                SetForegroundColor(CColor::DarkGrey),
                Print("  ↑/↓ Navigate │ Enter/L/R Toggle │ Esc Back\r\n"),
                ResetColor
            )?;

            needs_redraw = false;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Down => {
                        if selected < 2 {
                            selected += 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Left | KeyCode::Right => {
                        if selected == 0 {
                            // Cycle language
                            if key.code == KeyCode::Right {
                                lang_index = (lang_index + 1) % crate::i18n::SUPPORTED_LANGUAGES.len();
                            } else {
                                lang_index = if lang_index == 0 { crate::i18n::SUPPORTED_LANGUAGES.len() - 1 } else { lang_index - 1 };
                            }
                            let (new_code, _) = crate::i18n::SUPPORTED_LANGUAGES[lang_index];
                            let _ = crate::i18n::set_language(new_code);
                            // Save to config
                            let config = ctx.config();
                            let db = ctx.db();
                            let _ = config.set_value("language", new_code, db);
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Enter => {
                        if selected == 0 {
                            // Cycle language
                            lang_index = (lang_index + 1) % crate::i18n::SUPPORTED_LANGUAGES.len();
                            let (new_code, _) = crate::i18n::SUPPORTED_LANGUAGES[lang_index];
                            let _ = crate::i18n::set_language(new_code);
                            // Save to config
                            let config = ctx.config();
                            let db = ctx.db();
                            let _ = config.set_value("language", new_code, db);
                            needs_redraw = true;
                        } else if selected == 1 {
                            hash_enabled = !hash_enabled;
                            if let Err(e) = set_hash_enabled(ctx, hash_enabled) {
                                eprintln!("Failed to save setting: {}", e);
                            }
                            needs_redraw = true;
                        } else {
                            break;
                        }
                    }
                    KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, Show)?;

    Ok(())
}
