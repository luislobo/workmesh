use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::storage::{
    append_jsonl_locked_with_key, atomic_write_text, cas_update_json_with_key, read_jsonl_tolerant,
    read_versioned_or_legacy_json, truncate_jsonl_trailing_invalid, with_resource_lock,
    ResourceKey, StorageError, DEFAULT_LOCK_TIMEOUT,
};

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
pub struct WorktreeBinding {
    #[serde(default)]
    pub id: Option<String>,
    pub path: String,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub repo_root: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct HandoffSummary {
    #[serde(default)]
    pub completed: Vec<String>,
    #[serde(default)]
    pub remaining: Vec<String>,
    #[serde(default)]
    pub decisions: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
    #[serde(default)]
    pub next_step: Option<String>,
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
    #[serde(default)]
    pub handoff: Option<HandoffSummary>,
    #[serde(default)]
    pub worktree: Option<WorktreeBinding>,
    #[serde(default)]
    pub truth_refs: Vec<String>,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct CurrentSessionPointerPayload {
    pub current_session_id: String,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[cfg(test)]
type CurrentSessionPointerState = crate::storage::VersionedState<CurrentSessionPointerPayload>;

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
    let path = sessions_events_path(home);
    append_jsonl_locked_with_key(&path, &line, &global_lock_key(home, "sessions.events"))
        .with_context(|| format!("append jsonl line {}", path.display()))?;
    Ok(())
}

pub fn set_current_session(home: &Path, session_id: &str) -> Result<()> {
    ensure_global_dirs(home)?;
    let path = sessions_current_path(home);
    let resource_key = global_lock_key(home, "sessions.current");
    let payload = CurrentSessionPointerPayload {
        current_session_id: session_id.trim().to_string(),
        updated_at: Some(now_rfc3339()),
    };
    cas_retry_set_current_session(&path, &resource_key, payload)
        .context("write current session pointer")?;
    Ok(())
}

pub fn read_current_session_id(home: &Path) -> Option<String> {
    let path = sessions_current_path(home);
    let state = read_versioned_or_legacy_json::<CurrentSessionPointerPayload>(&path).ok()??;
    Some(state.payload.current_session_id)
}

pub fn load_sessions_latest(home: &Path) -> Result<Vec<AgentSession>> {
    let path = sessions_events_path(home);
    let parsed = read_jsonl_tolerant::<SessionSavedEvent>(&path)
        .with_context(|| format!("read session events from {}", path.display()))?;
    let mut latest: BTreeMap<String, AgentSession> = BTreeMap::new();
    for event in parsed.records {
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

pub fn recover_sessions_events(home: &Path) -> Result<usize> {
    let path = sessions_events_path(home);
    let trimmed = truncate_jsonl_trailing_invalid(&path).map_err(anyhow::Error::from)?;
    Ok(trimmed)
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
    let mut lines = Vec::with_capacity(sessions.len());
    for session in &sessions {
        lines.push(serde_json::to_string(session).context("serialize session for index")?);
    }
    let payload = if lines.is_empty() {
        String::new()
    } else {
        let mut body = lines.join("\n");
        body.push('\n');
        body
    };
    let key = global_lock_key(home, "sessions.index");
    with_resource_lock(&key, DEFAULT_LOCK_TIMEOUT, || {
        atomic_write_text(&index_path, &payload)?;
        Ok(())
    })
    .map_err(anyhow::Error::from)
    .with_context(|| format!("write {}", index_path.display()))?;

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

fn global_lock_key(home: &Path, resource: &str) -> ResourceKey {
    ResourceKey::global(home, resource)
}

fn cas_retry_set_current_session(
    path: &Path,
    resource_key: &ResourceKey,
    payload: CurrentSessionPointerPayload,
) -> Result<()> {
    const MAX_RETRIES: usize = 8;
    let mut attempts = 0usize;
    loop {
        attempts += 1;
        let expected_version = read_versioned_or_legacy_json::<CurrentSessionPointerPayload>(path)
            .map_err(anyhow::Error::from)?
            .map(|state| state.version)
            .unwrap_or(0);

        match cas_update_json_with_key(path, resource_key, expected_version, payload.clone()) {
            Ok(_) => return Ok(()),
            Err(StorageError::Conflict(_)) if attempts < MAX_RETRIES => continue,
            Err(err) => return Err(anyhow!(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use tempfile::TempDir;

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
            handoff: None,
            worktree: None,
            truth_refs: Vec::new(),
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
        fs::write(
            &path,
            "\n{\"type\":\"other\",\"session\":{\"id\":\"x\",\"created_at\":\"t\",\"updated_at\":\"t\",\"cwd\":\"c\",\"objective\":\"o\",\"working_set\":[]}}\n",
        )
        .expect("seed events");
        append_session_saved(temp.path(), session("s1", "2026-02-01T01:00:00Z", "/tmp"))
            .expect("append");

        let sessions = load_sessions_latest(temp.path()).expect("load");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "s1");
    }

    #[test]
    fn load_sessions_latest_tolerates_trailing_partial_line_and_recovery_trims_it() {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();
        append_session_saved(home, session("s1", "2026-02-01T01:00:00Z", "/a")).expect("append");
        let path = sessions_events_path(home);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open append")
            .write_all(b"{")
            .expect("append partial");

        let sessions = load_sessions_latest(home).expect("load tolerant");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "s1");

        let trimmed = recover_sessions_events(home).expect("recover");
        assert_eq!(trimmed, 1);
        let repaired = std::fs::read_to_string(path).expect("read repaired");
        assert!(repaired.ends_with('\n'));
        assert!(!repaired.ends_with("{"));
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
        let _lock = crate::test_env::lock();

        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();
        let fake_home = temp.path().join("home");
        fs::create_dir_all(&fake_home).expect("create fake home");

        // Basic helpers.
        assert!(!now_rfc3339().is_empty());
        assert!(!new_session_id().is_empty());
        assert!(sessions_events_path(home).ends_with("sessions/events.jsonl"));
        assert!(sessions_current_path(home).ends_with("sessions/current.json"));
        assert!(sessions_index_path(home).ends_with(".index/sessions.jsonl"));

        // WORKMESH_HOME empty should fall back to HOME.
        std::env::set_var("HOME", &fake_home);
        std::env::remove_var("USERPROFILE");
        std::env::set_var("WORKMESH_HOME", "   ");
        let resolved = resolve_workmesh_home().expect("resolve");
        assert!(resolved.to_string_lossy().ends_with(".workmesh"));
        std::env::remove_var("WORKMESH_HOME");
        std::env::remove_var("HOME");

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
        let _lock = crate::test_env::lock();

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
    fn set_current_session_migrates_legacy_snapshot() {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();
        ensure_global_dirs(home).expect("dirs");
        fs::write(
            sessions_current_path(home),
            r#"{"current_session_id":"legacy","updated_at":"2026-02-01T00:00:00Z"}"#,
        )
        .expect("seed legacy current");

        set_current_session(home, "s2").expect("set");
        let stored = fs::read_to_string(sessions_current_path(home)).expect("read");
        let parsed: CurrentSessionPointerState =
            serde_json::from_str(&stored).expect("versioned state");
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.payload.current_session_id, "s2");
        assert_eq!(read_current_session_id(home).as_deref(), Some("s2"));
    }

    #[test]
    fn append_session_saved_is_lock_safe_under_parallel_writers() {
        let temp = TempDir::new().expect("tempdir");
        let home = Arc::new(temp.path().to_path_buf());
        let workers = 8usize;
        let barrier = Arc::new(Barrier::new(workers));
        let mut handles = Vec::new();
        for i in 0..workers {
            let home = Arc::clone(&home);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                append_session_saved(
                    home.as_path(),
                    session(
                        &format!("s{}", i),
                        &format!("2026-02-01T01:{:02}:00Z", i),
                        &format!("/{}", i),
                    ),
                )
                .expect("append session");
            }));
        }
        for handle in handles {
            handle.join().expect("join");
        }

        let sessions = load_sessions_latest(home.as_path()).expect("load");
        assert_eq!(sessions.len(), workers);
    }
}
