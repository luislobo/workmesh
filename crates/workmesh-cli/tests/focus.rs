use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

#[test]
fn focus_set_show_clear_json() {
    let repo = TempDir::new().expect("repo");

    // Minimal workmesh layout + docs singleton project for inference.
    std::fs::create_dir_all(repo.path().join("workmesh").join("tasks")).expect("tasks");
    std::fs::create_dir_all(
        repo.path()
            .join("docs")
            .join("projects")
            .join("alpha")
            .join("updates"),
    )
    .expect("docs");

    let set = bin()
        .arg("--root")
        .arg(repo.path())
        .arg("focus")
        .arg("set")
        .arg("--objective")
        .arg("Ship focus")
        .arg("--tasks")
        .arg("task-001,task-002")
        .arg("--json")
        .output()
        .expect("focus set");
    assert!(set.status.success());
    let created: Value = serde_json::from_slice(&set.stdout).expect("json");
    assert!(created.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
    let focus = created.get("focus").expect("focus");
    assert_eq!(
        focus.get("objective").and_then(|v| v.as_str()).unwrap(),
        "Ship focus"
    );
    assert_eq!(
        focus.get("project_id").and_then(|v| v.as_str()).unwrap(),
        "alpha"
    );

    let show = bin()
        .arg("--root")
        .arg(repo.path())
        .arg("focus")
        .arg("show")
        .arg("--json")
        .output()
        .expect("focus show");
    assert!(show.status.success());
    let shown: Value = serde_json::from_slice(&show.stdout).expect("json");
    assert!(shown.get("focus").is_some());

    let clear = bin()
        .arg("--root")
        .arg(repo.path())
        .arg("focus")
        .arg("clear")
        .arg("--json")
        .output()
        .expect("focus clear");
    assert!(clear.status.success());
    let cleared: Value = serde_json::from_slice(&clear.stdout).expect("json");
    assert!(cleared
        .get("cleared")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));
}
