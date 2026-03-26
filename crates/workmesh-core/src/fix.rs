use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::task::{Task, TaskParseError};
use crate::task_ops::{canonical_task_filename, set_list_field, update_task_field, FieldValue};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FixerKind {
    Uid,
    Deps,
    Ids,
    Filenames,
}

impl FixerKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FixerKind::Uid => "uid",
            FixerKind::Deps => "deps",
            FixerKind::Ids => "ids",
            FixerKind::Filenames => "filenames",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UidFixChange {
    pub task_id: String,
    pub path: Option<PathBuf>,
    pub uid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UidFixReport {
    pub detected: usize,
    pub fixed: usize,
    pub skipped: usize,
    pub changes: Vec<UidFixChange>,
    pub warnings: Vec<String>,
}

pub fn backfill_missing_uids(tasks: &[Task], apply: bool) -> Result<UidFixReport, TaskParseError> {
    let mut report = UidFixReport::default();
    let mut used_uids: HashSet<String> = tasks
        .iter()
        .filter_map(|task| task.uid.as_deref())
        .filter(|uid| !uid.trim().is_empty())
        .map(|uid| uid.to_lowercase())
        .collect();

    let mut candidates: Vec<&Task> = tasks
        .iter()
        .filter(|task| {
            task.uid
                .as_deref()
                .map(|uid| uid.trim())
                .unwrap_or("")
                .is_empty()
        })
        .collect();
    candidates.sort_by(|a, b| a.id.cmp(&b.id));

    for task in candidates {
        report.detected += 1;
        let Some(path) = task.file_path.as_ref() else {
            report.skipped += 1;
            report.warnings.push(format!(
                "{} missing uid but has no file path; skipping",
                task.id
            ));
            report.changes.push(UidFixChange {
                task_id: task.id.clone(),
                path: None,
                uid: None,
            });
            continue;
        };

        let mut change = UidFixChange {
            task_id: task.id.clone(),
            path: Some(path.clone()),
            uid: None,
        };

        if apply {
            let uid = next_unique_ulid(&used_uids);
            update_task_field(path, "uid", Some(FieldValue::Scalar(uid.clone())))?;
            used_uids.insert(uid.to_lowercase());
            report.fixed += 1;
            change.uid = Some(uid);
        }

        report.changes.push(change);
    }

    Ok(report)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyFixChange {
    pub task_id: String,
    pub path: Option<PathBuf>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DependencyFixReport {
    pub detected: usize,
    pub fixed: usize,
    pub skipped: usize,
    pub changes: Vec<DependencyFixChange>,
    pub warnings: Vec<String>,
}

pub fn fix_dependencies(
    tasks: &[Task],
    apply: bool,
) -> Result<DependencyFixReport, TaskParseError> {
    let existing_ids: HashSet<String> = tasks.iter().map(|task| task.id.to_lowercase()).collect();
    let mut report = DependencyFixReport::default();

    let mut sorted: Vec<&Task> = tasks.iter().collect();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));

    for task in sorted {
        let (cleaned, removed) = clean_dependencies(task, &existing_ids);
        if removed.is_empty() {
            continue;
        }

        report.detected += 1;
        let Some(path) = task.file_path.as_ref() else {
            report.skipped += 1;
            report.warnings.push(format!(
                "{} has invalid dependencies but no file path; skipping",
                task.id
            ));
            report.changes.push(DependencyFixChange {
                task_id: task.id.clone(),
                path: None,
                removed,
            });
            continue;
        };

        if apply {
            set_list_field(path, "dependencies", cleaned)?;
            report.fixed += 1;
        }

        report.changes.push(DependencyFixChange {
            task_id: task.id.clone(),
            path: Some(path.clone()),
            removed,
        });
    }

    Ok(report)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilenameFixChange {
    pub task_id: String,
    pub uid: Option<String>,
    pub old_path: Option<PathBuf>,
    pub new_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FilenameFixReport {
    pub detected: usize,
    pub fixed: usize,
    pub skipped: usize,
    pub changes: Vec<FilenameFixChange>,
    pub warnings: Vec<String>,
}

pub fn fix_task_filenames(
    tasks: &[Task],
    apply: bool,
) -> Result<FilenameFixReport, TaskParseError> {
    let mut report = FilenameFixReport::default();
    let mut sorted: Vec<&Task> = tasks.iter().collect();
    sorted.sort_by_key(|task| {
        (
            task.id.clone(),
            task.file_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
        )
    });

    for task in sorted {
        let Some(path) = task.file_path.as_ref() else {
            report.skipped += 1;
            report.warnings.push(format!(
                "{} has no file path; skipping filename normalization",
                task.id
            ));
            report.changes.push(FilenameFixChange {
                task_id: task.id.clone(),
                uid: task.uid.clone(),
                old_path: None,
                new_path: None,
            });
            continue;
        };

        let Some(uid) = task
            .uid
            .as_deref()
            .map(str::trim)
            .filter(|uid| !uid.is_empty())
        else {
            report.skipped += 1;
            report.warnings.push(format!(
                "{} is missing uid; run `fix uid --apply` before normalizing filenames",
                task.id
            ));
            report.changes.push(FilenameFixChange {
                task_id: task.id.clone(),
                uid: task.uid.clone(),
                old_path: Some(path.clone()),
                new_path: Some(path.clone()),
            });
            continue;
        };

        let expected_path = path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(canonical_task_filename(&task.id, &task.title, uid));

        if expected_path == *path {
            continue;
        }

        report.detected += 1;
        let mut change = FilenameFixChange {
            task_id: task.id.clone(),
            uid: task.uid.clone(),
            old_path: Some(path.clone()),
            new_path: Some(expected_path.clone()),
        };

        if apply {
            if expected_path.exists() {
                report.skipped += 1;
                report.warnings.push(format!(
                    "{} target filename already exists; skipping {}",
                    task.id,
                    expected_path.display()
                ));
                change.new_path = Some(path.clone());
            } else {
                fs::rename(path, &expected_path)
                    .map_err(|err| TaskParseError::Invalid(err.to_string()))?;
                report.fixed += 1;
            }
        }

        report.changes.push(change);
    }

    Ok(report)
}

fn clean_dependencies(task: &Task, existing_ids: &HashSet<String>) -> (Vec<String>, Vec<String>) {
    let mut seen = HashSet::new();
    let mut cleaned = Vec::new();
    let mut removed = Vec::new();

    for dep in &task.dependencies {
        let dep_trimmed = dep.trim();
        let dep_lower = dep_trimmed.to_lowercase();
        let is_blank = dep_trimmed.is_empty();
        let is_missing = !is_blank && !existing_ids.contains(&dep_lower);
        let is_duplicate = !is_blank && seen.contains(&dep_lower);

        if is_blank || is_missing || is_duplicate {
            removed.push(dep.clone());
            continue;
        }

        seen.insert(dep_lower);
        cleaned.push(dep.trim().to_string());
    }

    (cleaned, removed)
}

fn next_unique_ulid(used: &HashSet<String>) -> String {
    loop {
        let candidate = Ulid::new().to_string();
        if !used.contains(&candidate.to_lowercase()) {
            return candidate;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::task::load_tasks;

    use super::*;

    fn write_task(backlog_dir: &std::path::Path, file_name: &str, body: &str) {
        let tasks_dir = backlog_dir.join("tasks");
        fs::create_dir_all(&tasks_dir).expect("mkdir");
        fs::write(tasks_dir.join(file_name), body).expect("write");
    }

    #[test]
    fn uid_fix_detects_and_applies_missing_uids() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path();
        write_task(
            backlog_dir,
            "task-main-001 - alpha.md",
            "---\nid: task-main-001\ntitle: Alpha\nkind: task\nstatus: To Do\npriority: P2\nphase: Phase1\ndependencies: []\nlabels: []\nassignee: []\n---\n",
        );

        let tasks = load_tasks(backlog_dir);
        let dry = backfill_missing_uids(&tasks, false).expect("dry");
        assert_eq!(dry.detected, 1);
        assert_eq!(dry.fixed, 0);
        assert_eq!(dry.changes[0].uid, None);

        let tasks = load_tasks(backlog_dir);
        let applied = backfill_missing_uids(&tasks, true).expect("apply");
        assert_eq!(applied.detected, 1);
        assert_eq!(applied.fixed, 1);
        assert!(applied.changes[0].uid.is_some());

        let tasks = load_tasks(backlog_dir);
        assert!(tasks[0].uid.is_some());
    }

    #[test]
    fn deps_fix_removes_missing_and_duplicate_dependencies() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path();
        write_task(
            backlog_dir,
            "task-main-001 - alpha.md",
            "---\nid: task-main-001\ntitle: Alpha\nkind: task\nstatus: To Do\npriority: P2\nphase: Phase1\ndependencies: [task-main-002, task-main-002, task-missing-999]\nlabels: []\nassignee: []\n---\n",
        );
        write_task(
            backlog_dir,
            "task-main-002 - beta.md",
            "---\nid: task-main-002\ntitle: Beta\nkind: task\nstatus: To Do\npriority: P2\nphase: Phase1\ndependencies: []\nlabels: []\nassignee: []\n---\n",
        );

        let tasks = load_tasks(backlog_dir);
        let dry = fix_dependencies(&tasks, false).expect("dry");
        assert_eq!(dry.detected, 1);
        assert_eq!(dry.fixed, 0);
        assert_eq!(
            dry.changes[0].removed,
            vec!["task-main-002".to_string(), "task-missing-999".to_string()]
        );

        let tasks = load_tasks(backlog_dir);
        let applied = fix_dependencies(&tasks, true).expect("apply");
        assert_eq!(applied.detected, 1);
        assert_eq!(applied.fixed, 1);

        let tasks = load_tasks(backlog_dir);
        let task = tasks
            .into_iter()
            .find(|task| task.id == "task-main-001")
            .expect("task");
        assert_eq!(task.dependencies, vec!["task-main-002".to_string()]);
    }

    #[test]
    fn filename_fix_normalizes_percent_encoded_names() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path();
        write_task(
            backlog_dir,
            "task-meli-207%20-%20copiar%20datos%20base%20desde%20una%20publicacin%20similar%20-%2001KMKKJE.md",
            "---\nid: task-meli-207\nuid: 01KMKKJE9ABCDEFGHIJKLMN\ntitle: Copiar datos base desde una publicaci%C3%B3n similar\nkind: task\nstatus: To Do\npriority: P2\nphase: Phase1\ndependencies: []\nlabels: []\nassignee: []\n---\n",
        );

        let tasks = load_tasks(backlog_dir);
        let dry = fix_task_filenames(&tasks, false).expect("dry");
        assert_eq!(dry.detected, 1);
        assert_eq!(dry.fixed, 0);
        assert_eq!(dry.skipped, 0);
        let new_name = dry.changes[0]
            .new_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .expect("new name");
        assert_eq!(
            new_name,
            "task-meli-207 - copiar datos base desde una publicacin similar - 01KMKKJE.md"
        );

        let tasks = load_tasks(backlog_dir);
        let applied = fix_task_filenames(&tasks, true).expect("apply");
        assert_eq!(applied.fixed, 1);
        assert!(backlog_dir
            .join("tasks")
            .join("task-meli-207 - copiar datos base desde una publicacin similar - 01KMKKJE.md")
            .exists());
    }

    #[test]
    fn filename_fix_skips_tasks_without_uid() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path();
        write_task(
            backlog_dir,
            "task-main-001%20-%20alpha.md",
            "---\nid: task-main-001\ntitle: Alpha\nkind: task\nstatus: To Do\npriority: P2\nphase: Phase1\ndependencies: []\nlabels: []\nassignee: []\n---\n",
        );

        let tasks = load_tasks(backlog_dir);
        let report = fix_task_filenames(&tasks, true).expect("apply");
        assert_eq!(report.detected, 0);
        assert_eq!(report.fixed, 0);
        assert_eq!(report.skipped, 1);
        assert!(report.warnings[0].contains("run `fix uid --apply`"));
    }
}
