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
fn worktree_create_list_and_attach_session() {
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

    let worktree_path = repo.path().join("wt-feature");
    let create = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("worktree")
        .arg("create")
        .arg("--path")
        .arg(&worktree_path)
        .arg("--branch")
        .arg("feature/worktree-test")
        .arg("--json")
        .output()
        .expect("worktree create");
    assert!(create.status.success());
    let created: Value = serde_json::from_slice(&create.stdout).expect("json");
    assert!(created
        .get("ok")
        .and_then(|value| value.as_bool())
        .unwrap_or(false));

    let list = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("worktree")
        .arg("list")
        .arg("--json")
        .output()
        .expect("worktree list");
    assert!(list.status.success());
    let listed: Value = serde_json::from_slice(&list.stdout).expect("json");
    let entries = listed
        .get("worktrees")
        .and_then(|value| value.as_array())
        .expect("worktrees");
    assert!(!entries.is_empty());

    let save = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("save")
        .arg("--objective")
        .arg("Worktree test")
        .arg("--cwd")
        .arg(repo.path())
        .arg("--json")
        .output()
        .expect("session save");
    assert!(save.status.success());
    let saved: Value = serde_json::from_slice(&save.stdout).expect("json");
    let session_id = saved
        .get("id")
        .and_then(|value| value.as_str())
        .expect("session id");

    let attach = bin()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("worktree")
        .arg("attach")
        .arg("--session-id")
        .arg(session_id)
        .arg("--path")
        .arg(&worktree_path)
        .arg("--json")
        .output()
        .expect("worktree attach");
    assert!(attach.status.success());
    let attached: Value = serde_json::from_slice(&attach.stdout).expect("json");
    let bound_path = attached
        .get("worktree")
        .and_then(|value| value.get("path"))
        .and_then(|value| value.as_str())
        .expect("worktree path");
    assert!(bound_path.contains("wt-feature"));
}
