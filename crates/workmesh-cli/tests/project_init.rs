use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_workmesh"))
}

#[test]
fn project_init_creates_docs() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    std::fs::create_dir_all(backlog_dir.join("tasks")).expect("tasks dir");

    let output = bin()
        .arg("--root")
        .arg(&backlog_dir)
        .arg("project-init")
        .arg("alpha")
        .arg("--name")
        .arg("Alpha Project")
        .output()
        .expect("run");
    assert!(output.status.success());

    let docs_root = temp.path().join("docs").join("projects").join("alpha");
    assert!(docs_root.join("README.md").is_file());
    assert!(docs_root.join("prds").is_dir());
    assert!(docs_root.join("decisions").is_dir());
    assert!(docs_root.join("updates").is_dir());
    assert!(docs_root.join("initiatives").is_dir());
}

