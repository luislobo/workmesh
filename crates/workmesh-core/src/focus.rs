use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FocusState {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub epic_id: Option<String>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub working_set: Vec<String>,
    /// RFC3339 timestamp
    #[serde(default)]
    pub updated_at: Option<String>,
}

pub fn focus_path(backlog_dir: &Path) -> PathBuf {
    backlog_dir.join("focus.json")
}

pub fn load_focus(backlog_dir: &Path) -> Result<Option<FocusState>> {
    let path = focus_path(backlog_dir);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)?;
    let state: FocusState = serde_json::from_str(&raw)?;
    Ok(Some(state))
}

pub fn save_focus(backlog_dir: &Path, mut state: FocusState) -> Result<PathBuf> {
    state.updated_at = Some(now_rfc3339());
    let path = focus_path(backlog_dir);
    let raw = serde_json::to_string_pretty(&state)?;
    fs::write(&path, raw)?;
    Ok(path)
}

pub fn clear_focus(backlog_dir: &Path) -> Result<bool> {
    let path = focus_path(backlog_dir);
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path)?;
    Ok(true)
}

fn normalize_task_id(task_id: &str) -> String {
    task_id.trim().to_lowercase()
}

fn dedup_preserve_order(values: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    values.retain(|v| {
        let k = v.to_lowercase();
        if seen.contains(&k) {
            return false;
        }
        seen.insert(k);
        true
    });
}

/// Update focus.working_set based on a task mutation.
///
/// Rules (conservative, deterministic):
/// - If no focus is set, do nothing.
/// - If status becomes "In Progress", add task id.
/// - If status becomes "Done" or "To Do", remove task id.
/// - If a lease is active (owner non-empty), add task id.
/// - Otherwise, leave working_set unchanged.
///
/// Returns true if focus.json was modified.
pub fn update_focus_for_task_mutation(
    backlog_dir: &Path,
    task_id: &str,
    new_status: Option<&str>,
    lease_owner: Option<&str>,
) -> Result<bool> {
    let Some(mut focus) = load_focus(backlog_dir)? else {
        return Ok(false);
    };
    let id_norm = normalize_task_id(task_id);
    let mut changed = false;

    let status_lc = new_status.map(|s| s.trim().to_lowercase());
    let lease_active = lease_owner
        .map(|o| !o.trim().is_empty())
        .unwrap_or(false);

    let mut has_id = focus
        .working_set
        .iter()
        .any(|id| id.to_lowercase() == id_norm);

    if status_lc.as_deref() == Some("in progress") {
        if !has_id {
            focus.working_set.push(task_id.trim().to_string());
            has_id = true;
            changed = true;
        }
    } else if status_lc.as_deref() == Some("done") || status_lc.as_deref() == Some("to do") {
        let before = focus.working_set.len();
        focus.working_set.retain(|id| id.to_lowercase() != id_norm);
        if focus.working_set.len() != before {
            has_id = false;
            changed = true;
        }
    }

    if lease_active && !has_id {
        focus.working_set.push(task_id.trim().to_string());
        changed = true;
    }

    if changed {
        dedup_preserve_order(&mut focus.working_set);
        save_focus(backlog_dir, focus)?;
    }
    Ok(changed)
}

pub fn now_rfc3339() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.to_rfc3339()
}

pub fn infer_project_id(repo_root: &Path) -> Option<String> {
    let projects_dir = repo_root.join("docs").join("projects");
    let Ok(read_dir) = fs::read_dir(&projects_dir) else {
        return None;
    };
    let mut ids: Vec<String> = read_dir
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .collect();
    ids.sort();
    if ids.len() == 1 {
        Some(ids[0].clone())
    } else {
        None
    }
}

pub fn extract_task_id_from_branch(branch: &str) -> Option<String> {
    // Keep it simple and deterministic: accept the canonical `task-<digits>` form anywhere.
    let mut buf = String::new();
    let mut i = 0;
    let bytes = branch.as_bytes();
    while i + 5 < bytes.len() {
        if &bytes[i..i + 5] == b"task-" {
            buf.clear();
            buf.push_str("task-");
            let mut j = i + 5;
            let mut has_digit = false;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                buf.push(bytes[j] as char);
                has_digit = true;
                j += 1;
            }
            if has_digit {
                return Some(buf.clone());
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn update_focus_adds_on_in_progress_and_removes_on_done_and_dedups() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path();
        let focus = FocusState {
            project_id: None,
            epic_id: None,
            objective: None,
            working_set: vec!["task-001".to_string(), "TASK-001".to_string()],
            updated_at: None,
        };
        save_focus(backlog_dir, focus).expect("save");

        // In Progress should add if missing.
        update_focus_for_task_mutation(backlog_dir, "task-002", Some("In Progress"), None)
            .expect("update");
        let focus = load_focus(backlog_dir).expect("load").expect("present");
        assert!(focus.working_set.iter().any(|id| id == "task-002"));
        // Dedupe should have removed duplicates (case-insensitive).
        assert_eq!(
            focus.working_set
                .iter()
                .filter(|id| id.to_lowercase() == "task-001")
                .count(),
            1
        );

        // Done should remove.
        update_focus_for_task_mutation(backlog_dir, "task-002", Some("Done"), None).expect("rm");
        let focus = load_focus(backlog_dir).expect("load").expect("present");
        assert!(!focus.working_set.iter().any(|id| id.to_lowercase() == "task-002"));
    }

    #[test]
    fn update_focus_adds_on_active_lease() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path();
        let focus = FocusState {
            project_id: None,
            epic_id: None,
            objective: None,
            working_set: vec![],
            updated_at: None,
        };
        save_focus(backlog_dir, focus).expect("save");

        update_focus_for_task_mutation(backlog_dir, "task-123", None, Some("me")).expect("add");
        let focus = load_focus(backlog_dir).expect("load").expect("present");
        assert_eq!(focus.working_set, vec!["task-123".to_string()]);
    }
}
