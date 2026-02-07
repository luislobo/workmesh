use std::fs;
use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;

use workmesh_core::index::{index_path, rebuild_index, refresh_index, verify_index};

fn write_task(tasks_dir: &Path, id: &str, title: &str) {
    let content = format!(
        "---\n\
id: {id}\n\
title: {title}\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: [core]\n\
---\n",
        id = id,
        title = title
    );
    let filename = format!("{id} - {title}.md", id = id, title = title);
    fs::write(tasks_dir.join(filename), content).expect("write task");
}

fn read_index_hash(path: &Path, id: &str) -> String {
    let data = fs::read_to_string(path).expect("read index");
    for line in data.lines() {
        let value: Value = serde_json::from_str(line).expect("parse");
        if value.get("id").and_then(|v| v.as_str()) == Some(id) {
            return value
                .get("hash")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
        }
    }
    String::new()
}

fn read_index_path(path: &Path, id: &str) -> String {
    let data = fs::read_to_string(path).expect("read index");
    for line in data.lines() {
        let value: Value = serde_json::from_str(line).expect("parse");
        if value.get("id").and_then(|v| v.as_str()) == Some(id) {
            return value
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
        }
    }
    String::new()
}

#[test]
fn rebuild_refresh_verify_index() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha");
    write_task(&tasks_dir, "task-002", "Beta");

    let summary = rebuild_index(&backlog_dir).expect("rebuild");
    assert_eq!(summary.entries, 2);

    let index_file = index_path(&backlog_dir);
    let indexed_path = read_index_path(&index_file, "task-001");
    assert!(!indexed_path.is_empty());
    let temp_prefix = temp.path().to_string_lossy().to_string();
    assert!(
        !indexed_path.contains(&temp_prefix),
        "expected repo-relative path, got {}",
        indexed_path
    );
    let original_hash = read_index_hash(&index_file, "task-001");
    assert!(!original_hash.is_empty());

    let task_path = tasks_dir.join("task-001 - Alpha.md");
    let mut content = fs::read_to_string(&task_path).expect("read task");
    content.push_str("\nNotes:\n- Updated\n");
    fs::write(&task_path, content).expect("write task");

    let report = verify_index(&backlog_dir).expect("verify");
    assert!(!report.ok);
    assert!(report.stale.iter().any(|path| path.contains("task-001")));

    let summary = refresh_index(&backlog_dir).expect("refresh");
    assert_eq!(summary.entries, 2);

    let refreshed_hash = read_index_hash(&index_file, "task-001");
    assert_ne!(original_hash, refreshed_hash);

    let report = verify_index(&backlog_dir).expect("verify");
    assert!(report.ok);
}
