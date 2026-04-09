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

fn main() -> anyhow::Result<()> {
    // Initialize logging based on verbosity
    let cli = Cli::parse();
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
    }?;

    Ok(())
}