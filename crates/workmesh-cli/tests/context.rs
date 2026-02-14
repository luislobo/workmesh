use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

#[test]
fn context_set_show_clear_json() {
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
        .arg("context")
        .arg("set")
        .arg("--objective")
        .arg("Ship context")
        .arg("--tasks")
        .arg("task-001,task-002")
        .arg("--json")
        .output()
        .expect("context set");
    assert!(set.status.success());
    let created: Value = serde_json::from_slice(&set.stdout).expect("json");
    assert!(created.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
    let context = created.get("context").expect("context");
    assert_eq!(
        context.get("objective").and_then(|v| v.as_str()).unwrap(),
        "Ship context"
    );
    assert_eq!(
        context.get("project_id").and_then(|v| v.as_str()).unwrap(),
        "alpha"
    );

    let show = bin()
        .arg("--root")
        .arg(repo.path())
        .arg("context")
        .arg("show")
        .arg("--json")
        .output()
        .expect("context show");
    assert!(show.status.success());
    let shown: Value = serde_json::from_slice(&show.stdout).expect("json");
    assert!(shown.get("context").is_some());

    let clear = bin()
        .arg("--root")
        .arg(repo.path())
        .arg("context")
        .arg("clear")
        .arg("--json")
        .output()
        .expect("context clear");
    assert!(clear.status.success());
    let cleared: Value = serde_json::from_slice(&clear.stdout).expect("json");
    assert!(cleared
        .get("cleared")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));
}
