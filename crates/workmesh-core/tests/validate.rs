use workmesh_core::task_ops::validate_tasks;
use workmesh_core::task::Task;

fn task(id: &str, status: &str, deps: Vec<&str>) -> Task {
    Task {
        id: id.to_string(),
        title: "Example".to_string(),
        status: status.to_string(),
        priority: "P2".to_string(),
        phase: "Phase1".to_string(),
        dependencies: deps.into_iter().map(|d| d.to_string()).collect(),
        labels: Vec::new(),
        assignee: Vec::new(),
        relationships: Default::default(),
        project: None,
        initiative: None,
        created_date: None,
        updated_date: None,
        extra: Default::default(),
        file_path: None,
        body: String::new(),
    }
}

#[test]
fn validate_does_not_error_on_missing_dependencies() {
    let tasks = vec![task("task-001", "To Do", Vec::new())];
    let report = validate_tasks(&tasks, None);
    assert!(report.errors.iter().all(|err| !err.contains("dependencies")));
    assert!(report
        .warnings
        .iter()
        .any(|warn| warn.contains("no dependencies listed")));
}
