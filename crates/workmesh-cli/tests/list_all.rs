use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn write_task(dir: &std::path::Path, id: &str, title: &str, status: &str) {
    let filename = format!("{} - {}.md", id, title.to_lowercase());
    let path = dir.join(filename);
    let content = format!(
        "---\n\
id: {id}\n\
title: {title}\n\
kind: task\n\
status: {status}\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
---\n\n\
Body\n",
        id = id,
        title = title,
        status = status
    );
    std::fs::write(path, content).expect("write task");
}

#[test]
fn list_all_includes_archived_done_tasks() {
    let repo = TempDir::new().expect("repo");
    let backlog_dir = repo.path().join("workmesh");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");

    // Create one Done task, then archive it so it moves out of workmesh/tasks/.
    write_task(&tasks_dir, "task-001", "Alpha", "Done");

    let archive = bin()
        .arg("--root")
        .arg(repo.path())
        .arg("archive")
        .arg("--before")
        .arg("2100-01-01")
        .output()
        .expect("archive");
    assert!(archive.status.success());

    // Default list should not include archived tasks (because it only loads workmesh/tasks/).
    let list_active = bin()
        .arg("--root")
        .arg(repo.path())
        .arg("list")
        .arg("--json")
        .output()
        .expect("list");
    assert!(list_active.status.success());
    let active: Value = serde_json::from_slice(&list_active.stdout).expect("json");
    assert_eq!(active.as_array().unwrap().len(), 0);

    // --all should include archived tasks.
    let list_all = bin()
        .arg("--root")
        .arg(repo.path())
        .arg("list")
        .arg("--all")
        .arg("--json")
        .output()
        .expect("list --all");
    assert!(list_all.status.success());
    let all: Value = serde_json::from_slice(&list_all.stdout).expect("json");
    let ids: Vec<String> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.get("id").unwrap().as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"task-001".to_string()));
}

