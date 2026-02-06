use workmesh_core::gantt::plantuml_gantt;
use workmesh_core::task::Task;

fn task(id: &str, title: &str, status: &str, phase: &str, deps: &[&str]) -> Task {
    Task {
        id: id.to_string(),
        uid: None,
        kind: "task".to_string(),
        title: title.to_string(),
        status: status.to_string(),
        priority: "P2".to_string(),
        phase: phase.to_string(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        labels: Vec::new(),
        assignee: Vec::new(),
        relationships: Default::default(),
        lease: None,
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
fn gantt_includes_dependencies() {
    let tasks = vec![
        task("task-001", "One", "Done", "Phase1", &[]),
        task("task-002", "Two", "To Do", "Phase1", &["task-001"]),
    ];
    let output = plantuml_gantt(&tasks, Some("2026-02-01"), None, 3, None, true);
    assert!(output.contains("Project starts 2026-02-01"));
    assert!(output.contains("[task-001 One] --> [task-002 Two]"));
}
