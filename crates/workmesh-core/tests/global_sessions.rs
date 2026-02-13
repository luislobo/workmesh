use std::fs;

use workmesh_core::global_sessions::{
    append_session_saved, load_sessions_latest, new_session_id, now_rfc3339,
    rebuild_sessions_index, resolve_workmesh_home, set_current_session, verify_sessions_index,
    AgentSession,
};

fn temp_home() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn workmesh_home_prefers_env_var() {
    let dir = temp_home();
    std::env::set_var("WORKMESH_HOME", dir.path());
    let resolved = resolve_workmesh_home().expect("resolve workmesh home");
    assert_eq!(resolved, dir.path());
}

#[test]
fn append_and_load_sessions_returns_latest_snapshots() {
    let dir = temp_home();
    let home = dir.path();

    let id1 = new_session_id();
    let id2 = new_session_id();

    let s1 = AgentSession {
        id: id1.clone(),
        created_at: now_rfc3339(),
        updated_at: "2026-01-01T10:00:00-08:00".to_string(),
        cwd: "/repo/a".to_string(),
        repo_root: None,
        project_id: None,
        epic_id: None,
        objective: "Do thing A".to_string(),
        working_set: vec!["task-001".to_string()],
        notes: None,
        git: None,
        checkpoint: None,
        recent_changes: None,
        handoff: None,
        worktree: None,
    };
    let s2 = AgentSession {
        id: id2.clone(),
        created_at: now_rfc3339(),
        updated_at: "2026-01-02T10:00:00-08:00".to_string(),
        cwd: "/repo/b".to_string(),
        repo_root: None,
        project_id: None,
        epic_id: None,
        objective: "Do thing B".to_string(),
        working_set: vec![],
        notes: None,
        git: None,
        checkpoint: None,
        recent_changes: None,
        handoff: None,
        worktree: None,
    };

    append_session_saved(home, s1).expect("append s1");
    append_session_saved(home, s2).expect("append s2");

    let sessions = load_sessions_latest(home).expect("load sessions");
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].id, id2);
    assert_eq!(sessions[1].id, id1);
}

#[test]
fn set_current_session_writes_pointer_file() {
    let dir = temp_home();
    let home = dir.path();

    set_current_session(home, "01TESTSESSION").expect("set current");
    let path = home.join("sessions").join("current.json");
    let contents = fs::read_to_string(path).expect("read current.json");
    assert!(contents.contains("01TESTSESSION"));
}

#[test]
fn rebuild_and_verify_sessions_index() {
    let dir = temp_home();
    let home = dir.path();

    let session = AgentSession {
        id: new_session_id(),
        created_at: now_rfc3339(),
        updated_at: "2026-01-03T10:00:00-08:00".to_string(),
        cwd: "/repo/c".to_string(),
        repo_root: None,
        project_id: None,
        epic_id: None,
        objective: "Do thing C".to_string(),
        working_set: vec![],
        notes: None,
        git: None,
        checkpoint: None,
        recent_changes: None,
        handoff: None,
        worktree: None,
    };
    append_session_saved(home, session).expect("append");

    let summary = rebuild_sessions_index(home).expect("rebuild");
    assert!(summary.indexed >= 1);

    let report = verify_sessions_index(home).expect("verify");
    assert!(report.ok, "expected ok, got {:?}", report);
}
