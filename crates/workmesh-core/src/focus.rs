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
