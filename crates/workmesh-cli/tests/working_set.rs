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
fn working_set_writes_file() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha", "In Progress");

    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("working-set")
        .arg("--project")
        .arg("alpha")
        .output()
        .expect("working-set");
    assert!(output.status.success());

    let working_set = temp
        .path()
        .join("docs")
        .join("projects")
        .join("alpha")
        .join("updates")
        .join("working-set.md");
    assert!(working_set.is_file());
    let content = fs::read_to_string(working_set).expect("read working set");
    assert!(content.contains("task-001"));
}
