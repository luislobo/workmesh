use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

#[test]
fn best_practices_mentions_derived_files() {
    let temp = TempDir::new().expect("tempdir");
    // The CLI currently resolves tasks before dispatching commands, so create a minimal task.
    let tasks_dir = temp.path().join("workmesh").join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");
    fs::write(
        tasks_dir.join("task-001 - seed.md"),
        "---\n\
id: task-001\n\
title: Seed\n\
kind: task\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
---\n",
    )
    .expect("write task");
    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("best-practices")
        .output()
        .expect("run best-practices");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(text.contains("Derived files"));
    assert!(text.contains("workmesh/.index/"));
}
