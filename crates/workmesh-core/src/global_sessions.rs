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
    #[serde(default)]
    pub epic_id: Option<String>,
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
    home_dir()
        .map(|home| home.join(".workmesh"))
        .ok_or_else(|| {
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

pub fn sessions_index_path(home: &Path) -> PathBuf {
    home.join(".index").join("sessions.jsonl")
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SessionsIndexSummary {
    pub indexed: usize,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SessionsIndexReport {
    pub ok: bool,
    pub missing_in_index: Vec<String>,
    pub extra_in_index: Vec<String>,
    pub mismatched: Vec<String>,
}

pub fn rebuild_sessions_index(home: &Path) -> Result<SessionsIndexSummary> {
    ensure_global_dirs(home)?;
    let sessions = load_sessions_latest(home)?;
    let index_path = sessions_index_path(home);
    let tmp = index_path.with_extension("jsonl.tmp");

    let mut file =
        fs::File::create(&tmp).with_context(|| format!("create temp index {}", tmp.display()))?;
    for session in &sessions {
        let line = serde_json::to_string(session).context("serialize session for index")?;
        writeln!(file, "{}", line).context("write index line")?;
    }
    fs::rename(&tmp, &index_path)
        .with_context(|| format!("rename {} -> {}", tmp.display(), index_path.display()))?;

    Ok(SessionsIndexSummary {
        indexed: sessions.len(),
        path: index_path.to_string_lossy().to_string(),
    })
}

pub fn refresh_sessions_index(home: &Path) -> Result<SessionsIndexSummary> {
    // Sessions are stored as append-only events; a cheap and correct refresh is a rebuild.
    rebuild_sessions_index(home)
}

pub fn load_sessions_latest_from_index(home: &Path) -> Result<Vec<AgentSession>> {
    let index = sessions_index_path(home);
    if !index.exists() {
        return Err(anyhow!(
            "Sessions index not found: {} (run session index-rebuild)",
            index.display()
        ));
    }
    let file = fs::File::open(&index).with_context(|| format!("open {}", index.display()))?;
    let reader = BufReader::new(file);

    let mut sessions = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("read line {}", idx + 1))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let session: AgentSession = serde_json::from_str(trimmed)
            .with_context(|| format!("parse session on line {}", idx + 1))?;
        sessions.push(session);
    }
    sessions.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(sessions)
}

pub fn load_sessions_latest_fast(home: &Path) -> Result<Vec<AgentSession>> {
    if sessions_index_path(home).exists() {
        if let Ok(sessions) = load_sessions_latest_from_index(home) {
            return Ok(sessions);
        }
    }
    load_sessions_latest(home)
}

pub fn verify_sessions_index(home: &Path) -> Result<SessionsIndexReport> {
    let source = load_sessions_latest(home)?;
    let indexed = match load_sessions_latest_from_index(home) {
        Ok(value) => value,
        Err(_) => Vec::new(),
    };

    let source_map: BTreeMap<String, AgentSession> =
        source.into_iter().map(|s| (s.id.clone(), s)).collect();
    let index_map: BTreeMap<String, AgentSession> =
        indexed.into_iter().map(|s| (s.id.clone(), s)).collect();

    let mut missing_in_index = Vec::new();
    let mut extra_in_index = Vec::new();
    let mut mismatched = Vec::new();

    for (id, session) in &source_map {
        match index_map.get(id) {
            None => missing_in_index.push(id.clone()),
            Some(indexed) => {
                if indexed.updated_at != session.updated_at || indexed.cwd != session.cwd {
                    mismatched.push(id.clone());
                }
            }
        }
    }

    for id in index_map.keys() {
        if !source_map.contains_key(id) {
            extra_in_index.push(id.clone());
        }
    }

    Ok(SessionsIndexReport {
        ok: missing_in_index.is_empty() && extra_in_index.is_empty() && mismatched.is_empty(),
        missing_in_index,
        extra_in_index,
        mismatched,
    })
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
