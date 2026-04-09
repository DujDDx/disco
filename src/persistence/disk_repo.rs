//! Disk repository - CRUD operations for disks

use crate::domain::disk::{Disk, DiskId, DiskIdentity, MountStatus};
use crate::persistence::db::Database;
use crate::{Result, DiscoError};
use chrono::{DateTime, Utc};
use rusqlite::params;

pub struct DiskRepo<'a> {
    db: &'a Database,
}

impl<'a> DiskRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Insert a new disk registration
    pub fn insert_disk(&self, disk: &Disk) -> Result<()> {
        self.db.conn().execute(
            "INSERT INTO disks (disk_id, name, serial, volume_uuid, volume_label, capacity_bytes, fingerprint, first_registered, last_mount_point)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                disk.disk_id.as_str(),
                &disk.name,
                disk.identity.serial.as_ref(),
                disk.identity.volume_uuid.as_ref(),
                disk.identity.volume_label.as_ref(),
                disk.identity.capacity_bytes,
                &disk.identity.fingerprint,
                disk.first_registered.to_rfc3339(),
                disk.last_mount_point.as_ref(),
            ],
        )?;
        Ok(())
    }

    /// Get a disk by its ID
    pub fn get_disk_by_id(&self, disk_id: &DiskId) -> Result<Disk> {
        self.db.conn()
            .query_row(
                "SELECT disk_id, name, serial, volume_uuid, volume_label, capacity_bytes, fingerprint, first_registered, last_mount_point
                 FROM disks WHERE disk_id = ?1",
                [disk_id.as_str()],
                |row| {
                    Ok(Disk {
                        disk_id: DiskId::new(row.get::<_, String>(0)?),
                        name: row.get::<_, String>(1)?,
                        identity: DiskIdentity {
                            serial: row.get::<_, Option<String>>(2)?,
                            volume_uuid: row.get::<_, Option<String>>(3)?,
                            volume_label: row.get::<_, Option<String>>(4)?,
                            capacity_bytes: row.get::<_, u64>(5)?,
                            fingerprint: row.get::<_, String>(6)?,
                        },
                        first_registered: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        last_mount_point: row.get::<_, Option<String>>(8)?,
                        mount_status: MountStatus::Offline, // Computed, not stored
                        current_mount_point: None, // Computed, not stored
                    })
                },
            )
            .map_err(|_| DiscoError::DiskNotFound(disk_id.as_str().to_string()))
    }

    /// List all registered disks
    pub fn list_disks(&self) -> Result<Vec<Disk>> {
        let mut disks = Vec::new();
        let mut stmt = self.db.conn().prepare(
            "SELECT disk_id, name, serial, volume_uuid, volume_label, capacity_bytes, fingerprint, first_registered, last_mount_point
             FROM disks ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Disk {
                disk_id: DiskId::new(row.get::<_, String>(0)?),
                name: row.get::<_, String>(1)?,
                identity: DiskIdentity {
                    serial: row.get::<_, Option<String>>(2)?,
                    volume_uuid: row.get::<_, Option<String>>(3)?,
                    volume_label: row.get::<_, Option<String>>(4)?,
                    capacity_bytes: row.get::<_, u64>(5)?,
                    fingerprint: row.get::<_, String>(6)?,
                },
                first_registered: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                last_mount_point: row.get::<_, Option<String>>(8)?,
                mount_status: MountStatus::Offline,
                current_mount_point: None,
            })
        })?;

        for row in rows {
            disks.push(row?);
        }

        Ok(disks)
    }

    /// Update the last mount point for a disk
    pub fn update_last_mount_point(&self, disk_id: &DiskId, mount_point: String) -> Result<()> {
        self.db.conn().execute(
            "UPDATE disks SET last_mount_point = ?1 WHERE disk_id = ?2",
            params![mount_point, disk_id.as_str()],
        )?;
        Ok(())
    }

    /// Update disk name
    pub fn update_disk_name(&self, disk_id: &DiskId, name: &str) -> Result<()> {
        self.db.conn().execute(
            "UPDATE disks SET name = ?1 WHERE disk_id = ?2",
            params![name, disk_id.as_str()],
        )?;
        Ok(())
    }

    /// Update disk identity information (serial, volume_uuid, volume_label, capacity_bytes, fingerprint)
    pub fn update_disk_identity(&self, disk_id: &DiskId, identity: &DiskIdentity) -> Result<()> {
        self.db.conn().execute(
            "UPDATE disks SET serial = ?1, volume_uuid = ?2, volume_label = ?3, capacity_bytes = ?4, fingerprint = ?5 WHERE disk_id = ?6",
            params![
                identity.serial.as_ref(),
                identity.volume_uuid.as_ref(),
                identity.volume_label.as_ref(),
                identity.capacity_bytes,
                &identity.fingerprint,
                disk_id.as_str(),
            ],
        )?;
        Ok(())
    }

    /// Find disk by identity (for matching during mount detection)
    pub fn find_disk_by_identity(&self, identity: &DiskIdentity) -> Result<Option<Disk>> {
        let disks = self.list_disks()?;

        for disk in disks {
            if disk.identity.matches(identity) {
                return Ok(Some(disk));
            }
        }

        Ok(None)
    }

    /// Delete a disk registration (and all its entries)
    pub fn delete_disk(&self, disk_id: &DiskId) -> Result<()> {
        // First delete entries
        self.db.conn().execute(
            "DELETE FROM entries WHERE disk_id = ?1",
            [disk_id.as_str()],
        )?;
        // Then delete disk
        self.db.conn().execute(
            "DELETE FROM disks WHERE disk_id = ?1",
            [disk_id.as_str()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::db::Database;

    #[test]
    fn test_insert_and_get_disk() {
        let db = Database::open_in_memory().unwrap();
        let repo = DiskRepo::new(&db);

        let disk = Disk::new(
            DiskId::new("test-001"),
            "Test Disk".to_string(),
            DiskIdentity {
                serial: Some("ABC123".to_string()),
                volume_uuid: None,
                volume_label: Some("TEST".to_string()),
                capacity_bytes: 1000,
                fingerprint: "fp".to_string(),
            },
        );

        repo.insert_disk(&disk).unwrap();
        let retrieved = repo.get_disk_by_id(&DiskId::new("test-001")).unwrap();
        assert_eq!(retrieved.name, "Test Disk");
        assert_eq!(retrieved.identity.serial, Some("ABC123".to_string()));
    }

    #[test]
    fn test_list_disks() {
        let db = Database::open_in_memory().unwrap();
        let repo = DiskRepo::new(&db);

        repo.insert_disk(&Disk::new(
            DiskId::new("d1"),
            "Disk1".to_string(),
            DiskIdentity { serial: None, volume_uuid: None, volume_label: None, capacity_bytes: 100, fingerprint: "fp1".to_string() },
        )).unwrap();
        repo.insert_disk(&Disk::new(
            DiskId::new("d2"),
            "Disk2".to_string(),
            DiskIdentity { serial: None, volume_uuid: None, volume_label: None, capacity_bytes: 200, fingerprint: "fp2".to_string() },
        )).unwrap();

        let disks = repo.list_disks().unwrap();
        assert_eq!(disks.len(), 2);
    }
}