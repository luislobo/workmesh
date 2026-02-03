use tempfile::TempDir;

use workmesh_core::project::{ensure_project_docs, project_docs_dir, repo_root_from_backlog};
use workmesh_core::task::Task;
use workmesh_core::task_ops::validate_tasks;

fn task_with_project(project: &str) -> Task {
    Task {
        id: "task-001".to_string(),
        title: "Example".to_string(),
        status: "To Do".to_string(),
        priority: "P2".to_string(),
        phase: "Phase1".to_string(),
        dependencies: Vec::new(),
        labels: Vec::new(),
        assignee: Vec::new(),
        relationships: Default::default(),
        project: Some(project.to_string()),
        initiative: None,
        created_date: None,
        updated_date: None,
        extra: Default::default(),
        file_path: None,
        body: String::new(),
    }
}

#[test]
fn ensure_project_docs_creates_scaffold() {
    let temp = TempDir::new().expect("tempdir");
    let repo_root = temp.path();

    let path = ensure_project_docs(repo_root, "alpha", Some("Alpha Project"))
        .expect("create project docs");
    assert!(path.join("README.md").is_file());
    assert!(path.join("prds").is_dir());
    assert!(path.join("decisions").is_dir());
    assert!(path.join("updates").is_dir());
    assert!(path.join("initiatives").is_dir());
}

#[test]
fn validate_errors_when_project_docs_missing() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    std::fs::create_dir_all(backlog_dir.join("tasks")).expect("tasks dir");

    let report = validate_tasks(&[task_with_project("alpha")], Some(&backlog_dir));
    assert!(report
        .errors
        .iter()
        .any(|err| err.contains("project docs missing")));

    let repo_root = repo_root_from_backlog(&backlog_dir);
    let project_dir = project_docs_dir(&repo_root, "alpha");
    std::fs::create_dir_all(&project_dir).expect("project dir");
    std::fs::write(project_dir.join("README.md"), "# Alpha\n").expect("readme");

    let report = validate_tasks(&[task_with_project("alpha")], Some(&backlog_dir));
    assert!(report.errors.iter().all(|err| !err.contains("project docs missing")));
}
