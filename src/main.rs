//! Disco - Local multi-disk storage scheduling CLI tool
//!
//! Organizes multiple independent disks into a "disk pool" with offline index search,
//! smart storage scheduling, Solid/SolidLayer rules, and terminal visualization.

use clap::{Parser, Subcommand};
use disco::cli::commands::{
    disk::{DiskAddCmd, DiskListCmd},
    get::GetCmd,
    retrieve::RetrieveCmd,
    scan::ScanCmd,
    search::SearchCmd,
    solid::SolidCmd,
    store::StoreCmd,
    visualize::VisualizeCmd,
};
use tracing_subscriber::EnvFilter;

/// Disco - Multi-disk storage management tool
#[derive(Parser, Debug)]
#[command(name = "disco", author, version, about, long_about = None)]
struct Cli {
    /// Increase logging verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Start interactive shell mode
    #[arg(short, long)]
    interactive: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Manage disk pool entries
    #[command(subcommand)]
    Disk(DiskCommands),

    /// Scan disks and build/update the file index
    Scan(ScanCmd),

    /// Search for files in the index
    Search(SearchCmd),

    /// Get a file location by entry ID
    Get(GetCmd),

    /// Store files/folders into the disk pool
    Store(StoreCmd),

    /// Retrieve files from the disk pool
    Retrieve(RetrieveCmd),

    /// Manage Solid markers on directories
    #[command(subcommand)]
    Solid(SolidCommands),

    /// Open the terminal visualization interface
    Visualize(VisualizeCmd),

    /// Open visual menu mode with arrow key navigation
    Menu,

    /// Configure Disco settings
    #[command(subcommand)]
    Config(ConfigCmd),
}

#[derive(Subcommand, Debug)]
enum DiskCommands {
    /// Add a new disk to the pool
    Add(DiskAddCmd),

    /// List all registered disks and their status
    List(DiskListCmd),
}

#[derive(Subcommand, Debug)]
enum SolidCommands {
    /// Set Solid marker on a directory
    Set(SolidCmd),

    /// Remove Solid marker from a directory
    Unset(SolidCmd),
}

/// Configuration commands
#[derive(Subcommand, Debug)]
enum ConfigCmd {
    /// Set or show the display language (e.g., "en", "zh-CN")
    Lang(ConfigLangCmd),
}

/// Language configuration
#[derive(Parser, Debug)]
struct ConfigLangCmd {
    /// Language code to set (e.g., "en", "zh-CN"). If omitted, shows current language.
    lang: Option<String>,
}

fn main() -> anyhow::Result<()> {
    // Parse CLI args
    let cli = Cli::parse();

    // Initialize logging based on verbosity
    let filter = match std::env::var("RUST_LOG") {
        Ok(_) => EnvFilter::from_default_env(),
        Err(_) => EnvFilter::new(match cli.verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }),
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    // Initialize i18n from saved config or detect system language
    init_i18n_from_config();

    // Enter interactive mode if requested or no command provided
    if cli.interactive || cli.command.is_none() {
        return disco::cli::interactive::run_interactive().map_err(|e| anyhow::anyhow!("{}", e));
    }

    // Dispatch to command handlers
    match cli.command.unwrap() {
        Commands::Disk(DiskCommands::Add(cmd)) => disco::cli::commands::disk::handle_add(cmd),
        Commands::Disk(DiskCommands::List(cmd)) => disco::cli::commands::disk::handle_list(cmd),
        Commands::Scan(cmd) => disco::cli::commands::scan::handle_scan(cmd),
        Commands::Search(cmd) => disco::cli::commands::search::handle_search(cmd),
        Commands::Get(cmd) => disco::cli::commands::get::handle_get(cmd),
        Commands::Store(cmd) => disco::cli::commands::store::handle_store(cmd),
        Commands::Retrieve(cmd) => disco::cli::commands::retrieve::handle_retrieve(cmd),
        Commands::Solid(SolidCommands::Set(cmd)) => disco::cli::commands::solid::handle_set(cmd),
        Commands::Solid(SolidCommands::Unset(cmd)) => disco::cli::commands::solid::handle_unset(cmd),
        Commands::Visualize(cmd) => disco::cli::commands::visualize::handle_visualize(cmd),
        Commands::Menu => disco::cli::interactive::run_menu_direct(),
        Commands::Config(ConfigCmd::Lang(cmd)) => handle_config_lang(cmd),
    }?;

    Ok(())
}

/// Initialize i18n from saved config or detect system language
fn init_i18n_from_config() {
    // Try to get saved language from config database
    let saved_lang = get_saved_language();

    let lang = match saved_lang {
        Some(lang) => lang,
        None => disco::i18n::detect_system_lang(),
    };

    if let Err(e) = disco::i18n::init(&lang) {
        eprintln!("Warning: Failed to initialize i18n ({}): using default language", e);
    }
}

/// Get saved language from config database
fn get_saved_language() -> Option<String> {
    // Need to initialize context to access database
    use disco::cli::context::AppContext;

    match AppContext::init() {
        Ok(ctx) => {
            let config = ctx.config();
            let db = ctx.db();
            config.get_value("language", db).ok().flatten()
        }
        Err(_) => None,
    }
}

/// Handle config lang command
fn handle_config_lang(cmd: ConfigLangCmd) -> disco::Result<()> {
    use disco::cli::context::AppContext;
    use disco::t;

    let ctx = AppContext::init()?;
    let config = ctx.config();
    let db = ctx.db();

    match cmd.lang {
        Some(lang) => {
            // Validate and set language
            let normalized = normalize_language(&lang);

            // Save to config
            config.set_value("language", &normalized, db)?;

            // Update runtime language
            disco::i18n::set_language(&normalized)
                .map_err(|e| disco::DiscoError::ConfigError(e))?;

            println!("✓ {}", t!("config-lang-set", "lang" => disco::i18n::get_language_name(&normalized)));
        }
        None => {
            // Show current language
            let current = disco::i18n::current_language();
            let name = disco::i18n::get_language_name(&current);
            println!("{}: {} ({})", t!("config-current-lang"), name, current);
            println!();
            println!("{}:", t!("config-available-langs"));
            for (code, name) in disco::i18n::SUPPORTED_LANGUAGES {
                let marker = if *code == current { " *" } else { "" };
                println!("  {} - {}{}", code, name, marker);
            }
            println!();
            println!("{}: disco config lang <code>", t!("config-usage"));
        }
    }

    Ok(())
}

/// Normalize language code
fn normalize_language(lang: &str) -> String {
    let lang = lang.replace('_', "-");

    // Check direct match
    for (code, _) in disco::i18n::SUPPORTED_LANGUAGES {
        if *code == lang {
            return lang;
        }
    }

    // Check prefix match
    let prefix = lang.split('-').next().unwrap_or(&lang);
    for (code, _) in disco::i18n::SUPPORTED_LANGUAGES {
        if code.starts_with(prefix) {
            return code.to_string();
        }
    }

    disco::i18n::DEFAULT_LANGUAGE.to_string()
}