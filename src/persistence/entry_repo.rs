//! Entry repository - CRUD operations for index entries

use crate::domain::disk::DiskId;
use crate::domain::entry::{IndexEntry, EntryType, EntryStatus};
use crate::persistence::db::Database;
use crate::{Result, DiscoError};
use chrono::{DateTime, Utc};
use rusqlite::params;

pub struct EntryRepo<'a> {
    db: &'a Database,
}

impl<'a> EntryRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Upsert an entry (insert or update if exists)
    pub fn upsert_entry(&self, entry: &IndexEntry) -> Result<i64> {
        // Check if entry exists
        let existing = self.db.conn()
            .query_row(
                "SELECT entry_id FROM entries WHERE disk_id = ?1 AND relative_path = ?2",
                params![entry.disk_id.as_str(), &entry.relative_path],
                |row| row.get::<_, i64>(0),
            );

        match existing {
            Ok(entry_id) => {
                // Update existing entry
                self.db.conn().execute(
                    "UPDATE entries SET
                        file_name = ?1, size = ?2, hash = ?3, mtime = ?4,
                        entry_type = ?5, solid_flag = ?6, last_seen_mount_point = ?7,
                        indexed_at = ?8, status = ?9, disk_name = ?10
                     WHERE entry_id = ?11",
                    params![
                        &entry.file_name,
                        entry.size,
                        entry.hash.as_ref(),
                        entry.mtime.to_rfc3339(),
                        entry.entry_type.to_string(),
                        entry.solid_flag as i64,
                        &entry.last_seen_mount_point,
                        entry.indexed_at.to_rfc3339(),
                        entry.status.to_string(),
                        &entry.disk_name,
                        entry_id,
                    ],
                )?;
                Ok(entry_id)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Insert new entry
                self.db.conn().execute(
                    "INSERT INTO entries (disk_id, disk_name, relative_path, file_name, size, hash, mtime, entry_type, solid_flag, last_seen_mount_point, indexed_at, status)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        entry.disk_id.as_str(),
                        &entry.disk_name,
                        &entry.relative_path,
                        &entry.file_name,
                        entry.size,
                        entry.hash.as_ref(),
                        entry.mtime.to_rfc3339(),
                        entry.entry_type.to_string(),
                        entry.solid_flag as i64,
                        &entry.last_seen_mount_point,
                        entry.indexed_at.to_rfc3339(),
                        entry.status.to_string(),
                    ],
                )?;
                Ok(self.db.conn().last_insert_rowid())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Batch upsert entries (using transaction)
    pub fn batch_upsert(&self, entries: &[IndexEntry]) -> Result<Vec<i64>> {
        let ids: Vec<i64> = entries.iter().map(|e| self.upsert_entry(e)).collect::<Result<Vec<_>>>()?;
        Ok(ids)
    }

    /// Search entries by file name (SQL LIKE)
    pub fn search_by_name(&self, keyword: &str, limit: usize) -> Result<Vec<IndexEntry>> {
        let mut entries = Vec::new();
        let mut stmt = self.db.conn().prepare(
            "SELECT entry_id, disk_id, disk_name, relative_path, file_name, size, hash, mtime, entry_type, solid_flag, last_seen_mount_point, indexed_at, status
             FROM entries
             WHERE lower(file_name) LIKE lower(?1)
             ORDER BY indexed_at DESC
             LIMIT ?2",
        )?;

        let keyword_pattern = format!("%{}%", keyword);
        let rows = stmt.query_map(params![keyword_pattern, limit as i64], |row| {
            Ok(IndexEntry {
                entry_id: row.get::<_, i64>(0)?,
                disk_id: DiskId::new(row.get::<_, String>(1)?),
                disk_name: row.get::<_, String>(2)?,
                relative_path: row.get::<_, String>(3)?,
                file_name: row.get::<_, String>(4)?,
                size: row.get::<_, u64>(5)?,
                hash: row.get::<_, Option<String>>(6)?,
                mtime: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                entry_type: row.get::<_, String>(8)?.parse().unwrap_or(EntryType::File),
                solid_flag: row.get::<_, i64>(9)? != 0,
                last_seen_mount_point: row.get::<_, String>(10)?,
                indexed_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                status: row.get::<_, String>(12)?.parse().unwrap_or(EntryStatus::Normal),
            })
        })?;

        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }

    /// Get entry by ID
    pub fn get_entry_by_id(&self, entry_id: i64) -> Result<IndexEntry> {
        self.db.conn()
            .query_row(
                "SELECT entry_id, disk_id, disk_name, relative_path, file_name, size, hash, mtime, entry_type, solid_flag, last_seen_mount_point, indexed_at, status
                 FROM entries WHERE entry_id = ?1",
                [entry_id],
                |row| {
                    Ok(IndexEntry {
                        entry_id: row.get::<_, i64>(0)?,
                        disk_id: DiskId::new(row.get::<_, String>(1)?),
                        disk_name: row.get::<_, String>(2)?,
                        relative_path: row.get::<_, String>(3)?,
                        file_name: row.get::<_, String>(4)?,
                        size: row.get::<_, u64>(5)?,
                        hash: row.get::<_, Option<String>>(6)?,
                        mtime: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        entry_type: row.get::<_, String>(8)?.parse().unwrap_or(EntryType::File),
                        solid_flag: row.get::<_, i64>(9)? != 0,
                        last_seen_mount_point: row.get::<_, String>(10)?,
                        indexed_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        status: row.get::<_, String>(12)?.parse().unwrap_or(EntryStatus::Normal),
                    })
                },
            )
            .map_err(|_| DiscoError::EntryNotFound(entry_id))
    }

    /// Get all entries for a disk
    pub fn get_entries_by_disk(&self, disk_id: &DiskId) -> Result<Vec<IndexEntry>> {
        let mut entries = Vec::new();
        let mut stmt = self.db.conn().prepare(
            "SELECT entry_id, disk_id, disk_name, relative_path, file_name, size, hash, mtime, entry_type, solid_flag, last_seen_mount_point, indexed_at, status
             FROM entries WHERE disk_id = ?1
             ORDER BY relative_path",
        )?;

        let rows = stmt.query_map([disk_id.as_str()], |row| {
            Ok(IndexEntry {
                entry_id: row.get::<_, i64>(0)?,
                disk_id: DiskId::new(row.get::<_, String>(1)?),
                disk_name: row.get::<_, String>(2)?,
                relative_path: row.get::<_, String>(3)?,
                file_name: row.get::<_, String>(4)?,
                size: row.get::<_, u64>(5)?,
                hash: row.get::<_, Option<String>>(6)?,
                mtime: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                entry_type: row.get::<_, String>(8)?.parse().unwrap_or(EntryType::File),
                solid_flag: row.get::<_, i64>(9)? != 0,
                last_seen_mount_point: row.get::<_, String>(10)?,
                indexed_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                status: row.get::<_, String>(12)?.parse().unwrap_or(EntryStatus::Normal),
            })
        })?;

        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }

    /// Mark an entry as missing
    pub fn mark_missing(&self, disk_id: &DiskId, relative_path: &str) -> Result<()> {
        self.db.conn().execute(
            "UPDATE entries SET status = 'missing' WHERE disk_id = ?1 AND relative_path = ?2",
            params![disk_id.as_str(), relative_path],
        )?;
        Ok(())
    }

    /// Set Solid flag on a directory
    pub fn set_solid_flag(&self, disk_id: &DiskId, relative_path: &str) -> Result<()> {
        self.db.conn().execute(
            "UPDATE entries SET solid_flag = 1 WHERE disk_id = ?1 AND relative_path = ?2",
            params![disk_id.as_str(), relative_path],
        )?;
        Ok(())
    }

    /// Remove Solid flag
    pub fn unset_solid_flag(&self, disk_id: &DiskId, relative_path: &str) -> Result<()> {
        self.db.conn().execute(
            "UPDATE entries SET solid_flag = 0 WHERE disk_id = ?1 AND relative_path = ?2",
            params![disk_id.as_str(), relative_path],
        )?;
        Ok(())
    }

    /// Find entry by hash (for deduplication)
    pub fn find_by_hash(&self, hash: &str) -> Result<Option<IndexEntry>> {
        let result = self.db.conn()
            .query_row(
                "SELECT entry_id, disk_id, disk_name, relative_path, file_name, size, hash, mtime, entry_type, solid_flag, last_seen_mount_point, indexed_at, status
                 FROM entries WHERE hash = ?1 LIMIT 1",
                [hash],
                |row| {
                    Ok(IndexEntry {
                        entry_id: row.get::<_, i64>(0)?,
                        disk_id: DiskId::new(row.get::<_, String>(1)?),
                        disk_name: row.get::<_, String>(2)?,
                        relative_path: row.get::<_, String>(3)?,
                        file_name: row.get::<_, String>(4)?,
                        size: row.get::<_, u64>(5)?,
                        hash: row.get::<_, Option<String>>(6)?,
                        mtime: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        entry_type: row.get::<_, String>(8)?.parse().unwrap_or(EntryType::File),
                        solid_flag: row.get::<_, i64>(9)? != 0,
                        last_seen_mount_point: row.get::<_, String>(10)?,
                        indexed_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        status: row.get::<_, String>(12)?.parse().unwrap_or(EntryStatus::Normal),
                    })
                },
            );

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete entry by ID
    pub fn delete_entry(&self, entry_id: i64) -> Result<()> {
        self.db.conn().execute("DELETE FROM entries WHERE entry_id = ?1", [entry_id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::db::Database;
    use crate::persistence::disk_repo::DiskRepo;
    use crate::domain::disk::{Disk, DiskIdentity};

    fn create_test_entry(disk_id: &str, path: &str) -> IndexEntry {
        IndexEntry {
            entry_id: 0,
            disk_id: DiskId::new(disk_id),
            disk_name: "TestDisk".to_string(),
            relative_path: path.to_string(),
            file_name: "test.txt".to_string(),
            size: 100,
            hash: Some("abc123".to_string()),
            mtime: Utc::now(),
            entry_type: EntryType::File,
            solid_flag: false,
            last_seen_mount_point: "/mnt/test".to_string(),
            indexed_at: Utc::now(),
            status: EntryStatus::Normal,
        }
    }

    fn setup_test_disk(db: &Database, disk_id: &str) {
        let disk_repo = DiskRepo::new(db);
        let disk = Disk::new(
            DiskId::new(disk_id),
            "TestDisk".to_string(),
            DiskIdentity {
                serial: None,
                volume_uuid: None,
                volume_label: None,
                capacity_bytes: 1000,
                fingerprint: "test".to_string(),
            },
        );
        disk_repo.insert_disk(&disk).unwrap();
    }

    #[test]
    fn test_upsert_and_get() {
        let db = Database::open_in_memory().unwrap();
        setup_test_disk(&db, "d1");

        let repo = EntryRepo::new(&db);

        let entry = create_test_entry("d1", "test.txt");
        let id = repo.upsert_entry(&entry).unwrap();
        assert!(id > 0);

        let retrieved = repo.get_entry_by_id(id).unwrap();
        assert_eq!(retrieved.relative_path, "test.txt");
    }

    #[test]
    fn test_search_by_name() {
        let db = Database::open_in_memory().unwrap();
        setup_test_disk(&db, "d1");

        let repo = EntryRepo::new(&db);

        repo.upsert_entry(&IndexEntry {
            entry_id: 0,
            disk_id: DiskId::new("d1"),
            disk_name: "Disk1".to_string(),
            relative_path: "doc.pdf".to_string(),
            file_name: "document.pdf".to_string(),
            size: 1000,
            hash: None,
            mtime: Utc::now(),
            entry_type: EntryType::File,
            solid_flag: false,
            last_seen_mount_point: "/mnt".to_string(),
            indexed_at: Utc::now(),
            status: EntryStatus::Normal,
        }).unwrap();

        let results = repo.search_by_name("doc", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_name, "document.pdf");
    }
}