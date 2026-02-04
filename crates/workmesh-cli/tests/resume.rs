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
fn resume_reads_checkpoint() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha", "In Progress");
    write_task(&tasks_dir, "task-002", "Beta", "To Do");

    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("checkpoint")
        .arg("--project")
        .arg("alpha")
        .arg("--id")
        .arg("20260204-120500")
        .output()
        .expect("checkpoint");
    assert!(output.status.success());

    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("resume")
        .arg("--project")
        .arg("alpha")
        .arg("--id")
        .arg("20260204-120500")
        .output()
        .expect("resume");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Resume from checkpoint"));
    assert!(stdout.contains("Current task"));
}
