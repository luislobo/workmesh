use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn write_task(tasks_dir: &std::path::Path, id: &str, title: &str, status: &str) {
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
---\n\
\n\
## Notes\n\
- seed\n",
        id = id,
        title = title,
        status = status
    );
    let filename = format!("{id} - {title}.md", id = id, title = title);
    fs::write(tasks_dir.join(filename), content).expect("write task");
}

#[test]
fn common_read_commands_smoke() {
    let temp = TempDir::new().expect("tempdir");
    let tasks_dir = temp.path().join("workmesh").join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");

    // Minimal project docs so resume/checkpoint paths don't error.
    let project_dir = temp.path().join("docs").join("projects").join("alpha");
    fs::create_dir_all(project_dir.join("updates")).expect("updates dir");

    write_task(&tasks_dir, "task-001", "Alpha", "To Do");
    write_task(&tasks_dir, "task-002", "Beta", "Done");

    // list (json)
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("list")
        .arg("--json")
        .output()
        .expect("list");
    assert!(out.status.success());

    // show (json)
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("task-001")
        .arg("--json")
        .output()
        .expect("show");
    assert!(out.status.success());

    // stats (json)
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("stats")
        .arg("--json")
        .output()
        .expect("stats");
    assert!(out.status.success());

    // export (json)
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("export")
        .arg("--pretty")
        .output()
        .expect("export");
    assert!(out.status.success());

    // graph-export (json)
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("graph-export")
        .arg("--pretty")
        .output()
        .expect("graph-export");
    assert!(out.status.success());

    // index verify should be ok after rebuild
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("index-rebuild")
        .output()
        .expect("index-rebuild");
    assert!(out.status.success());
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("index-verify")
        .arg("--json")
        .output()
        .expect("index-verify");
    assert!(out.status.success());

    // checkpoint + resume (text)
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("checkpoint")
        .arg("--project")
        .arg("alpha")
        .output()
        .expect("checkpoint");
    assert!(out.status.success());
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("resume")
        .arg("--project")
        .arg("alpha")
        .output()
        .expect("resume");
    assert!(out.status.success());
}

#[test]
fn common_write_commands_smoke() {
    let temp = TempDir::new().expect("tempdir");
    let tasks_dir = temp.path().join("workmesh").join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");
    write_task(&tasks_dir, "task-001", "Alpha", "To Do");

    // label add/remove
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("label-add")
        .arg("task-001")
        .arg("docs")
        .output()
        .expect("label-add");
    assert!(out.status.success());
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("label-remove")
        .arg("task-001")
        .arg("docs")
        .output()
        .expect("label-remove");
    assert!(out.status.success());

    // note
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("note")
        .arg("task-001")
        .arg("hello")
        .output()
        .expect("note");
    assert!(out.status.success());

    // set-field
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("set-field")
        .arg("task-001")
        .arg("priority")
        .arg("P1")
        .output()
        .expect("set-field");
    assert!(out.status.success());

    // claim + release
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("claim")
        .arg("task-001")
        .arg("you")
        .arg("--minutes")
        .arg("10")
        .output()
        .expect("claim");
    assert!(out.status.success());
    let out = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("release")
        .arg("task-001")
        .output()
        .expect("release");
    assert!(out.status.success());
}
