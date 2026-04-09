//! SQL schema definitions

pub const SCHEMA_V1: &str = "
CREATE TABLE disks (
    disk_id          TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    serial           TEXT,
    volume_uuid      TEXT,
    volume_label     TEXT,
    capacity_bytes   INTEGER NOT NULL,
    fingerprint      TEXT NOT NULL,
    first_registered TEXT NOT NULL,
    last_mount_point TEXT
);

CREATE TABLE entries (
    entry_id              INTEGER PRIMARY KEY AUTOINCREMENT,
    disk_id               TEXT NOT NULL REFERENCES disks(disk_id),
    disk_name             TEXT NOT NULL,
    relative_path         TEXT NOT NULL,
    file_name             TEXT NOT NULL,
    size                  INTEGER NOT NULL,
    hash                  TEXT,
    mtime                 TEXT NOT NULL,
    entry_type            TEXT NOT NULL CHECK(entry_type IN ('file', 'dir')),
    solid_flag            INTEGER NOT NULL DEFAULT 0,
    last_seen_mount_point TEXT NOT NULL,
    indexed_at            TEXT NOT NULL,
    status                TEXT NOT NULL DEFAULT 'normal'
                          CHECK(status IN ('normal', 'missing', 'pending_confirm')),
    UNIQUE(disk_id, relative_path)
);

CREATE INDEX idx_entries_file_name ON entries(file_name);
CREATE INDEX idx_entries_disk_id ON entries(disk_id);
CREATE INDEX idx_entries_hash ON entries(hash) WHERE hash IS NOT NULL;
CREATE INDEX idx_entries_file_name_lower ON entries(lower(file_name));

CREATE TABLE tasks (
    task_id    TEXT PRIMARY KEY,
    task_type  TEXT NOT NULL CHECK(task_type IN ('store', 'scan')),
    status     TEXT NOT NULL CHECK(status IN ('pending', 'running', 'completed', 'failed', 'interrupted')),
    payload    TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

pub const SCHEMA_V1_DOWN: &str = "
DROP TABLE IF EXISTS config;
DROP TABLE IF EXISTS tasks;
DROP INDEX IF EXISTS idx_entries_file_name_lower;
DROP INDEX IF EXISTS idx_entries_hash;
DROP INDEX IF EXISTS idx_entries_disk_id;
DROP INDEX IF EXISTS idx_entries_file_name;
DROP TABLE IF EXISTS entries;
DROP TABLE IF EXISTS disks;
";