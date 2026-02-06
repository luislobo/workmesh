use std::path::Path;

use tempfile::TempDir;

use workmesh_core::backlog::{locate_backlog_dir, resolve_backlog_dir};

fn create_tasks_dir(path: &Path) {
    std::fs::create_dir_all(path).expect("create tasks dir");
}

#[test]
fn resolve_backlog_dir_accepts_tasks_dir() {
    let temp = TempDir::new().expect("tempdir");
    let tasks_dir = temp.path().join("tasks");
    create_tasks_dir(&tasks_dir);

    let resolved = resolve_backlog_dir(&tasks_dir).expect("resolve");
    assert_eq!(resolved, temp.path());
}

#[test]
fn resolve_backlog_dir_accepts_backlog_dir() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    create_tasks_dir(&backlog_dir.join("tasks"));

    let resolved = resolve_backlog_dir(&backlog_dir).expect("resolve");
    assert_eq!(resolved, backlog_dir);
}

#[test]
fn resolve_backlog_dir_accepts_project_dir() {
    let temp = TempDir::new().expect("tempdir");
    let project_dir = temp.path().join("project");
    create_tasks_dir(&project_dir.join("tasks"));

    let resolved = resolve_backlog_dir(&project_dir).expect("resolve");
    assert_eq!(resolved, project_dir);
}

#[test]
fn locate_backlog_dir_finds_backlog_from_child() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    create_tasks_dir(&backlog_dir.join("tasks"));

    let child = backlog_dir.join("tasks");
    let resolved = locate_backlog_dir(&child).expect("resolve");
    // Windows can surface different path representations (verbatim prefix, 8.3 short names).
    // Compare canonical paths to avoid false negatives.
    let expected = std::fs::canonicalize(&backlog_dir).unwrap_or(backlog_dir);
    let actual = std::fs::canonicalize(&resolved).unwrap_or(resolved);
    assert_eq!(actual, expected);
}

#[test]
fn locate_backlog_dir_errors_when_missing() {
    let temp = TempDir::new().expect("tempdir");
    let err = locate_backlog_dir(temp.path());
    assert!(err.is_err());
}
