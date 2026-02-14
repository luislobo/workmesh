use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn write_task(repo: &std::path::Path) {
    let tasks_dir = repo.join("workmesh").join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
    std::fs::write(
        tasks_dir.join("task-main-001 - seed.md"),
        "---\n\
id: task-main-001\n\
title: Seed\n\
kind: epic\n\
status: In Progress\n\
priority: P1\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
---\n\n\
## Notes\n\
- Decision: use append-only truth log\n",
    )
    .expect("write task");

    std::fs::write(
        repo.join("workmesh").join("context.json"),
        r#"{"version":1,"project_id":"workmesh","objective":"Ship truth","scope":{"mode":"epic","epic_id":"task-main-001","task_ids":[]},"updated_at":"2026-02-13T00:00:00Z"}"#,
    )
    .expect("write context");
}

#[test]
fn truth_commands_end_to_end_and_resume_includes_truth_refs() {
    let repo = TempDir::new().expect("repo");
    let home = TempDir::new().expect("home");
    write_task(repo.path());

    let propose = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("truth")
        .arg("propose")
        .arg("--title")
        .arg("Use append-only truth events")
        .arg("--statement")
        .arg("Truth records are append-only and immutable")
        .arg("--project")
        .arg("workmesh")
        .arg("--epic")
        .arg("task-main-001")
        .arg("--feature")
        .arg("truth-ledger")
        .arg("--tags")
        .arg("architecture,decision")
        .arg("--json")
        .output()
        .expect("propose");
    assert!(propose.status.success());
    let proposed: Value = serde_json::from_slice(&propose.stdout).expect("json");
    assert_eq!(proposed["state"], "proposed");
    let truth_a = proposed["id"].as_str().expect("id a").to_string();

    let accept = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("truth")
        .arg("accept")
        .arg(&truth_a)
        .arg("--note")
        .arg("approved")
        .arg("--json")
        .output()
        .expect("accept");
    assert!(accept.status.success());
    let accepted: Value = serde_json::from_slice(&accept.stdout).expect("json");
    assert_eq!(accepted["state"], "accepted");

    let propose_b = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("truth")
        .arg("propose")
        .arg("--title")
        .arg("Use projected current view")
        .arg("--statement")
        .arg("Current truth state is materialized from events")
        .arg("--project")
        .arg("workmesh")
        .arg("--epic")
        .arg("task-main-001")
        .arg("--json")
        .output()
        .expect("propose b");
    assert!(propose_b.status.success());
    let proposed_b: Value = serde_json::from_slice(&propose_b.stdout).expect("json");
    let truth_b = proposed_b["id"].as_str().expect("id b").to_string();

    let accept_b = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("truth")
        .arg("accept")
        .arg(&truth_b)
        .arg("--json")
        .output()
        .expect("accept b");
    assert!(accept_b.status.success());

    let supersede = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("truth")
        .arg("supersede")
        .arg(&truth_a)
        .arg("--by")
        .arg(&truth_b)
        .arg("--reason")
        .arg("replacement adopted")
        .arg("--json")
        .output()
        .expect("supersede");
    assert!(supersede.status.success());
    let superseded: Value = serde_json::from_slice(&supersede.stdout).expect("json");
    assert_eq!(superseded["state"], "superseded");
    assert_eq!(superseded["superseded_by"], truth_b);

    let list = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("truth")
        .arg("list")
        .arg("--state")
        .arg("accepted")
        .arg("--project")
        .arg("workmesh")
        .arg("--epic")
        .arg("task-main-001")
        .arg("--json")
        .output()
        .expect("list");
    assert!(list.status.success());
    let listed: Value = serde_json::from_slice(&list.stdout).expect("json");
    assert!(listed
        .as_array()
        .expect("array")
        .iter()
        .any(|entry| entry["id"] == truth_b));

    let validate = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("truth")
        .arg("validate")
        .arg("--json")
        .output()
        .expect("validate");
    assert!(validate.status.success());
    let validated: Value = serde_json::from_slice(&validate.stdout).expect("json");
    assert_eq!(validated["ok"], true);

    let save_1 = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("save")
        .arg("--objective")
        .arg("truth workflow")
        .arg("--cwd")
        .arg(repo.path())
        .arg("--json")
        .output()
        .expect("save1");
    assert!(save_1.status.success());
    let saved1: Value = serde_json::from_slice(&save_1.stdout).expect("json");
    let session_id_1 = saved1["id"].as_str().expect("session id").to_string();
    let truth_refs_1 = saved1["truth_refs"].as_array().expect("truth refs 1");
    assert!(!truth_refs_1.is_empty());

    let save_2 = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("save")
        .arg("--objective")
        .arg("truth workflow 2")
        .arg("--cwd")
        .arg(repo.path())
        .arg("--json")
        .output()
        .expect("save2");
    assert!(save_2.status.success());
    let saved2: Value = serde_json::from_slice(&save_2.stdout).expect("json");
    let truth_refs_2 = saved2["truth_refs"].as_array().expect("truth refs 2");
    assert!(!truth_refs_2.is_empty());

    let resume = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("resume")
        .arg(&session_id_1)
        .arg("--json")
        .output()
        .expect("resume");
    assert!(resume.status.success());
    let resumed: Value = serde_json::from_slice(&resume.stdout).expect("json");
    let script = resumed["resume_script"].as_array().expect("script");
    assert!(script.iter().any(|line| {
        line.as_str()
            .unwrap_or_default()
            .contains("truth list --state accepted")
    }));
}
