use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn write_legacy_task(tasks_dir: &std::path::Path, id: &str, title: &str) {
    let path = tasks_dir.join(format!("{} - {}.md", id, title.to_lowercase()));
    let body = format!(
        "---\nid: {id}\ntitle: {title}\nstatus: To Do\npriority: P2\nphase: Phase1\ndependencies: []\nlabels: []\nassignee: []\n---\n\nBody\n",
    );
    fs::write(path, body).expect("write task");
}

#[test]
fn bootstrap_initializes_new_repo_state() {
    let temp = TempDir::new().expect("tempdir");
    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("bootstrap")
        .arg("--project-id")
        .arg("alpha")
        .arg("--feature")
        .arg("Alpha Integration")
        .arg("--json")
        .output()
        .expect("run bootstrap");
    assert!(output.status.success());

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(
        parsed.get("state").and_then(|v| v.as_str()),
        Some("new_repo")
    );
    assert_eq!(
        parsed.get("project_id").and_then(|v| v.as_str()),
        Some("alpha")
    );
    assert!(parsed
        .get("quickstart")
        .and_then(|value| value.as_object())
        .is_some());
    assert!(temp.path().join("workmesh").join("tasks").is_dir());
    assert!(temp.path().join("workmesh").join("context.json").is_file());
}

#[test]
fn bootstrap_migrates_legacy_layout() {
    let temp = TempDir::new().expect("tempdir");
    let legacy_tasks = temp.path().join("backlog").join("tasks");
    fs::create_dir_all(&legacy_tasks).expect("mkdir");
    write_legacy_task(&legacy_tasks, "task-001", "Legacy");

    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("bootstrap")
        .arg("--json")
        .output()
        .expect("run bootstrap");
    assert!(output.status.success());

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(
        parsed.get("state").and_then(|value| value.as_str()),
        Some("legacy_repo")
    );
    assert!(temp.path().join("workmesh").join("tasks").is_dir());
}
