use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Failed to write audit log: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to serialize audit event: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Serialize)]
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
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(event)?;
    writeln!(file, "{}", line)?;
    Ok(())
}
