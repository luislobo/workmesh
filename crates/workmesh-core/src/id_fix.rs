use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::task::{Task, TaskParseError};
use crate::task_ops::{update_task_field, FieldValue};

#[derive(Debug, Clone)]
pub struct FixIdsOptions {
    pub apply: bool,
}

impl Default for FixIdsOptions {
    fn default() -> Self {
        Self { apply: false }
    }
}

#[derive(Debug, Clone)]
pub struct FixIdsChange {
    pub old_id: String,
    pub new_id: String,
    pub old_path: PathBuf,
    pub new_path: PathBuf,
    pub uid: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FixIdsReport {
    pub changes: Vec<FixIdsChange>,
    pub warnings: Vec<String>,
}

fn lower_set(tasks: &[Task]) -> HashSet<String> {
    tasks.iter().map(|t| t.id.to_lowercase()).collect()
}

fn parse_namespaced(id: &str) -> Option<(String, i32)> {
    // task-<slug>-NNN
    let re = Regex::new(r"^(task-[a-z0-9-]+-)(\d{3})$").expect("regex");
    let lowered = id.to_lowercase();
    let caps = re.captures(&lowered)?;
    let prefix = caps.get(1)?.as_str().to_string();
    let num = caps.get(2)?.as_str().parse::<i32>().ok()?;
    Some((prefix, num))
}

fn next_free_namespaced_id(prefix: &str, used: &HashSet<String>) -> String {
    for n in 1..=999 {
        let candidate = format!("{}{:03}", prefix, n);
        if !used.contains(&candidate.to_lowercase()) {
            return candidate;
        }
    }
    // Extremely unlikely. Fallback to an unpadded suffix.
    let mut n = 1000;
    loop {
        let candidate = format!("{}{}", prefix, n);
        if !used.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        n += 1;
    }
}

fn next_free_legacy_dup_id(old_id: &str, used: &HashSet<String>) -> String {
    for n in 2..=999 {
        let candidate = format!("{}-dup{}", old_id, n);
        if !used.contains(&candidate.to_lowercase()) {
            return candidate;
        }
    }
    // Fallback
    let mut n = 1000;
    loop {
        let candidate = format!("{}-dup{}", old_id, n);
        if !used.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        n += 1;
    }
}

fn rename_task_file(
    old_path: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<PathBuf, TaskParseError> {
    let file_name = old_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let new_file_name = if file_name
        .to_lowercase()
        .starts_with(&format!("{} ", old_id.to_lowercase()))
        || file_name
            .to_lowercase()
            .starts_with(&format!("{}-", old_id.to_lowercase()))
        || file_name
            .to_lowercase()
            .starts_with(&format!("{}_", old_id.to_lowercase()))
        || file_name
            .to_lowercase()
            .starts_with(&format!("{}.", old_id.to_lowercase()))
        || file_name
            .to_lowercase()
            .starts_with(&format!("{}-", old_id.to_lowercase()))
        || file_name
            .to_lowercase()
            .starts_with(&format!("{} -", old_id.to_lowercase()))
        || file_name.to_lowercase().starts_with(&old_id.to_lowercase())
    {
        // Replace the leading id only.
        format!("{}{}", new_id, &file_name[old_id.len()..])
    } else {
        // If the filename doesn't start with the id, keep it unchanged.
        file_name
    };

    let new_path = old_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(new_file_name);
    if new_path != old_path {
        fs::rename(old_path, &new_path).map_err(|err| TaskParseError::Invalid(err.to_string()))?;
    }
    Ok(new_path)
}

pub fn fix_duplicate_task_ids(
    backlog_dir: &Path,
    tasks: &[Task],
    options: FixIdsOptions,
) -> Result<FixIdsReport, TaskParseError> {
    // Group tasks by lowercased id, but keep deterministic ordering inside each group.
    let mut groups: BTreeMap<String, Vec<&Task>> = BTreeMap::new();
    for task in tasks {
        if task.id.trim().is_empty() {
            continue;
        }
        groups.entry(task.id.to_lowercase()).or_default().push(task);
    }

    let mut used = lower_set(tasks);
    let mut changes = Vec::new();
    let mut warnings = Vec::new();

    for (id_lc, mut group) in groups {
        if group.len() <= 1 {
            continue;
        }
        group.sort_by_key(|task| {
            (
                task.uid.clone().unwrap_or_else(|| "~~~~".to_string()),
                task.file_path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string(),
            )
        });

        let keep = group[0];
        warnings.push(format!(
            "Duplicate id '{}' detected; keeping '{}' (uid={}) and rekeying {} other task(s). References to '{}' remain ambiguous and will continue to resolve to the kept task.",
            id_lc,
            keep.id,
            keep.uid.clone().unwrap_or_else(|| "(none)".to_string()),
            group.len() - 1,
            keep.id
        ));

        for task in group.into_iter().skip(1) {
            let old_id = task.id.clone();
            let Some(old_path) = task.file_path.as_ref() else {
                continue;
            };

            let new_id = if let Some((prefix, _num)) = parse_namespaced(&old_id) {
                next_free_namespaced_id(&prefix, &used)
            } else {
                next_free_legacy_dup_id(&old_id, &used)
            };
            used.insert(new_id.to_lowercase());

            let mut new_path = old_path.to_path_buf();
            if options.apply {
                // Update the task's own id.
                update_task_field(old_path, "id", Some(FieldValue::Scalar(new_id.clone())))?;

                // Keep the filename aligned with the id.
                new_path = rename_task_file(old_path, &old_id, &new_id)?;
            }

            changes.push(FixIdsChange {
                old_id,
                new_id,
                old_path: old_path.to_path_buf(),
                new_path,
                uid: task.uid.clone(),
            });
        }
    }

    // Sanity: ensure we didn't accidentally move outside the backlog dir.
    for change in &changes {
        if !change.new_path.starts_with(backlog_dir) {
            return Err(TaskParseError::Invalid(format!(
                "Refusing to write outside backlog dir: {}",
                change.new_path.display()
            )));
        }
    }

    Ok(FixIdsReport { changes, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::parse_task_file;
    use crate::task_ops::create_task_file;
    use tempfile::TempDir;

    fn mk_task(tasks_dir: &Path, id: &str, title: &str) -> PathBuf {
        create_task_file(tasks_dir, id, title, "To Do", "P2", "Phase1", &[], &[], &[])
            .expect("create task")
    }

    #[test]
    fn fix_duplicate_task_ids_dry_run_reports_changes() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path().join("workmesh");
        let tasks_dir = backlog_dir.join("tasks");
        fs::create_dir_all(&tasks_dir).expect("tasks dir");

        let a = mk_task(&tasks_dir, "task-login-001", "Alpha");
        let b = mk_task(&tasks_dir, "task-login-001", "Beta");

        let tasks = vec![
            parse_task_file(&a).expect("a"),
            parse_task_file(&b).expect("b"),
        ];
        let report = fix_duplicate_task_ids(&backlog_dir, &tasks, FixIdsOptions { apply: false })
            .expect("report");
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].old_id, "task-login-001");
        assert!(report.changes[0].new_id.starts_with("task-login-"));
    }

    #[test]
    fn fix_duplicate_task_ids_apply_changes_id_and_filename() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path().join("workmesh");
        let tasks_dir = backlog_dir.join("tasks");
        fs::create_dir_all(&tasks_dir).expect("tasks dir");

        let a = mk_task(&tasks_dir, "task-001", "Alpha");
        let b = mk_task(&tasks_dir, "task-001", "Beta");

        let tasks = vec![
            parse_task_file(&a).expect("a"),
            parse_task_file(&b).expect("b"),
        ];
        let report = fix_duplicate_task_ids(&backlog_dir, &tasks, FixIdsOptions { apply: true })
            .expect("apply");
        assert_eq!(report.changes.len(), 1);

        let changed = &report.changes[0];
        assert!(changed.new_id.starts_with("task-001-dup"));
        assert!(changed
            .new_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains(&changed.new_id));
    }
}
