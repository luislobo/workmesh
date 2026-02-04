use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

fn write_task(dir: &std::path::Path, id: &str, title: &str, status: &str, updated: Option<&str>) {
    let filename = format!("{} - {}.md", id, title.to_lowercase());
    let path = dir.join(filename);
    let updated_line = updated.map(|value| format!("updated_date: {}\n", value)).unwrap_or_default();
    let content = format!(
        "---\nid: {id}\ntitle: {title}\nstatus: {status}\npriority: P2\nphase: Phase3\ndependencies: []\nlabels: []\nassignee: []\n{updated}---\n\nBody\n",
        id = id,
        title = title,
        status = status,
        updated = updated_line
    );
    fs::write(path, content).expect("write task");
}

#[test]
fn checkpoint_diff_shows_updates() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha", "To Do", Some("2099-01-01 00:00"));

    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("checkpoint")
        .arg("--project")
        .arg("alpha")
        .arg("--id")
        .arg("20260204-121000")
        .output()
        .expect("checkpoint");
    assert!(output.status.success());

    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("checkpoint-diff")
        .arg("--project")
        .arg("alpha")
        .arg("--id")
        .arg("20260204-121000")
        .output()
        .expect("diff");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated tasks"));
    assert!(stdout.contains("task-001"));
}
