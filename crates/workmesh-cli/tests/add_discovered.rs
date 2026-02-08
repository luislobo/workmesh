use std::fs;
use std::process::Command;

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
fn add_discovered_sets_relationship() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha");

    let output = bin()
        .arg("--root")
        .arg(&backlog_dir)
        .arg("add-discovered")
        .arg("--from")
        .arg("task-001")
        .arg("--title")
        .arg("Found bug")
        .output()
        .expect("add discovered");
    assert!(output.status.success());

    // The exact next id is implementation-defined (it may be namespaced by initiative),
    // but the discovered_from relationship must be recorded.
    let created_content = fs::read_dir(&tasks_dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .map(|entry| fs::read_to_string(entry.path()).expect("read task"))
        .find(|content| content.contains("discovered_from: [task-001]"))
        .expect("created task content");
    assert!(created_content.contains("title: Found bug"));
}
