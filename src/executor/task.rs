//! Task state machine for interrupt recovery

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    Store,
    Scan,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::Store => write!(f, "store"),
            TaskType::Scan => write!(f, "scan"),
        }
    }
}

impl std::str::FromStr for TaskType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "store" => Ok(TaskType::Store),
            "scan" => Ok(TaskType::Scan),
            _ => Err(format!("Invalid task type: {}", s)),
        }
    }
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Interrupted,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Interrupted => write!(f, "interrupted"),
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "running" => Ok(TaskStatus::Running),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            "interrupted" => Ok(TaskStatus::Interrupted),
            _ => Err(format!("Invalid task status: {}", s)),
        }
    }
}

/// Task state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    /// JSON-encoded task-specific payload
    pub payload: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(task_id: String, task_type: TaskType, payload: String) -> Self {
        let now = Utc::now();
        Self {
            task_id,
            task_type,
            status: TaskStatus::Pending,
            payload,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.updated_at = Utc::now();
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.updated_at = Utc::now();
    }

    pub fn fail(&mut self) {
        self.status = TaskStatus::Failed;
        self.updated_at = Utc::now();
    }

    pub fn interrupt(&mut self) {
        self.status = TaskStatus::Interrupted;
        self.updated_at = Utc::now();
    }

    pub fn is_resumable(&self) -> bool {
        self.status == TaskStatus::Interrupted || self.status == TaskStatus::Pending
    }
}

/// Store task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreTaskPayload {
    pub source_path: String,
    pub target_disk_id: String,
    pub target_relative_path: String,
    /// Files already copied (for interrupt recovery)
    pub completed_files: Vec<String>,
    /// Total files to copy
    pub total_files: usize,
}

/// Scan task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanTaskPayload {
    pub disk_id: String,
    /// Files/dirs already scanned (for interrupt recovery)
    pub scanned_count: usize,
    pub is_full_scan: bool,
}