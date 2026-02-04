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

    let backlog_dir = temp.path().join("backlog");
    assert!(backlog_dir.join("tasks").is_dir());

    let docs_root = temp.path().join("docs").join("projects").join("alpha");
    assert!(docs_root.join("README.md").is_file());

    let agents = temp.path().join("AGENTS.md");
    assert!(agents.is_file());
}
