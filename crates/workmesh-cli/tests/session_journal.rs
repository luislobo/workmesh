use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

#[test]
fn session_journal_appends_entry() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    fs::create_dir_all(backlog_dir.join("tasks")).expect("tasks dir");

    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("session-journal")
        .arg("--project")
        .arg("alpha")
        .arg("--task")
        .arg("task-001")
        .arg("--next")
        .arg("Review notes")
        .output()
        .expect("session-journal");
    assert!(output.status.success());

    let journal = temp
        .path()
        .join("docs")
        .join("projects")
        .join("alpha")
        .join("updates")
        .join("session-journal.md");
    assert!(journal.is_file());
    let content = fs::read_to_string(journal).expect("read journal");
    assert!(content.contains("Task: task-001"));
    assert!(content.contains("Next: Review notes"));
}
