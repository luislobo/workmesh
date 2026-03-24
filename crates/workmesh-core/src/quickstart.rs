use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

use crate::config::{load_config, resolve_worktrees_default_with_source};
use crate::initiative::{
    best_effort_git_branch, ensure_branch_initiative_with_hint, initiative_key_from_hint,
    next_namespaced_task_id,
};
use crate::project::{ensure_project_docs, write_repo_root_metadata};
use crate::task::load_tasks;
use crate::task_ops::{create_task_file_with_sections, TaskSectionContent};

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
    pub state_root: PathBuf,
    pub tasks_root: PathBuf,
    pub created_task: Option<PathBuf>,
    pub agents_snippet_written: bool,
    pub worktrees_default: bool,
    pub worktrees_default_source: String,
    pub worktree_hint: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct QuickstartOptions {
    pub agents_snippet: bool,
    pub tasks_root: Option<String>,
    pub state_root: Option<String>,
}

pub fn quickstart(
    repo_root: &Path,
    project_id: &str,
    name: Option<&str>,
    initiative_hint: Option<&str>,
    options: &QuickstartOptions,
) -> Result<QuickstartResult, QuickstartError> {
    let config = load_config(repo_root);
    let tasks_root = resolve_scaffold_root(
        repo_root,
        options.tasks_root.as_deref(),
        config.as_ref().and_then(|cfg| cfg.tasks_root.as_deref()),
        config.as_ref().and_then(|cfg| cfg.root_dir.as_deref()),
        "tasks",
    );
    let state_root = resolve_scaffold_root(
        repo_root,
        options.state_root.as_deref(),
        config.as_ref().and_then(|cfg| cfg.state_root.as_deref()),
        config.as_ref().and_then(|cfg| cfg.root_dir.as_deref()),
        ".workmesh",
    );
    fs::create_dir_all(&tasks_root)?;
    fs::create_dir_all(&state_root)?;
    write_repo_root_metadata(&state_root, repo_root)?;

    let project_dir = ensure_project_docs(repo_root, project_id, name)?;
    let tasks = load_tasks(&state_root);
    let hint = initiative_hint.or(name).unwrap_or(project_id);
    let seed_task_id = resolve_seed_task_id(repo_root, &tasks, hint);
    let created_task = create_sample_task_if_missing(&tasks_root, &seed_task_id)?;
    let agents_snippet_written = if options.agents_snippet {
        write_agents_snippet(repo_root, &tasks_root, &state_root)?
    } else {
        false
    };
    let (worktrees_default, worktrees_default_source) =
        resolve_worktrees_default_with_source(repo_root);
    let worktree_hint = if worktrees_default {
        Some(default_worktree_hint(project_id))
    } else {
        None
    };

    Ok(QuickstartResult {
        project_dir,
        state_root,
        tasks_root,
        created_task,
        agents_snippet_written,
        worktrees_default,
        worktrees_default_source: worktrees_default_source.to_string(),
        worktree_hint,
    })
}

fn resolve_scaffold_root(
    repo_root: &Path,
    configured: Option<&str>,
    config_value: Option<&str>,
    legacy_root_dir: Option<&str>,
    default_name: &str,
) -> PathBuf {
    let selected = configured
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            config_value
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .or_else(|| {
            let legacy = legacy_root_dir?.trim();
            if legacy.is_empty() {
                return None;
            }
            if default_name == "tasks" {
                Some(format!("{legacy}/tasks"))
            } else {
                Some(legacy.to_string())
            }
        });
    selected
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                repo_root.join(path)
            }
        })
        .unwrap_or_else(|| repo_root.join(default_name))
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
    let path = create_task_file_with_sections(
        tasks_dir,
        task_id,
        "Initial setup",
        "To Do",
        "P2",
        "Phase1",
        &[],
        &[],
        &[],
        &TaskSectionContent {
            description:
                "- Establish the initial WorkMesh scaffold and verify the repository is ready for task-driven work."
                    .to_string(),
            acceptance_criteria:
                "- WorkMesh task and state directories exist in the configured locations.\n- Repo-local docs and context are initialized for this repository.".to_string(),
            definition_of_done:
                "- Bootstrap or quickstart completed successfully.\n- The initial repository workflow is ready for the next actionable task.".to_string(),
        },
    )?;
    Ok(Some(path))
}

fn write_agents_snippet(
    repo_root: &Path,
    tasks_root: &Path,
    state_root: &Path,
) -> Result<bool, QuickstartError> {
    let path = repo_root.join("AGENTS.md");
    let snippet = agents_snippet(repo_root, tasks_root, state_root);
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        if content.contains(snippet_marker()) {
            return Ok(false);
        }
        let mut updated = content;
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&snippet);
        fs::write(&path, updated)?;
        return Ok(true);
    }
    fs::write(&path, snippet)?;
    Ok(true)
}

fn snippet_marker() -> &'static str {
    "WorkMesh Quickstart"
}

fn agents_snippet(repo_root: &Path, tasks_root: &Path, state_root: &Path) -> String {
    let tasks = relative_display(repo_root, tasks_root);
    let state = relative_display(repo_root, state_root);
    format!(
        "# WorkMesh Quickstart\n\n- Tasks live in `{tasks}`.\n- Run `workmesh --root . next` to find the next task.\n- Run `workmesh --root . ready --json` for ready work.\n- Derived files (`{state}/.index/`, `{state}/.audit.log`) should not be committed.\n"
    )
}

fn relative_display(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .ok()
        .map(|value| value.to_string_lossy().trim_start_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn default_worktree_hint(project_id: &str) -> String {
    let stream = "<stream>";
    format!(
        "workmesh --root . worktree create --path ../<repo>-{stream} --branch feature/{stream} --project {project_id} --objective \"<objective>\"",
    )
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
        assert!(write_agents_snippet(repo, &repo.join("tasks"), &repo.join(".workmesh")).expect("write"));
        let content = fs::read_to_string(repo.join("AGENTS.md")).expect("read");
        assert!(content.contains(snippet_marker()));

        // When marker already present, it does not modify.
        assert!(!write_agents_snippet(repo, &repo.join("tasks"), &repo.join(".workmesh")).expect("idempotent"));

        // When file exists without marker, it appends.
        let temp2 = TempDir::new().expect("tempdir");
        let repo2 = temp2.path();
        fs::write(repo2.join("AGENTS.md"), "existing\n").expect("write");
        assert!(write_agents_snippet(repo2, &repo2.join("tasks"), &repo2.join(".workmesh")).expect("append"));
        let content2 = fs::read_to_string(repo2.join("AGENTS.md")).expect("read");
        assert!(content2.starts_with("existing\n"));
        assert!(content2.contains(snippet_marker()));
    }

    #[test]
    fn quickstart_uses_configured_roots_when_options_omit_them() {
        let temp = TempDir::new().expect("tempdir");
        fs::write(
            temp.path().join(".workmesh.toml"),
            "tasks_root = \"planning/tasks\"\nstate_root = \"planning/state\"\n",
        )
        .expect("config");

        let result = quickstart(
            temp.path(),
            "demo",
            None,
            None,
            &QuickstartOptions::default(),
        )
        .expect("quickstart");

        assert_eq!(result.tasks_root, temp.path().join("planning").join("tasks"));
        assert_eq!(result.state_root, temp.path().join("planning").join("state"));
        assert!(result.tasks_root.is_dir());
        assert!(result.state_root.is_dir());
    }
}
