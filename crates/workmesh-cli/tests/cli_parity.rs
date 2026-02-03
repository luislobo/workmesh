use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn fixture_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample-backlog")
}

fn to_temp_path(path: &str) -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    let target = temp.path().join(path);
    std::fs::create_dir_all(&target).expect("mkdir");
    temp
}

#[test]
fn list_outputs_expected_rows() {
    let output = bin()
        .arg("--root")
        .arg(fixture_root())
        .arg("list")
        .output()
        .expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("task-001 | To Do | P2 | Phase1 | Alpha"));
    assert!(stdout.contains("task-002 | Done | P1 | Phase1 | Beta"));
}

#[test]
fn ready_outputs_ready_tasks() {
    let output = bin()
        .arg("--root")
        .arg(fixture_root())
        .arg("ready")
        .output()
        .expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("task-001 | To Do | P2 | Phase1 | Alpha"));
    assert!(!stdout.contains("task-002 | Done | P1 | Phase1 | Beta"));
}

#[test]
fn show_outputs_file_when_full() {
    let output = bin()
        .arg("--root")
        .arg(fixture_root())
        .arg("show")
        .arg("task-001")
        .arg("--full")
        .output()
        .expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("title: Alpha"));
    assert!(stdout.contains("Description:"));
}

#[test]
fn next_returns_empty_json_when_none_ready() {
    let temp = to_temp_path("repo/backlog/tasks");
    let tasks_dir = temp.path().join("repo/backlog/tasks");
    let content = "---\n"
        .to_string()
        + "id: task-001\n"
        + "title: Blocked\n"
        + "status: To Do\n"
        + "priority: P2\n"
        + "phase: Phase1\n"
        + "dependencies: [task-999]\n"
        + "labels: []\n"
        + "---\n";
    std::fs::write(tasks_dir.join("task-001 - blocked.md"), content).expect("write");

    let output = bin()
        .arg("--root")
        .arg(temp.path().join("repo/backlog"))
        .arg("next")
        .arg("--json")
        .output()
        .expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "{}");
}
