use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn run_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_seed_task(repo: &Path) {
    let tasks_dir = repo.join("workmesh").join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
    std::fs::write(
        tasks_dir.join("task-001 - seed.md"),
        "---\n\
id: task-001\n\
title: Seed\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
---\n\n\
## Notes\n- seed\n",
    )
    .expect("seed task");
}

#[test]
fn workstream_create_list_switch_updates_context_pointer() {
    let home = TempDir::new().expect("home");
    let repo = TempDir::new().expect("repo");
    write_seed_task(repo.path());

    run_git(repo.path(), &["init"]);
    run_git(repo.path(), &["config", "user.name", "WorkMesh Test"]);
    run_git(
        repo.path(),
        &["config", "user.email", "workmesh-test@example.com"],
    );
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "seed"]);

    let alpha = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("workstream")
        .arg("create")
        .arg("--name")
        .arg("Alpha")
        .arg("--key")
        .arg("alpha")
        .arg("--objective")
        .arg("Ship alpha")
        .arg("--json")
        .output()
        .expect("workstream create alpha");
    assert!(alpha.status.success());
    let alpha_json: Value = serde_json::from_slice(&alpha.stdout).expect("json");
    assert!(alpha_json
        .get("ok")
        .and_then(|value| value.as_bool())
        .unwrap_or(false));
    let alpha_id = alpha_json
        .get("workstream")
        .and_then(|value| value.get("id"))
        .and_then(|value| value.as_str())
        .expect("alpha id")
        .to_string();

    let beta = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("workstream")
        .arg("create")
        .arg("--name")
        .arg("Beta")
        .arg("--key")
        .arg("beta")
        .arg("--objective")
        .arg("Ship beta")
        .arg("--json")
        .output()
        .expect("workstream create beta");
    assert!(beta.status.success());
    let beta_json: Value = serde_json::from_slice(&beta.stdout).expect("json");
    let beta_id = beta_json
        .get("workstream")
        .and_then(|value| value.get("id"))
        .and_then(|value| value.as_str())
        .expect("beta id")
        .to_string();
    assert_ne!(alpha_id, beta_id);

    // Latest created stream should be active in this worktree.
    let list = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("workstream")
        .arg("list")
        .arg("--json")
        .output()
        .expect("workstream list");
    assert!(list.status.success());
    let list_json: Value = serde_json::from_slice(&list.stdout).expect("json");
    let active_id = list_json
        .get("active_workstream_id")
        .and_then(|value| value.as_str())
        .expect("active id");
    assert_eq!(active_id, beta_id);

    // Switch back to alpha by key, and verify context pointer updated.
    let switch = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("workstream")
        .arg("switch")
        .arg("alpha")
        .arg("--json")
        .output()
        .expect("workstream switch");
    assert!(switch.status.success());

    let context = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("context")
        .arg("show")
        .arg("--json")
        .output()
        .expect("context show");
    assert!(context.status.success());
    let context_json: Value = serde_json::from_slice(&context.stdout).expect("json");
    let workstream_id = context_json
        .get("context")
        .and_then(|value| value.get("workstream_id"))
        .and_then(|value| value.as_str())
        .expect("context workstream id");
    assert_eq!(workstream_id, alpha_id);
}
