use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContextScopeMode {
    None,
    Epic,
    Tasks,
}

impl Default for ContextScopeMode {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ContextScope {
    #[serde(default)]
    pub mode: ContextScopeMode,
    #[serde(default)]
    pub epic_id: Option<String>,
    #[serde(default)]
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextState {
    #[serde(default = "default_context_version")]
    pub version: u32,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub scope: ContextScope,
    /// RFC3339 timestamp
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl Default for ContextState {
    fn default() -> Self {
        Self {
            version: default_context_version(),
            project_id: None,
            objective: None,
            scope: ContextScope::default(),
            updated_at: None,
        }
    }
}

fn default_context_version() -> u32 {
    1
}

pub fn context_path(backlog_dir: &Path) -> PathBuf {
    backlog_dir.join("context.json")
}

pub fn load_context(backlog_dir: &Path) -> Result<Option<ContextState>> {
    let path = context_path(backlog_dir);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)?;
    let state: ContextState = serde_json::from_str(&raw)?;
    Ok(Some(state))
}

pub fn save_context(backlog_dir: &Path, mut state: ContextState) -> Result<PathBuf> {
    normalize_scope(&mut state.scope);
    state.version = default_context_version();
    state.updated_at = Some(now_rfc3339());
    let path = context_path(backlog_dir);
    let raw = serde_json::to_string_pretty(&state)?;
    fs::write(&path, raw)?;
    Ok(path)
}

pub fn clear_context(backlog_dir: &Path) -> Result<bool> {
    let path = context_path(backlog_dir);
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path)?;
    Ok(true)
}

pub fn context_from_legacy_focus(
    project_id: Option<String>,
    epic_id: Option<String>,
    objective: Option<String>,
    task_ids: Vec<String>,
) -> ContextState {
    let mut state = ContextState {
        version: default_context_version(),
        project_id,
        objective,
        scope: ContextScope {
            mode: ContextScopeMode::None,
            epic_id,
            task_ids,
        },
        updated_at: None,
    };
    if state
        .scope
        .epic_id
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        state.scope.mode = ContextScopeMode::Epic;
        state.scope.task_ids.clear();
    } else if state.scope.task_ids.iter().any(|id| !id.trim().is_empty()) {
        state.scope.mode = ContextScopeMode::Tasks;
        state.scope.epic_id = None;
    }
    normalize_scope(&mut state.scope);
    state
}

fn normalize_scope(scope: &mut ContextScope) {
    match scope.mode {
        ContextScopeMode::None => {
            scope.epic_id = None;
            scope.task_ids.clear();
        }
        ContextScopeMode::Epic => {
            scope.epic_id = scope
                .epic_id
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            scope.task_ids.clear();
            if scope.epic_id.is_none() {
                scope.mode = ContextScopeMode::None;
            }
        }
        ContextScopeMode::Tasks => {
            scope.epic_id = None;
            let mut seen = std::collections::HashSet::new();
            let mut out = Vec::new();
            for raw in scope.task_ids.iter() {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let key = trimmed.to_lowercase();
                if seen.insert(key) {
                    out.push(trimmed.to_string());
                }
            }
            scope.task_ids = out;
            if scope.task_ids.is_empty() {
                scope.mode = ContextScopeMode::None;
            }
        }
    }
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
    fn save_context_normalizes_scope() {
        let temp = TempDir::new().expect("tempdir");
        let backlog = temp.path();
        let path = save_context(
            backlog,
            ContextState {
                version: 99,
                project_id: Some("demo".to_string()),
                objective: Some("ship".to_string()),
                scope: ContextScope {
                    mode: ContextScopeMode::Tasks,
                    epic_id: Some("task-main-001".to_string()),
                    task_ids: vec![
                        "task-001".to_string(),
                        "TASK-001".to_string(),
                        " ".to_string(),
                        "task-002".to_string(),
                    ],
                },
                updated_at: None,
            },
        )
        .expect("save");
        assert!(path.ends_with("context.json"));

        let loaded = load_context(backlog).expect("load").expect("present");
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.scope.mode, ContextScopeMode::Tasks);
        assert!(loaded.scope.epic_id.is_none());
        assert_eq!(
            loaded.scope.task_ids,
            vec!["task-001".to_string(), "task-002".to_string()]
        );
        assert!(loaded.updated_at.is_some());
    }

    #[test]
    fn context_from_legacy_focus_prefers_epic_scope() {
        let state = context_from_legacy_focus(
            Some("demo".to_string()),
            Some("task-main-010".to_string()),
            Some("ship".to_string()),
            vec!["task-main-011".to_string()],
        );
        assert_eq!(state.scope.mode, ContextScopeMode::Epic);
        assert_eq!(state.scope.epic_id.as_deref(), Some("task-main-010"));
        assert!(state.scope.task_ids.is_empty());
    }
}
