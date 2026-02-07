use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn fixture_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample-backlog")
}

#[test]
fn session_save_list_show_resume_json() {
    let home = TempDir::new().expect("tempdir");

    let save = bin()
        .arg("--root")
        .arg(fixture_root())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("save")
        .arg("--objective")
        .arg("Test objective")
        .arg("--cwd")
        .arg(fixture_root())
        .arg("--json")
        .output()
        .expect("run");
    assert!(save.status.success());
    let saved: Value = serde_json::from_slice(&save.stdout).expect("json");
    let session_id = saved
        .get("id")
        .and_then(|v| v.as_str())
        .expect("id")
        .to_string();

    let list = bin()
        .arg("--root")
        .arg(fixture_root())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("list")
        .arg("--json")
        .output()
        .expect("run");
    assert!(list.status.success());
    let listed: Value = serde_json::from_slice(&list.stdout).expect("json");
    assert!(listed.to_string().contains(&session_id));

    let show = bin()
        .arg("--root")
        .arg(fixture_root())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("show")
        .arg(&session_id)
        .arg("--json")
        .output()
        .expect("run");
    assert!(show.status.success());
    let shown: Value = serde_json::from_slice(&show.stdout).expect("json");
    assert_eq!(
        shown.get("id").and_then(|v| v.as_str()).unwrap(),
        session_id
    );

    let resume = bin()
        .arg("--root")
        .arg(fixture_root())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("resume")
        .arg(&session_id)
        .arg("--json")
        .output()
        .expect("run");
    assert!(resume.status.success());
    let resumed: Value = serde_json::from_slice(&resume.stdout).expect("json");
    assert_eq!(
        resumed
            .get("session")
            .and_then(|s| s.get("id"))
            .and_then(|v| v.as_str())
            .unwrap(),
        session_id
    );
    let script = resumed
        .get("resume_script")
        .and_then(|v| v.as_array())
        .expect("resume_script");
    assert!(!script.is_empty());
}

#[test]
fn auto_session_save_updates_current_session_on_mutations() {
    let home = TempDir::new().expect("home tempdir");
    let repo = TempDir::new().expect("repo tempdir");

    let tasks_dir = repo.path().join("workmesh").join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
    std::fs::write(
        tasks_dir.join("task-001 - alpha.md"),
        "---\n\
id: task-001\n\
title: Alpha\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
---\n\n\
## Notes\n- initial\n",
    )
    .expect("write task");

    let save = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("save")
        .arg("--objective")
        .arg("Auto test")
        .arg("--cwd")
        .arg(repo.path())
        .arg("--json")
        .output()
        .expect("save");
    assert!(save.status.success());
    let saved: Value = serde_json::from_slice(&save.stdout).expect("json");
    let session_id = saved
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();
    let first_updated = saved
        .get("updated_at")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();

    let mutate = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("--auto-session-save")
        .arg("set-status")
        .arg("task-001")
        .arg("In Progress")
        .output()
        .expect("mutate");
    assert!(mutate.status.success());

    let show = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("show")
        .arg(&session_id)
        .arg("--json")
        .output()
        .expect("show");
    assert!(show.status.success());
    let shown: Value = serde_json::from_slice(&show.stdout).expect("json");
    let second_updated = shown
        .get("updated_at")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();
    assert_ne!(first_updated, second_updated);
}
