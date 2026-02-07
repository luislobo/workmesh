use tempfile::TempDir;

use workmesh_core::focus::{
    clear_focus, extract_task_id_from_branch, infer_project_id, load_focus, save_focus, FocusState,
};

#[test]
fn focus_round_trip_save_load_clear() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("workmesh");
    std::fs::create_dir_all(&backlog_dir).expect("backlog dir");

    assert!(load_focus(&backlog_dir).expect("load").is_none());

    let state = FocusState {
        project_id: Some("alpha".to_string()),
        epic_id: Some("task-039".to_string()),
        objective: Some("Ship focus feature".to_string()),
        working_set: vec!["task-052".to_string(), "task-053".to_string()],
        updated_at: None,
    };
    save_focus(&backlog_dir, state.clone()).expect("save");

    let loaded = load_focus(&backlog_dir).expect("load").expect("some");
    assert_eq!(loaded.project_id, state.project_id);
    assert_eq!(loaded.epic_id, state.epic_id);
    assert_eq!(loaded.objective, state.objective);
    assert_eq!(loaded.working_set, state.working_set);
    assert!(loaded.updated_at.is_some());

    assert!(clear_focus(&backlog_dir).expect("clear"));
    assert!(load_focus(&backlog_dir).expect("load").is_none());
}

#[test]
fn infer_project_id_returns_only_project_when_singleton() {
    let temp = TempDir::new().expect("tempdir");
    let repo_root = temp.path();
    std::fs::create_dir_all(repo_root.join("docs").join("projects").join("alpha").join("updates"))
        .expect("docs");
    assert_eq!(infer_project_id(repo_root), Some("alpha".to_string()));
}

#[test]
fn extract_task_id_from_branch_finds_task_id_anywhere() {
    assert_eq!(
        extract_task_id_from_branch("feature/task-123-focus"),
        Some("task-123".to_string())
    );
    assert_eq!(
        extract_task_id_from_branch("task-9"),
        Some("task-9".to_string())
    );
    assert_eq!(extract_task_id_from_branch("nope"), None);
    assert_eq!(extract_task_id_from_branch("task-"), None);
}

