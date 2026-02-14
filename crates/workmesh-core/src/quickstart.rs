use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

use crate::initiative::{
    best_effort_git_branch, ensure_branch_initiative_with_hint, initiative_key_from_hint,
    next_namespaced_task_id,
};
use crate::project::ensure_project_docs;
use crate::task::load_tasks;
use crate::task_ops::create_task_file;

#[derive(Debug, Error)]
pub enum QuickstartError {
    #[error("Failed to create quickstart files: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to create project docs: {0}")]
    Project(#[from] crate::project::ProjectError),
    #[error("Failed to create task file: {0}")]
    Task(#[from] crate::task::TaskParseError),
}

#[derive(Debug, Serialize)]
pub struct QuickstartResult {
    pub project_dir: PathBuf,
    pub backlog_dir: PathBuf,
    pub tasks_dir: PathBuf,
    pub created_task: Option<PathBuf>,
    pub agents_snippet_written: bool,
}

pub fn quickstart(
    repo_root: &Path,
    project_id: &str,
    name: Option<&str>,
    initiative_hint: Option<&str>,
    agents_snippet: bool,
) -> Result<QuickstartResult, QuickstartError> {
    let backlog_dir = repo_root.join("workmesh");
    let tasks_dir = backlog_dir.join("tasks");
    fs::create_dir_all(&tasks_dir)?;

    let project_dir = ensure_project_docs(repo_root, project_id, name)?;
    let tasks = load_tasks(&backlog_dir);
    let hint = initiative_hint.or(name).unwrap_or(project_id);
    let seed_task_id = resolve_seed_task_id(repo_root, &tasks, hint);
    let created_task = create_sample_task_if_missing(&tasks_dir, &seed_task_id)?;
    let agents_snippet_written = if agents_snippet {
        write_agents_snippet(repo_root)?
    } else {
        false
    };

    Ok(QuickstartResult {
        project_dir,
        backlog_dir,
        tasks_dir,
        created_task,
        agents_snippet_written,
    })
}

fn resolve_seed_task_id(repo_root: &Path, tasks: &[crate::task::Task], hint: &str) -> String {
    let initiative = best_effort_git_branch(repo_root)
        .and_then(|branch| {
            ensure_branch_initiative_with_hint(repo_root, &branch, Some(hint))
                .ok()
                .or_else(|| initiative_key_from_hint(hint))
        })
        .or_else(|| initiative_key_from_hint(hint))
        .unwrap_or_else(|| "work".to_string());
    next_namespaced_task_id(tasks, &initiative)
}

fn create_sample_task_if_missing(
    tasks_dir: &Path,
    task_id: &str,
) -> Result<Option<PathBuf>, QuickstartError> {
    let has_tasks = fs::read_dir(tasks_dir)?
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "md")
                .unwrap_or(false)
        });
    if has_tasks {
        return Ok(None);
    }
    let path = create_task_file(
        tasks_dir,
        task_id,
        "Initial setup",
        "To Do",
        "P2",
        "Phase1",
        &[],
        &[],
        &[],
    )?;
    Ok(Some(path))
}

fn write_agents_snippet(repo_root: &Path) -> Result<bool, QuickstartError> {
    let path = repo_root.join("AGENTS.md");
    let snippet = agents_snippet();
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        if content.contains(snippet_marker()) {
            return Ok(false);
        }
        let mut updated = content;
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(snippet);
        fs::write(&path, updated)?;
        return Ok(true);
    }
    fs::write(&path, snippet)?;
    Ok(true)
}

fn snippet_marker() -> &'static str {
    "WorkMesh Quickstart"
}

fn agents_snippet() -> &'static str {
    "# WorkMesh Quickstart\n\n- Tasks live in `workmesh/tasks/`.\n- Run `workmesh --root . next` to find the next task.\n- Run `workmesh --root . ready --json` for ready work.\n- Derived files (`workmesh/.index/`, `workmesh/.audit.log`) should not be committed.\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn create_sample_task_if_missing_creates_first_task_only_when_empty() {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path()).expect("dir");

        let created = create_sample_task_if_missing(temp.path(), "task-boot-001").expect("create");
        assert!(created.is_some());
        let created =
            create_sample_task_if_missing(temp.path(), "task-boot-002").expect("create again");
        assert!(created.is_none());

        // Non-markdown files don't count as tasks.
        let temp2 = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp2.path()).expect("dir");
        fs::write(temp2.path().join("note.txt"), "hi").expect("write");
        let created = create_sample_task_if_missing(temp2.path(), "task-boot-001").expect("create");
        assert!(created.is_some());
    }

    #[test]
    fn resolve_seed_task_id_prefers_hint_initials() {
        let temp = TempDir::new().expect("tempdir");
        let tasks = Vec::new();
        let id = resolve_seed_task_id(temp.path(), &tasks, "Smart Recipe Box");
        assert_eq!(id, "task-srbm-001");
    }

    #[test]
    fn write_agents_snippet_writes_or_appends_idempotently() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();

        // When missing, it writes a new file.
        assert!(write_agents_snippet(repo).expect("write"));
        let content = fs::read_to_string(repo.join("AGENTS.md")).expect("read");
        assert!(content.contains(snippet_marker()));

        // When marker already present, it does not modify.
        assert!(!write_agents_snippet(repo).expect("idempotent"));

        // When file exists without marker, it appends.
        let temp2 = TempDir::new().expect("tempdir");
        let repo2 = temp2.path();
        fs::write(repo2.join("AGENTS.md"), "existing\n").expect("write");
        assert!(write_agents_snippet(repo2).expect("append"));
        let content2 = fs::read_to_string(repo2.join("AGENTS.md")).expect("read");
        assert!(content2.starts_with("existing\n"));
        assert!(content2.contains(snippet_marker()));
    }
}
