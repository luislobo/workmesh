use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

use crate::backlog::{resolve_backlog, BacklogError};
use crate::context::{
    context_path, infer_project_id, load_context, save_context, ContextScope, ContextScopeMode,
    ContextState,
};
use crate::migration_audit::{
    apply_migration_plan, audit_deprecations, plan_migrations, MigrationApplyOptions,
    MigrationAuditError, MigrationPlanOptions,
};
use crate::quickstart::{quickstart, QuickstartError, QuickstartResult};
use crate::session::resolve_project_id;
use crate::task::load_tasks;
use crate::task_ops::recommend_next_tasks_with_context;
use crate::worktrees::list_git_worktrees;

#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("Backlog resolution failed: {0}")]
    Backlog(#[from] BacklogError),
    #[error("Quickstart failed: {0}")]
    Quickstart(#[from] QuickstartError),
    #[error("Migration failed: {0}")]
    Migration(#[from] MigrationAuditError),
    #[error("Context update failed: {0}")]
    Context(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapState {
    NewRepo,
    ModernRepo,
    LegacyRepo,
}

impl BootstrapState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NewRepo => "new_repo",
            Self::ModernRepo => "modern_repo",
            Self::LegacyRepo => "legacy_repo",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BootstrapOptions {
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub feature: Option<String>,
    pub objective: Option<String>,
    pub agents_snippet: bool,
}

#[derive(Debug, Serialize)]
pub struct BootstrapResult {
    pub repo_root: PathBuf,
    pub backlog_dir: PathBuf,
    pub state: BootstrapState,
    pub project_id: String,
    pub context_seeded: bool,
    pub context_path: PathBuf,
    pub quickstart: Option<QuickstartResult>,
    pub migration_applied: Vec<String>,
    pub migration_warnings: Vec<String>,
    pub next_task_ids: Vec<String>,
    pub recommendations: Vec<String>,
}

pub fn bootstrap_repo(
    repo_root: &Path,
    options: &BootstrapOptions,
) -> Result<BootstrapResult, BootstrapError> {
    let repo_root = normalize_root(repo_root);

    let mut quickstart_result = None;
    let mut migration_applied = Vec::new();
    let mut migration_warnings = Vec::new();
    let state = match resolve_backlog(&repo_root) {
        Ok(_) => {
            let report = audit_deprecations(&repo_root)?;
            let required = report
                .findings
                .iter()
                .any(|finding| finding.severity.eq_ignore_ascii_case("required"));
            if required {
                // Bootstrap applies only required modernization steps; optional migrations remain explicit.
                let plan = plan_migrations(
                    &report,
                    &MigrationPlanOptions {
                        include: vec![
                            "layout_backlog_to_workmesh".to_string(),
                            "focus_to_context".to_string(),
                        ],
                        exclude: Vec::new(),
                    },
                );
                let applied = apply_migration_plan(
                    &repo_root,
                    &plan,
                    &MigrationApplyOptions {
                        dry_run: false,
                        backup: false,
                    },
                )?;
                migration_applied = applied.applied;
                migration_warnings = applied.warnings;
                let optional_count = report
                    .findings
                    .iter()
                    .filter(|finding| finding.severity.eq_ignore_ascii_case("recommended"))
                    .count();
                if optional_count > 0 {
                    migration_warnings.push(format!(
                        "{optional_count} recommended migration finding(s) remain for manual review"
                    ));
                }
                BootstrapState::LegacyRepo
            } else {
                BootstrapState::ModernRepo
            }
        }
        Err(BacklogError::NotFound(_)) => {
            let project_id = normalize_project_id(
                options
                    .project_id
                    .as_deref()
                    .unwrap_or_else(|| repo_basename(&repo_root)),
            );
            let created = quickstart(
                &repo_root,
                &project_id,
                options.project_name.as_deref(),
                options.feature.as_deref(),
                options.agents_snippet,
            )?;
            quickstart_result = Some(created);
            BootstrapState::NewRepo
        }
    };

    let resolution = resolve_backlog(&repo_root)?;
    let backlog_dir = resolution.backlog_dir;
    let tasks = load_tasks(&backlog_dir);

    let project_id = options
        .project_id
        .as_deref()
        .map(normalize_project_id)
        .or_else(|| infer_project_id(&repo_root))
        .unwrap_or_else(|| resolve_project_id(&repo_root, &tasks, None));

    let mut context_seeded = false;
    if load_context(&backlog_dir).ok().flatten().is_none() {
        save_context(
            &backlog_dir,
            ContextState {
                version: 1,
                project_id: Some(project_id.clone()),
                objective: options
                    .objective
                    .as_deref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                scope: ContextScope {
                    mode: ContextScopeMode::None,
                    epic_id: None,
                    task_ids: Vec::new(),
                },
                updated_at: None,
            },
        )?;
        context_seeded = true;
    }

    let context_state = load_context(&backlog_dir).ok().flatten();
    let next_task_ids = recommend_next_tasks_with_context(&tasks, context_state.as_ref())
        .into_iter()
        .take(10)
        .map(|task| task.id.clone())
        .collect();

    let mut recommendations = Vec::new();
    if let Ok(worktrees) = list_git_worktrees(&repo_root) {
        if worktrees.len() <= 1 {
            recommendations.push(
                "For parallel streams, prefer one canonical repo + git worktrees (avoid sibling full clones)."
                    .to_string(),
            );
        }
    }

    Ok(BootstrapResult {
        repo_root: repo_root.clone(),
        backlog_dir: backlog_dir.clone(),
        state,
        project_id,
        context_seeded,
        context_path: context_path(&backlog_dir),
        quickstart: quickstart_result,
        migration_applied,
        migration_warnings,
        next_task_ids,
        recommendations,
    })
}

fn normalize_root(repo_root: &Path) -> PathBuf {
    if repo_root.is_absolute() {
        repo_root.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(repo_root)
    }
}

fn repo_basename(repo_root: &Path) -> &str {
    repo_root
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("project")
}

fn normalize_project_id(value: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in value.trim().chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };
        if mapped == '-' {
            if prev_dash {
                continue;
            }
            prev_dash = true;
            out.push('-');
        } else {
            prev_dash = false;
            out.push(mapped);
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_task(tasks_dir: &Path, id: &str, title: &str) {
        let path = tasks_dir.join(format!("{} - {}.md", id, title.to_lowercase()));
        fs::write(
            path,
            format!(
                "---\n\
id: {id}\n\
title: {title}\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
---\n\n\
Seed\n",
            ),
        )
        .expect("write task");
    }

    #[test]
    fn bootstrap_new_repo_runs_quickstart_and_context_seed() {
        let temp = TempDir::new().expect("tempdir");
        let result = bootstrap_repo(
            temp.path(),
            &BootstrapOptions {
                project_id: Some("alpha".to_string()),
                objective: Some("Ship alpha".to_string()),
                agents_snippet: true,
                ..BootstrapOptions::default()
            },
        )
        .expect("bootstrap");

        assert!(matches!(result.state, BootstrapState::NewRepo));
        assert!(result.quickstart.is_some());
        assert!(result.context_path.is_file());
        assert!(!result.next_task_ids.is_empty());
    }

    #[test]
    fn bootstrap_legacy_repo_migrates_to_workmesh_layout() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_tasks = temp.path().join("backlog").join("tasks");
        fs::create_dir_all(&backlog_tasks).expect("mkdir");
        write_task(&backlog_tasks, "task-001", "Legacy");

        let result = bootstrap_repo(temp.path(), &BootstrapOptions::default()).expect("bootstrap");
        assert!(matches!(result.state, BootstrapState::LegacyRepo));
        assert!(temp.path().join("workmesh").join("tasks").is_dir());
        assert!(!result.migration_applied.is_empty());
    }
}
