use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn write_task(tasks_dir: &std::path::Path, id: &str, title: &str) {
    let content = format!(
        "---\n\
id: {id}\n\
title: {title}\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
---\n",
        id = id,
        title = title
    );
    let filename = format!("{id} - {title}.md", id = id, title = title);
    fs::write(tasks_dir.join(filename), content).expect("write task");
}

fn read_index_status(index_path: &std::path::Path, id: &str) -> String {
    let data = fs::read_to_string(index_path).expect("read index");
    for line in data.lines() {
        let value: Value = serde_json::from_str(line).expect("parse jsonl");
        if value.get("id").and_then(|v| v.as_str()) == Some(id) {
            return value
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
        }
    }
    String::new()
}

#[test]
fn mutating_commands_refresh_index() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha");

    let output = bin()
        .arg("--root")
        .arg(&backlog_dir)
        .arg("index-rebuild")
        .output()
        .expect("index rebuild");
    assert!(output.status.success());

    let output = bin()
        .arg("--root")
        .arg(&backlog_dir)
        .arg("set-status")
        .arg("task-001")
        .arg("Done")
        .output()
        .expect("set status");
    assert!(output.status.success());

    let index_path = backlog_dir.join(".index").join("tasks.jsonl");
    let status = read_index_status(&index_path, "task-001");
    assert_eq!(status, "Done");
}
