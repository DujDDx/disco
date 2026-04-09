//! Configuration and data directory management

use crate::{Result, DiscoError};
use directories::ProjectDirs;
use std::path::PathBuf;
use rusqlite::params;

/// Configuration for Disco
pub struct Config {
    /// Base data directory (~/.disco/)
    pub data_dir: PathBuf,
    /// Database file path
    pub db_path: PathBuf,
    /// Log file path
    pub log_path: PathBuf,
    /// Default SolidLayer value
    pub default_solid_layer: String,
    /// Hash calculation mode (off, on_demand, full)
    pub hash_mode: HashMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashMode {
    Off,
    OnDemand,
    Full,
}

impl Config {
    /// Load configuration, creating directories if needed
    pub fn load() -> Result<Self> {
        let project_dirs = ProjectDirs::from("", "", "disco")
            .ok_or_else(|| DiscoError::ConfigError("Could not determine data directory".to_string()))?;

        let data_dir = project_dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;

        Self::load_from_dir(&data_dir)
    }

    /// Load from a specific directory (for testing)
    pub fn load_from_dir(data_dir: &std::path::Path) -> Result<Self> {
        let db_path = data_dir.join("index.db");
        let log_path = data_dir.join("disco.log");

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            db_path,
            log_path,
            default_solid_layer: "0".to_string(),
            hash_mode: HashMode::OnDemand,
        })
    }

    /// Get config value from database
    pub fn get_value(&self, key: &str, db: &crate::persistence::db::Database) -> Result<Option<String>> {
        let result = db.conn()
            .query_row(
                "SELECT value FROM config WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            );

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set config value in database
    pub fn set_value(&self, key: &str, value: &str, db: &crate::persistence::db::Database) -> Result<()> {
        db.conn().execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::load().unwrap_or_else(|_| {
            Self {
                data_dir: PathBuf::from("~/.disco"),
                db_path: PathBuf::from("~/.disco/index.db"),
                log_path: PathBuf::from("~/.disco/disco.log"),
                default_solid_layer: "0".to_string(),
                hash_mode: HashMode::OnDemand,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_from_dir() {
        let temp = TempDir::new().unwrap();
        let config = Config::load_from_dir(temp.path()).unwrap();
        assert!(config.db_path.ends_with("index.db"));
        assert!(config.log_path.ends_with("disco.log"));
    }
}