use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn write_task(dir: &std::path::Path, id: &str, title: &str, status: &str) {
    let filename = format!("{} - {}.md", id, title.to_lowercase());
    let path = dir.join(filename);
    let content = format!(
        "---\nid: {id}\ntitle: {title}\nstatus: {status}\npriority: P2\nphase: Phase3\ndependencies: []\nlabels: []\nassignee: []\n---\n\nBody\n",
        id = id,
        title = title,
        status = status
    );
    fs::write(path, content).expect("write task");
}

#[test]
fn checkpoint_writes_json_and_markdown() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha", "To Do");
    write_task(&tasks_dir, "task-002", "Beta", "In Progress");

    let project_id = "alpha";
    let docs_updates = temp
        .path()
        .join("docs")
        .join("projects")
        .join(project_id)
        .join("updates");
    fs::create_dir_all(&docs_updates).expect("updates dir");

    let audit_path = backlog_dir.join(".audit.log");
    fs::write(
        audit_path,
        r#"{"timestamp":"2026-02-04 10:00","actor":"tester","action":"set_status","task_id":"task-002","details":{}}"#,
    )
    .expect("audit log");

    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("checkpoint")
        .arg("--project")
        .arg(project_id)
        .arg("--id")
        .arg("20260204-120000")
        .output()
        .expect("run checkpoint");
    assert!(output.status.success());

    let json_path = docs_updates.join("checkpoint-20260204-120000.json");
    let md_path = docs_updates.join("checkpoint-20260204-120000.md");
    assert!(json_path.is_file());
    assert!(md_path.is_file());

    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).expect("read json"))
            .expect("parse json");
    assert_eq!(data["project_id"], project_id);
    assert_eq!(data["current_task"]["id"], "task-002");
    let ready = data["ready"].as_array().expect("ready array");
    assert!(ready.iter().any(|item| item["id"] == "task-001"));
}
