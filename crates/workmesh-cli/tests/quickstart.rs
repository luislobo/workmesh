use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

#[test]
fn quickstart_scaffolds_repo() {
    let temp = TempDir::new().expect("tempdir");

    let output = bin()
        .arg("--root")
        .arg(temp.path())
        .arg("quickstart")
        .arg("alpha")
        .arg("--name")
        .arg("Alpha Project")
        .arg("--agents-snippet")
        .output()
        .expect("run quickstart");
    assert!(output.status.success());

    // WorkMesh stores tasks in `workmesh/tasks/` (docs live under `docs/projects/<id>/`).
    let tasks_dir = temp.path().join("workmesh").join("tasks");
    assert!(tasks_dir.is_dir());
    let task_file = fs::read_dir(&tasks_dir)
        .expect("read tasks")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().map(|ext| ext == "md").unwrap_or(false))
        .expect("seed task file");
    let task_name = task_file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    assert!(task_name.starts_with("task-"));
    assert!(task_name.ends_with(".md"));
    assert!(!task_name.starts_with("task-001"));
    assert!(task_name.contains("-001"));

    let docs_root = temp.path().join("docs").join("projects").join("alpha");
    assert!(docs_root.join("README.md").is_file());

    let agents = temp.path().join("AGENTS.md");
    assert!(agents.is_file());
    let agents_text = fs::read_to_string(&agents).expect("read AGENTS.md");
    assert!(agents_text.contains("Derived files"));
}
