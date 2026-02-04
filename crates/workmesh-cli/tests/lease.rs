use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn write_task(tasks_dir: &std::path::Path) {
    let content = "---\n"
        .to_string()
        + "id: task-001\n"
        + "title: Lease Test\n"
        + "status: To Do\n"
        + "priority: P2\n"
        + "phase: Phase1\n"
        + "dependencies: []\n"
        + "labels: []\n"
        + "assignee: []\n"
        + "---\n";
    std::fs::write(tasks_dir.join("task-001 - lease.md"), content).expect("write");
}

#[test]
fn claim_and_release_updates_lease_fields() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
    write_task(&tasks_dir);

    let claim = bin()
        .arg("--root")
        .arg(&backlog_dir)
        .arg("claim")
        .arg("task-001")
        .arg("agent-1")
        .arg("--minutes")
        .arg("30")
        .output()
        .expect("claim");
    assert!(claim.status.success());

    let content = std::fs::read_to_string(tasks_dir.join("task-001 - lease.md")).expect("read");
    assert!(content.contains("lease_owner: agent-1"));
    assert!(content.contains("lease_expires_at:"));

    let release = bin()
        .arg("--root")
        .arg(&backlog_dir)
        .arg("release")
        .arg("task-001")
        .output()
        .expect("release");
    assert!(release.status.success());

    let content = std::fs::read_to_string(tasks_dir.join("task-001 - lease.md")).expect("read");
    assert!(!content.contains("lease_owner: agent-1"));
}
