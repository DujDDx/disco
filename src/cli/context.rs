//! Application context for CLI commands

use crate::persistence::{config::Config, db::Database, disk_repo::DiskRepo, entry_repo::EntryRepo, task_repo::TaskRepo};
use crate::storage::{fs::FsAdapter, platform::DiskDetector};
use crate::Result;
use std::path::PathBuf;

/// Application context that holds all shared state
pub struct AppContext {
    pub db: Database,
    pub config: Config,
    pub data_dir: PathBuf,
}

impl AppContext {
    /// Initialize application context
    pub fn init() -> Result<Self> {
        let config = Config::load()?;
        let data_dir = config.data_dir.clone();
        let db = Database::open(&config.db_path)?;

        Ok(Self {
            db,
            config,
            data_dir,
        })
    }

    /// Get config reference
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get database reference
    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Get disk repository
    pub fn disk_repo(&self) -> DiskRepo<'_> {
        DiskRepo::new(&self.db)
    }

    /// Get entry repository
    pub fn entry_repo(&self) -> EntryRepo<'_> {
        EntryRepo::new(&self.db)
    }

    /// Get task repository
    pub fn task_repo(&self) -> TaskRepo<'_> {
        TaskRepo::new(&self.db)
    }

    /// Get platform disk detector
    pub fn disk_detector() -> impl DiskDetector {
        crate::storage::platform::get_detector()
    }

    /// Get file system adapter
    pub fn fs_adapter() -> FsAdapter {
        FsAdapter::new()
    }
}