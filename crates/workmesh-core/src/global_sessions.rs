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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn session(id: &str, updated_at: &str, cwd: &str) -> AgentSession {
        AgentSession {
            id: id.to_string(),
            created_at: "2026-02-01T00:00:00Z".to_string(),
            updated_at: updated_at.to_string(),
            cwd: cwd.to_string(),
            repo_root: None,
            project_id: None,
            epic_id: None,
            objective: "ship".to_string(),
            working_set: vec!["task-001".to_string()],
            notes: None,
            git: None,
            checkpoint: None,
            recent_changes: None,
        }
    }

    #[test]
    fn read_current_session_id_returns_none_for_invalid_json() {
        let temp = TempDir::new().expect("tempdir");
        ensure_global_dirs(temp.path()).expect("dirs");
        fs::write(sessions_current_path(temp.path()), "not-json").expect("write");
        assert!(read_current_session_id(temp.path()).is_none());
    }

    #[test]
    fn load_sessions_latest_ignores_non_session_events_and_blank_lines() {
        let temp = TempDir::new().expect("tempdir");
        ensure_global_dirs(temp.path()).expect("dirs");
        let path = sessions_events_path(temp.path());
        append_jsonl_line(&path, "").expect("append blank");
        append_jsonl_line(&path, r#"{"type":"other","session":{"id":"x","created_at":"t","updated_at":"t","cwd":"c","objective":"o","working_set":[]}}"#)
            .expect("append other");
        append_session_saved(temp.path(), session("s1", "2026-02-01T01:00:00Z", "/tmp"))
            .expect("append");

        let sessions = load_sessions_latest(temp.path()).expect("load");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "s1");
    }

    #[test]
    fn verify_sessions_index_reports_missing_extra_and_mismatch() {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();

        append_session_saved(home, session("s1", "2026-02-01T01:00:00Z", "/a")).expect("append");
        append_session_saved(home, session("s2", "2026-02-01T02:00:00Z", "/b")).expect("append");
        rebuild_sessions_index(home).expect("rebuild");

        // Missing: drop s2 by rewriting index with only s1 + extra.
        let source = load_sessions_latest(home).expect("load source");
        let mut s1 = source.iter().find(|s| s.id == "s1").unwrap().clone();
        // Mismatch: keep the same id but change a field checked by verify.
        s1.cwd = "/changed".to_string();
        let rewritten = format!(
            "{}\n{}\n",
            serde_json::to_string(&s1).unwrap(),
            serde_json::to_string(&session("extra", "2026-02-01T03:00:00Z", "/x")).unwrap()
        );
        let index_path = sessions_index_path(home);
        fs::write(&index_path, rewritten).expect("write missing");

        let report = verify_sessions_index(home).expect("verify");
        assert!(!report.ok);
        assert!(report.missing_in_index.contains(&"s2".to_string()));
        assert!(report.extra_in_index.contains(&"extra".to_string()));
        assert!(report.mismatched.contains(&"s1".to_string()));
    }

    #[test]
    fn load_sessions_latest_fast_falls_back_when_index_is_corrupt() {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();

        append_session_saved(home, session("s1", "2026-02-01T01:00:00Z", "/a")).expect("append");
        rebuild_sessions_index(home).expect("rebuild");
        fs::write(sessions_index_path(home), "not-json\n").expect("corrupt");

        let sessions = load_sessions_latest_fast(home).expect("load fast");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "s1");
    }

    #[test]
    fn helpers_are_stable_and_refresh_is_a_rebuild() {
        // Keep env mutation serialized across tests.
        let _lock = ENV_LOCK.lock().expect("lock");

        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();

        // Basic helpers.
        assert!(!now_rfc3339().is_empty());
        assert!(!new_session_id().is_empty());
        assert!(sessions_events_path(home).ends_with("sessions/events.jsonl"));
        assert!(sessions_current_path(home).ends_with("sessions/current.json"));
        assert!(sessions_index_path(home).ends_with(".index/sessions.jsonl"));

        // WORKMESH_HOME empty should fall back to HOME.
        std::env::set_var("WORKMESH_HOME", "   ");
        let resolved = resolve_workmesh_home().expect("resolve");
        assert!(resolved.to_string_lossy().ends_with(".workmesh"));
        std::env::remove_var("WORKMESH_HOME");

        append_session_saved(home, session("s1", "2026-02-01T01:00:00Z", "/a")).expect("append");
        let rebuilt = rebuild_sessions_index(home).expect("rebuild");
        let refreshed = refresh_sessions_index(home).expect("refresh");
        assert_eq!(rebuilt.indexed, refreshed.indexed);
    }

    #[test]
    fn verify_sessions_index_reports_missing_when_index_is_absent() {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();

        append_session_saved(home, session("s1", "2026-02-01T01:00:00Z", "/a")).expect("append");
        let report = verify_sessions_index(home).expect("verify");
        assert!(!report.ok);
        assert!(report.missing_in_index.contains(&"s1".to_string()));
    }

    #[test]
    fn home_dir_prefers_home_then_userprofile_and_can_be_none() {
        let _lock = ENV_LOCK.lock().expect("lock");

        std::env::set_var("HOME", "/tmp/home-test");
        std::env::remove_var("USERPROFILE");
        assert_eq!(home_dir().unwrap(), PathBuf::from("/tmp/home-test"));

        std::env::set_var("HOME", "   ");
        std::env::set_var("USERPROFILE", "/tmp/profile-test");
        assert_eq!(home_dir().unwrap(), PathBuf::from("/tmp/profile-test"));

        std::env::set_var("HOME", "   ");
        std::env::set_var("USERPROFILE", "   ");
        assert!(home_dir().is_none());

        std::env::remove_var("HOME");
        std::env::remove_var("USERPROFILE");
    }

    #[test]
    fn session_saved_event_new_sets_type() {
        let event = SessionSavedEvent::new(session("s1", "2026-02-01T01:00:00Z", "/a"));
        assert_eq!(event.event_type, "session_saved");
        assert_eq!(event.session.id, "s1");
    }

    #[test]
    fn append_jsonl_line_creates_and_appends() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("events.jsonl");
        append_jsonl_line(&path, r#"{"a":1}"#).expect("append 1");
        append_jsonl_line(&path, r#"{"a":2}"#).expect("append 2");
        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains(r#"{"a":1}"#));
        assert!(content.contains(r#"{"a":2}"#));
    }
}
