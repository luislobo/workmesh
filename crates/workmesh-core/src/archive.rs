use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime};
use thiserror::Error;

use crate::task::Task;

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Missing task path for {0}")]
    MissingPath(String),
    #[error("Failed to move task: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct ArchiveOptions {
    pub before: NaiveDate,
    /// Explicit statuses to archive. When empty, terminal defaults are used.
    pub statuses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ArchiveResult {
    pub archived: Vec<String>,
    pub skipped: Vec<String>,
    pub archive_dir: PathBuf,
}

pub fn archive_tasks(
    backlog_dir: &Path,
    tasks: &[Task],
    options: &ArchiveOptions,
) -> Result<ArchiveResult, ArchiveError> {
    let archive_root = backlog_dir.join("archive");
    let mut archived = Vec::new();
    let skipped = Vec::new();
    let allowed_statuses = if options.statuses.is_empty() {
        default_archive_statuses()
            .iter()
            .map(|value| normalize_status(value))
            .collect::<std::collections::HashSet<_>>()
    } else {
        options
            .statuses
            .iter()
            .map(|value| normalize_status(value))
            .collect::<std::collections::HashSet<_>>()
    };

    for task in tasks {
        if !allowed_statuses.contains(&normalize_status(&task.status)) {
            continue;
        }
        let task_date = task_date(task).unwrap_or_else(|| Local::now().date_naive());
        if task_date > options.before {
            continue;
        }
        let path = task
            .file_path
            .as_ref()
            .ok_or_else(|| ArchiveError::MissingPath(task.id.clone()))?;
        let month_dir = format!("{:04}-{:02}", task_date.year(), task_date.month());
        let target_dir = archive_root.join(month_dir);
        fs::create_dir_all(&target_dir)?;
        let target = target_dir.join(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );
        fs::rename(path, &target)?;
        archived.push(task.id.clone());
    }

    Ok(ArchiveResult {
        archived,
        skipped,
        archive_dir: archive_root,
    })
}

pub fn default_archive_statuses() -> &'static [&'static str] {
    &["Done", "Cancelled", "Canceled", "Won't Do", "Wont Do"]
}

fn normalize_status(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn parse_task_date(value: &str) -> Option<NaiveDate> {
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return Some(date);
    }
    if let Ok(date_time) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M") {
        return Some(date_time.date());
    }
    None
}

fn task_date(task: &Task) -> Option<NaiveDate> {
    if let Some(value) = task.updated_date.as_deref().and_then(parse_task_date) {
        return Some(value);
    }
    if let Some(value) = task.created_date.as_deref().and_then(parse_task_date) {
        return Some(value);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::load_tasks;
    use crate::task_ops::create_task_file;
    use tempfile::TempDir;

    #[test]
    fn archive_moves_done_tasks_by_month() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path().join("workmesh");
        let tasks_dir = backlog_dir.join("tasks");
        fs::create_dir_all(&tasks_dir).expect("tasks dir");

        let _ = create_task_file(
            &tasks_dir,
            "task-001",
            "Done Task",
            "Done",
            "P2",
            "Phase1",
            &[],
            &[],
            &[],
        )
        .expect("create");

        let mut tasks = load_tasks(&backlog_dir);
        for task in &mut tasks {
            task.updated_date = Some("2024-01-15 10:00".to_string());
        }

        let result = archive_tasks(
            &backlog_dir,
            &tasks,
            &ArchiveOptions {
                before: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                statuses: vec!["Done".to_string()],
            },
        )
        .expect("archive");

        assert_eq!(result.archived, vec!["task-001".to_string()]);
        let archive_dir = backlog_dir.join("archive").join("2024-01");
        assert!(archive_dir.is_dir());
        let archived = fs::read_dir(&archive_dir)
            .expect("read archive")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .starts_with("task-001")
            });
        assert!(archived);
    }

    #[test]
    fn archive_defaults_to_terminal_statuses_only() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path().join("workmesh");
        let tasks_dir = backlog_dir.join("tasks");
        fs::create_dir_all(&tasks_dir).expect("tasks dir");

        let _ = create_task_file(
            &tasks_dir,
            "task-001",
            "Done Task",
            "Done",
            "P2",
            "Phase1",
            &[],
            &[],
            &[],
        )
        .expect("create done");
        let _ = create_task_file(
            &tasks_dir,
            "task-002",
            "Cancelled Task",
            "Cancelled",
            "P2",
            "Phase1",
            &[],
            &[],
            &[],
        )
        .expect("create cancelled");
        let _ = create_task_file(
            &tasks_dir,
            "task-003",
            "Todo Task",
            "To Do",
            "P2",
            "Phase1",
            &[],
            &[],
            &[],
        )
        .expect("create todo");

        let mut tasks = load_tasks(&backlog_dir);
        for task in &mut tasks {
            task.updated_date = Some("2024-01-15 10:00".to_string());
        }

        let result = archive_tasks(
            &backlog_dir,
            &tasks,
            &ArchiveOptions {
                before: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                statuses: Vec::new(),
            },
        )
        .expect("archive");

        assert_eq!(
            result.archived,
            vec!["task-001".to_string(), "task-002".to_string()]
        );
        let todo_still_present = fs::read_dir(&tasks_dir)
            .expect("read tasks")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .starts_with("task-003")
            });
        assert!(todo_still_present);
    }
}
