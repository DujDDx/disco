//! Display utilities for terminal output

use indicatif::{ProgressBar, ProgressStyle};
use colored::{ColoredString, Colorize};

/// Create a progress bar for scanning
pub fn create_scan_progress(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
        .expect("Valid template")
        .progress_chars("#>-");
    pb.set_style(style);
    pb
}

/// Create a progress bar for copying
pub fn create_copy_progress(total_bytes: u64) -> ProgressBar {
    let pb = ProgressBar::new(total_bytes);
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .expect("Valid template")
        .progress_chars("#>-");
    pb.set_style(style);
    pb
}

/// Format file size for display
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size >= TB {
        format!("{:.2} TB", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// Format file size with color (cyan for emphasis)
pub fn format_size_colored(size: u64) -> ColoredString {
    format_size(size).cyan()
}

/// Format mount status for display
pub fn format_mount_status(mounted: bool) -> &'static str {
    if mounted {
        "✓ Connected"
    } else {
        "✗ Offline"
    }
}

/// Format mount status with color
pub fn format_mount_status_colored(status: crate::domain::disk::MountStatus) -> ColoredString {
    match status {
        crate::domain::disk::MountStatus::Connected => "✓ Connected".green().bold(),
        crate::domain::disk::MountStatus::Offline => "✗ Offline".red(),
        crate::domain::disk::MountStatus::IdentityConflict => "⚠ Identity Conflict".yellow().bold(),
    }
}

/// Print styled success message
pub fn print_success(message: &str) {
    println!("{}", format!("✓ {}", message).green().bold());
}

/// Print styled error message
pub fn print_error(message: &str) {
    println!("{}", format!("✗ {}", message).red().bold());
}

/// Print styled warning message
pub fn print_warning(message: &str) {
    println!("{}", format!("⚠ {}", message).yellow());
}

/// Print styled info message
pub fn print_info(message: &str) {
    println!("{}", format!("ℹ {}", message).cyan());
}

/// Print styled header/title
pub fn print_header(message: &str) {
    println!("{}", message.cyan().bold());
}

/// Print styled section title
pub fn print_section(message: &str) {
    println!();
    println!("{}", format!("── {} ──", message).white().bold());
}

/// Print styled disk name
pub fn format_disk_name(name: &str) -> ColoredString {
    name.white().bold()
}

/// Print styled disk ID
pub fn format_disk_id(id: &str) -> ColoredString {
    format!("[{}]", id).bright_black()
}

/// Print styled command prompt
pub fn format_prompt() -> ColoredString {
    "disco>".green().bold()
}

/// Print styled help text
pub fn print_help_item(command: &str, description: &str) {
    println!("  {}  {}", command.bright_black(), description.white());
}

/// Print styled separator line
pub fn print_separator() {
    println!("{}", "─".repeat(80).bright_black());
}

/// Print styled disk list item
pub fn print_disk_item(name: &str, id: &str, status: crate::domain::disk::MountStatus, capacity: u64, file_count: usize) {
    let status_str = format_mount_status_colored(status);
    println!("  {} {}", format_disk_name(name), format_disk_id(id));
    println!("    Status: {}", status_str);
    println!("    Capacity: {}", format_size_colored(capacity));
    println!("    Files: {}", format!("{}", file_count).cyan());
}

/// Print styled table header
pub fn print_table_header(columns: &[&str]) {
    let header = columns.join("  ");
    println!("{}", header.white().bold().underline());
}

/// Print styled table row
pub fn print_table_row(columns: &[ColoredString]) {
    let row: Vec<String> = columns.iter().map(|c| c.to_string()).collect();
    println!("{}", row.join("  "));
}

/// Clear screen and reset cursor
pub fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}

/// Hide cursor
pub fn hide_cursor() {
    print!("\x1B[?25l");
}

/// Show cursor
pub fn show_cursor() {
    print!("\x1B[?25h");
}

/// Move cursor to position
pub fn move_cursor(row: u16, col: u16) {
    print!("\x1B[{};{}H", row, col);
}

/// Print error with suggestion
pub fn print_error_with_suggestion(error: &str, suggestion: &str) {
    println!("{}", format!("✗ {}", error).red().bold());
    println!("  {}", format!("Suggestion: {}", suggestion).yellow());
}

/// Print styled disk error with user-friendly message
pub fn print_disk_error(error: &crate::DiscoError) {
    let severity = error.severity();
    let user_msg = error.user_description();

    match severity {
        crate::ErrorSeverity::Warning => {
            println!("{}", format!("⚠ {}", user_msg).yellow());
        }
        crate::ErrorSeverity::Error => {
            println!("{}", format!("✗ {}", user_msg).red().bold());
        }
        crate::ErrorSeverity::Critical => {
            println!("{}", format!("✗✗ CRITICAL: {}", user_msg).red().bold().on_black());
        }
    }

    if let Some(suggestion) = error.suggestion() {
        println!("  {}", format!("💡 {}", suggestion).cyan());
    }
}

/// Print styled menu item
pub fn print_menu_item(key: &str, label: &str, selected: bool) {
    if selected {
        println!("  {} {} {}", "▶".green().bold(), format!("[{}]", key).green().bold(), label.white().bold());
    } else {
        println!("    {} {}", format!("[{}]", key).bright_black(), label.white());
    }
}