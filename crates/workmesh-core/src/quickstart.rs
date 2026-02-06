use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

use crate::project::ensure_project_docs;
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
    agents_snippet: bool,
) -> Result<QuickstartResult, QuickstartError> {
    let backlog_dir = repo_root.join("workmesh");
    let tasks_dir = backlog_dir.join("tasks");
    fs::create_dir_all(&tasks_dir)?;

    let project_dir = ensure_project_docs(repo_root, project_id, name)?;
    let created_task = create_sample_task_if_missing(&tasks_dir)?;
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

fn create_sample_task_if_missing(tasks_dir: &Path) -> Result<Option<PathBuf>, QuickstartError> {
    let has_tasks = fs::read_dir(tasks_dir)?
        .filter_map(Result::ok)
        .any(|entry| entry.path().extension().map(|ext| ext == "md").unwrap_or(false));
    if has_tasks {
        return Ok(None);
    }
    let path = create_task_file(
        tasks_dir,
        "task-001",
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
    "# WorkMesh Quickstart\n\n- Tasks live in `workmesh/tasks/`.\n- Run `workmesh --root . next` to find the next task.\n- Run `workmesh --root . ready --json` for ready work.\n"
}
