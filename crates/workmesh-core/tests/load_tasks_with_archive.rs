use tempfile::TempDir;

use workmesh_core::task::{load_tasks, load_tasks_with_archive};

fn write_task(dir: &std::path::Path, id: &str, title: &str, status: &str) {
    let filename = format!("{} - {}.md", id, title.to_lowercase());
    let path = dir.join(filename);
    let content = format!(
        "---\n\
id: {id}\n\
title: {title}\n\
status: {status}\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
---\n\n\
Body\n",
        id = id,
        title = title,
        status = status
    );
    std::fs::write(path, content).expect("write task");
}

#[test]
fn load_tasks_with_archive_includes_archived_markdown_files() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("workmesh");
    let tasks_dir = backlog_dir.join("tasks");
    let archive_dir = backlog_dir.join("archive").join("2026-02");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
    std::fs::create_dir_all(&archive_dir).expect("archive dir");

    write_task(&tasks_dir, "task-001", "Active", "To Do");
    write_task(&archive_dir, "task-002", "Archived", "Done");

    let active_only = load_tasks(&backlog_dir);
    assert_eq!(active_only.len(), 1);
    assert_eq!(active_only[0].id, "task-001");

    let all = load_tasks_with_archive(&backlog_dir);
    let ids: Vec<_> = all.into_iter().map(|t| t.id).collect();
    assert!(ids.contains(&"task-001".to_string()));
    assert!(ids.contains(&"task-002".to_string()));
}

