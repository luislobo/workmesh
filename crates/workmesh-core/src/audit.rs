use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::storage::{append_jsonl_locked_with_key, ResourceKey, StorageError};

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Failed to write audit log: {0}")]
    Storage(#[from] StorageError),
    #[error("Failed to serialize audit event: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditEvent {
    pub timestamp: String,
    pub actor: Option<String>,
    pub action: String,
    pub task_id: Option<String>,
    pub details: Value,
}

pub fn audit_log_path(backlog_dir: &Path) -> PathBuf {
    backlog_dir.join(".audit.log")
}

pub fn append_audit_event(backlog_dir: &Path, event: &AuditEvent) -> Result<(), AuditError> {
    let path = audit_log_path(backlog_dir);
    let line = serde_json::to_string(event)?;
    append_jsonl_locked_with_key(
        &path,
        &line,
        &ResourceKey::repo_local(backlog_dir, "audit.log"),
    )?;
    Ok(())
}

pub fn read_recent_audit_events(backlog_dir: &Path, limit: usize) -> Vec<AuditEvent> {
    if limit == 0 {
        return Vec::new();
    }
    let path = audit_log_path(backlog_dir);
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    let mut events = Vec::new();
    for line in content.lines() {
        if let Ok(event) = serde_json::from_str::<AuditEvent>(line) {
            events.push(event);
        }
    }
    if events.len() <= limit {
        return events;
    }
    events.split_off(events.len() - limit)
}
