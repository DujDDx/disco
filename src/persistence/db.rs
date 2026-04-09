//! Database connection and migration management

use crate::Result;
use rusqlite::{Connection, Transaction};
use rusqlite_migration::{Migrations, M};
use std::path::Path;

/// Database wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open database at a file path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let mut db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Open in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let mut db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Run database migrations
    fn run_migrations(&mut self) -> Result<()> {
        let migrations = Migrations::new(vec![
            // Migration 1: Initial schema
            M::up(crate::persistence::schema::SCHEMA_V1)
                .down(crate::persistence::schema::SCHEMA_V1_DOWN),
        ]);

        migrations.to_latest(&mut self.conn)?;
        Ok(())
    }

    /// Get a raw connection reference
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Begin a transaction
    pub fn transaction(&mut self) -> Result<Transaction<'_>> {
        Ok(self.conn.transaction()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_db() {
        let db = Database::open_in_memory().unwrap();
        // Tables should be created
        let count: i64 = db.conn()
            .query_row("SELECT COUNT(*) FROM disks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}