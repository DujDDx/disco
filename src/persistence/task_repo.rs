//! Task repository - CRUD operations for tasks

use crate::executor::task::{Task, TaskType, TaskStatus};
use crate::persistence::db::Database;
use crate::{Result, DiscoError};
use chrono::{DateTime, Utc};
use rusqlite::params;

pub struct TaskRepo<'a> {
    db: &'a Database,
}

impl<'a> TaskRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Insert a new task
    pub fn insert_task(&self, task: &Task) -> Result<()> {
        self.db.conn().execute(
            "INSERT INTO tasks (task_id, task_type, status, payload, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &task.task_id,
                task.task_type.to_string(),
                task.status.to_string(),
                &task.payload,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get task by ID
    pub fn get_task_by_id(&self, task_id: &str) -> Result<Task> {
        self.db.conn()
            .query_row(
                "SELECT task_id, task_type, status, payload, created_at, updated_at
                 FROM tasks WHERE task_id = ?1",
                [task_id],
                |row| {
                    Ok(Task {
                        task_id: row.get(0)?,
                        task_type: row.get::<_, String>(1)?.parse().unwrap_or(TaskType::Store),
                        status: row.get::<_, String>(2)?.parse().unwrap_or(TaskStatus::Pending),
                        payload: row.get(3)?,
                        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    })
                },
            )
            .map_err(|_| DiscoError::TaskFailed(format!("Task not found: {}", task_id)))
    }

    /// Update task status
    pub fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<()> {
        self.db.conn().execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE task_id = ?3",
            params![status.to_string(), Utc::now().to_rfc3339(), task_id],
        )?;
        Ok(())
    }

    /// Update task payload (for interrupt recovery progress)
    pub fn update_task_payload(&self, task_id: &str, payload: &str) -> Result<()> {
        self.db.conn().execute(
            "UPDATE tasks SET payload = ?1, updated_at = ?2 WHERE task_id = ?3",
            params![payload, Utc::now().to_rfc3339(), task_id],
        )?;
        Ok(())
    }

    /// List all pending or interrupted tasks (for recovery)
    pub fn list_resumable_tasks(&self) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        let mut stmt = self.db.conn().prepare(
            "SELECT task_id, task_type, status, payload, created_at, updated_at
             FROM tasks WHERE status IN ('pending', 'interrupted')
             ORDER BY created_at",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Task {
                task_id: row.get(0)?,
                task_type: row.get::<_, String>(1)?.parse().unwrap_or(TaskType::Store),
                status: row.get::<_, String>(2)?.parse().unwrap_or(TaskStatus::Pending),
                payload: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        for row in rows {
            tasks.push(row?);
        }

        Ok(tasks)
    }

    /// Delete a task
    pub fn delete_task(&self, task_id: &str) -> Result<()> {
        self.db.conn().execute("DELETE FROM tasks WHERE task_id = ?1", [task_id])?;
        Ok(())
    }

    /// Clean up completed tasks older than a threshold
    pub fn cleanup_completed_tasks(&self, days: i64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let count = self.db.conn().execute(
            "DELETE FROM tasks WHERE status IN ('completed', 'failed') AND updated_at < ?1",
            [cutoff.to_rfc3339()],
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::db::Database;
    use crate::executor::task::StoreTaskPayload;
    use serde_json;

    #[test]
    fn test_insert_and_get_task() {
        let db = Database::open_in_memory().unwrap();
        let repo = TaskRepo::new(&db);

        let payload = StoreTaskPayload {
            source_path: "/src/file".to_string(),
            target_disk_id: "disk1".to_string(),
            target_relative_path: "file".to_string(),
            completed_files: vec![],
            total_files: 1,
        };

        let task = Task::new(
            "task-001".to_string(),
            TaskType::Store,
            serde_json::to_string(&payload).unwrap(),
        );

        repo.insert_task(&task).unwrap();
        let retrieved = repo.get_task_by_id("task-001").unwrap();
        assert_eq!(retrieved.task_type, TaskType::Store);
        assert_eq!(retrieved.status, TaskStatus::Pending);
    }

    #[test]
    fn test_update_task_status() {
        let db = Database::open_in_memory().unwrap();
        let repo = TaskRepo::new(&db);

        let task = Task::new("task-002".to_string(), TaskType::Scan, "{}".to_string());
        repo.insert_task(&task).unwrap();

        repo.update_task_status("task-002", TaskStatus::Running).unwrap();
        let updated = repo.get_task_by_id("task-002").unwrap();
        assert_eq!(updated.status, TaskStatus::Running);
    }
}