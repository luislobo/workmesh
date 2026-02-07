use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GitSnapshot {
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub dirty: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CheckpointRef {
    pub path: String,
    pub timestamp: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RecentChanges {
    pub dirs: Vec<String>,
    pub files: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AgentSession {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub cwd: String,
    pub repo_root: Option<String>,
    pub project_id: Option<String>,
    pub objective: String,
    pub working_set: Vec<String>,
    pub notes: Option<String>,
    pub git: Option<GitSnapshot>,
    pub checkpoint: Option<CheckpointRef>,
    pub recent_changes: Option<RecentChanges>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SessionSavedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub session: AgentSession,
}

impl SessionSavedEvent {
    pub fn new(session: AgentSession) -> Self {
        Self {
            event_type: "session_saved".to_string(),
            session,
        }
    }
}

pub fn now_rfc3339() -> String {
    let now: DateTime<Local> = Local::now();
    now.to_rfc3339()
}

pub fn new_session_id() -> String {
    Ulid::new().to_string()
}

pub fn resolve_workmesh_home() -> Result<PathBuf> {
    if let Ok(value) = std::env::var("WORKMESH_HOME") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    home_dir().map(|home| home.join(".workmesh")).ok_or_else(|| {
        anyhow!("Unable to resolve home directory; set WORKMESH_HOME to an absolute path")
    })
}

fn home_dir() -> Option<PathBuf> {
    // Minimal, dependency-free home resolution for common platforms.
    if let Ok(home) = std::env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let trimmed = profile.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    None
}

pub fn sessions_events_path(home: &Path) -> PathBuf {
    home.join("sessions").join("events.jsonl")
}

pub fn sessions_current_path(home: &Path) -> PathBuf {
    home.join("sessions").join("current.json")
}

pub fn ensure_global_dirs(home: &Path) -> Result<()> {
    fs::create_dir_all(home.join("sessions"))
        .with_context(|| format!("create sessions dir under {}", home.display()))?;
    fs::create_dir_all(home.join(".index"))
        .with_context(|| format!("create index dir under {}", home.display()))?;
    Ok(())
}

pub fn append_session_saved(home: &Path, session: AgentSession) -> Result<()> {
    ensure_global_dirs(home)?;
    let event = SessionSavedEvent::new(session);
    let line = serde_json::to_string(&event).context("serialize session_saved event")?;
    append_jsonl_line(&sessions_events_path(home), &line)
}

pub fn set_current_session(home: &Path, session_id: &str) -> Result<()> {
    ensure_global_dirs(home)?;
    let payload = serde_json::json!({
        "current_session_id": session_id,
        "updated_at": now_rfc3339(),
    });
    fs::write(
        sessions_current_path(home),
        serde_json::to_string_pretty(&payload)?,
    )
    .context("write current session pointer")?;
    Ok(())
}

pub fn read_current_session_id(home: &Path) -> Option<String> {
    let path = sessions_current_path(home);
    let contents = fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&contents).ok()?;
    value
        .get("current_session_id")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
}

pub fn load_sessions_latest(home: &Path) -> Result<Vec<AgentSession>> {
    let path = sessions_events_path(home);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut latest: BTreeMap<String, AgentSession> = BTreeMap::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("read line {}", idx + 1))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event: SessionSavedEvent = serde_json::from_str(trimmed)
            .with_context(|| format!("parse session event on line {}", idx + 1))?;
        if event.event_type != "session_saved" {
            continue;
        }
        latest.insert(event.session.id.clone(), event.session);
    }

    let mut sessions: Vec<AgentSession> = latest.into_values().collect();
    sessions.sort_by(|a, b| {
        // Sort by updated_at descending, then id ascending for stability.
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(sessions)
}

fn append_jsonl_line(path: &Path, line: &str) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open for append {}", path.display()))?;
    writeln!(file, "{}", line).context("append jsonl line")?;
    Ok(())
}

