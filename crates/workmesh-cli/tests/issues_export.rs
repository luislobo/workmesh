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

#[test]
fn issues_export_outputs_jsonl() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha");
    write_task(&tasks_dir, "task-002", "Beta");

    let output = bin()
        .arg("--root")
        .arg(&backlog_dir)
        .arg("issues-export")
        .output()
        .expect("issues export");
    assert!(output.status.success());

    let text = String::from_utf8(output.stdout).expect("utf8");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);

    let first: Value = serde_json::from_str(lines[0]).expect("json line");
    assert_eq!(first.get("id").and_then(|v| v.as_str()), Some("task-001"));
}
