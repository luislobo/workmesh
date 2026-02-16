use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::backlog::locate_backlog_dir;
use crate::context::{context_from_legacy_focus, load_context, ContextScope, ContextState};
use crate::focus::load_focus;
use crate::global_sessions::WorktreeBinding;
use crate::global_sessions::{load_sessions_latest_fast, AgentSession};
use crate::storage::{
    cas_update_json_with_key, read_versioned_or_legacy_json, ResourceKey, StorageError,
    VersionedState,
};
use crate::task::load_tasks;
use crate::task_ops::recommend_next_tasks_with_context;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkstreamStatus {
    Active,
    Paused,
    Closed,
}

impl WorkstreamStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkstreamStatus::Active => "active",
            WorkstreamStatus::Paused => "paused",
            WorkstreamStatus::Closed => "closed",
        }
    }
}

impl Default for WorkstreamStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkstreamContextSnapshot {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub scope: ContextScope,
}

impl WorkstreamContextSnapshot {
    pub fn from_context_state(state: &ContextState) -> Self {
        Self {
            project_id: state.project_id.clone(),
            objective: state.objective.clone(),
            scope: state.scope.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamRecord {
    pub id: String,
    pub repo_root: String,
    #[serde(default)]
    pub key: Option<String>,
    pub name: String,
    #[serde(default)]
    pub status: WorkstreamStatus,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub worktree: Option<WorktreeBinding>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub context: Option<WorkstreamContextSnapshot>,
    #[serde(default)]
    pub truth_refs: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamRegistry {
    #[serde(default = "default_registry_schema_version")]
    pub version: u32,
    #[serde(default)]
    pub workstreams: Vec<WorkstreamRecord>,
}

impl Default for WorkstreamRegistry {
    fn default() -> Self {
        Self {
            version: default_registry_schema_version(),
            workstreams: Vec::new(),
        }
    }
}

fn default_registry_schema_version() -> u32 {
    1
}

pub fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

/// Resolve the canonical repo root used for workstream registry keys.
///
/// Workstreams should be visible from any git worktree checkout of the same repo. Using the
/// checkout path directly breaks that: every worktree has a different path.
///
/// Strategy:
/// - If `git` is available, compute the *common* git directory and return its parent (the canonical
///   main worktree path) when it looks like `<repo>/.git`.
/// - Otherwise, fall back to the provided checkout root.
pub fn resolve_repo_root_for_registry(checkout_root: &Path) -> PathBuf {
    if let Some(path) = git_common_dir_repo_root(checkout_root) {
        return path;
    }
    checkout_root.to_path_buf()
}

fn git_common_dir_repo_root(checkout_root: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(checkout_root)
        .arg("rev-parse")
        .arg("--absolute-git-dir")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let git_dir_raw = String::from_utf8_lossy(&output.stdout);
    let git_dir_str = git_dir_raw.trim();
    if git_dir_str.is_empty() {
        return None;
    }

    let git_dir = PathBuf::from(git_dir_str);
    let common_dir = match fs::read_to_string(git_dir.join("commondir")) {
        Ok(text) => {
            let rel = text.trim();
            if rel.is_empty() {
                git_dir.clone()
            } else {
                git_dir.join(rel)
            }
        }
        Err(_) => git_dir.clone(),
    };
    let common_dir = common_dir.canonicalize().unwrap_or(common_dir);
    if common_dir
        .file_name()
        .map(|name| name.to_string_lossy().eq_ignore_ascii_case(".git"))
        .unwrap_or(false)
    {
        return common_dir.parent().map(|parent| parent.to_path_buf());
    }
    Some(common_dir)
}

pub fn workstreams_registry_path(home: &Path) -> PathBuf {
    home.join("workstreams").join("registry.json")
}

pub fn load_workstream_registry(home: &Path) -> Result<WorkstreamRegistry> {
    let path = workstreams_registry_path(home);
    read_workstream_registry(&path).with_context(|| format!("load {}", path.display()))
}

fn read_workstream_registry(path: &Path) -> Result<WorkstreamRegistry> {
    let state = read_workstream_registry_state(path)?;
    let mut registry = state
        .map(|snapshot| snapshot.payload)
        .unwrap_or_else(WorkstreamRegistry::default);
    normalize_registry(&mut registry);
    Ok(registry)
}

fn read_workstream_registry_state(
    path: &Path,
) -> Result<Option<VersionedState<WorkstreamRegistry>>> {
    read_versioned_or_legacy_json::<WorkstreamRegistry>(path)
        .with_context(|| format!("read {}", path.display()))
}

fn load_workstream_registry_with_version(home: &Path) -> Result<(u64, WorkstreamRegistry)> {
    let path = workstreams_registry_path(home);
    let state = read_workstream_registry_state(&path)?;
    let expected_version = state.as_ref().map(|snapshot| snapshot.version).unwrap_or(0);
    let mut registry = state
        .map(|snapshot| snapshot.payload)
        .unwrap_or_else(WorkstreamRegistry::default);
    normalize_registry(&mut registry);
    Ok((expected_version, registry))
}

pub fn upsert_workstream_record(
    home: &Path,
    mut record: WorkstreamRecord,
) -> Result<WorkstreamRecord> {
    let path = workstreams_registry_path(home);
    record.repo_root = normalize_path_string(&resolve_repo_root_for_registry(Path::new(
        &record.repo_root,
    )))?;

    if let Some(binding) = record.worktree.as_mut() {
        binding.path = normalize_path_string(Path::new(&binding.path))?;
        if let Some(root) = binding.repo_root.as_mut() {
            *root = normalize_path_string(&resolve_repo_root_for_registry(Path::new(root)))?;
        }
    }

    if record.id.trim().is_empty() {
        record.id = Ulid::new().to_string();
    }

    if let Some(key) = record.key.as_mut() {
        let trimmed = key.trim().to_lowercase();
        if trimmed.is_empty() {
            record.key = None;
        } else {
            *key = trimmed;
        }
    }

    let lock_key = global_registry_key(home);
    const MAX_RETRIES: usize = 8;
    for _ in 0..MAX_RETRIES {
        let (expected_version, mut registry) = load_workstream_registry_with_version(home)?;
        let now = now_rfc3339();

        let updated = if let Some(index) = registry.workstreams.iter().position(|entry| {
            entry.id == record.id
                || record
                    .key
                    .as_ref()
                    .and_then(|key| entry.key.as_ref().map(|existing| (key, existing)))
                    .map(|(a, b)| a.eq_ignore_ascii_case(b) && entry.repo_root == record.repo_root)
                    .unwrap_or(false)
        }) {
            let existing = &mut registry.workstreams[index];
            let created_at = existing.created_at.clone();
            existing.repo_root = record.repo_root.clone();
            existing.key = record.key.clone();
            existing.name = record.name.clone();
            existing.status = record.status;
            existing.worktree = record.worktree.clone();
            existing.session_id = record.session_id.clone();
            existing.context = record.context.clone();
            existing.truth_refs = record.truth_refs.clone();
            existing.notes = record.notes.clone();
            existing.updated_at = now.clone();
            existing.created_at = created_at;
            existing.clone()
        } else {
            let mut inserted = record.clone();
            if inserted.created_at.trim().is_empty() {
                inserted.created_at = now.clone();
            }
            inserted.updated_at = now;
            registry.workstreams.push(inserted.clone());
            inserted
        };

        normalize_registry(&mut registry);
        match cas_update_json_with_key(&path, &lock_key, expected_version, registry.clone()) {
            Ok(_) => return Ok(updated),
            Err(StorageError::Conflict(_)) => continue,
            Err(err) => return Err(anyhow!(err).context("update workstream registry")),
        }
    }

    Err(anyhow!(
        "unable to upsert workstream record after repeated CAS conflicts"
    ))
}

pub fn remove_workstream_record(home: &Path, id: &str) -> Result<bool> {
    let path = workstreams_registry_path(home);
    let lock_key = global_registry_key(home);
    const MAX_RETRIES: usize = 8;
    for _ in 0..MAX_RETRIES {
        let (expected_version, mut registry) = load_workstream_registry_with_version(home)?;
        let before = registry.workstreams.len();
        registry.workstreams.retain(|record| record.id != id);
        if registry.workstreams.len() == before {
            return Ok(false);
        }
        normalize_registry(&mut registry);
        match cas_update_json_with_key(&path, &lock_key, expected_version, registry.clone()) {
            Ok(_) => return Ok(true),
            Err(StorageError::Conflict(_)) => continue,
            Err(err) => return Err(anyhow!(err).context("remove workstream record")),
        }
    }
    Err(anyhow!(
        "unable to remove workstream record after repeated CAS conflicts"
    ))
}

pub fn list_workstreams_for_repo(home: &Path, repo_root: &Path) -> Result<Vec<WorkstreamRecord>> {
    let registry = load_workstream_registry(home)?;
    let normalized_repo_root = normalize_path_string(&resolve_repo_root_for_registry(repo_root))?;
    Ok(registry
        .workstreams
        .into_iter()
        .filter(|record| {
            if record.repo_root.eq_ignore_ascii_case(&normalized_repo_root) {
                return true;
            }
            normalize_path_string(&resolve_repo_root_for_registry(Path::new(
                &record.repo_root,
            )))
            .map(|value| value.eq_ignore_ascii_case(&normalized_repo_root))
            .unwrap_or(false)
        })
        .collect())
}

pub fn find_workstream_for_repo_by_id(
    home: &Path,
    repo_root: &Path,
    id: &str,
) -> Result<Option<WorkstreamRecord>> {
    let id = id.trim();
    if id.is_empty() {
        return Ok(None);
    }
    let streams = list_workstreams_for_repo(home, repo_root)?;
    Ok(streams.into_iter().find(|record| record.id == id))
}

pub fn update_workstream_for_repo_by_id(
    home: &Path,
    repo_root: &Path,
    id: &str,
    mut update: impl FnMut(&mut WorkstreamRecord),
) -> Result<WorkstreamRecord> {
    let id = id.trim();
    if id.is_empty() {
        return Err(anyhow!("workstream id is blank"));
    }

    let normalized_repo_root = normalize_path_string(&resolve_repo_root_for_registry(repo_root))?;
    let path = workstreams_registry_path(home);
    let lock_key = global_registry_key(home);

    const MAX_RETRIES: usize = 8;
    for _ in 0..MAX_RETRIES {
        let (expected_version, mut registry) = load_workstream_registry_with_version(home)?;
        let index = registry
            .workstreams
            .iter()
            .enumerate()
            .find_map(|(idx, record)| {
                if record.id != id {
                    return None;
                }
                if record.repo_root.eq_ignore_ascii_case(&normalized_repo_root) {
                    return Some(idx);
                }
                let matches = normalize_path_string(&resolve_repo_root_for_registry(Path::new(
                    &record.repo_root,
                )))
                .map(|value| value.eq_ignore_ascii_case(&normalized_repo_root))
                .unwrap_or(false);
                if matches {
                    Some(idx)
                } else {
                    None
                }
            });
        let Some(index) = index else {
            return Err(anyhow!("workstream not found for repo: {}", id));
        };

        let existing = registry.workstreams[index].clone();
        let mut next = existing.clone();
        update(&mut next);

        next.repo_root = normalized_repo_root.clone();
        if let Some(binding) = next.worktree.as_mut() {
            binding.path = normalize_path_string(Path::new(&binding.path))?;
            if let Some(root) = binding.repo_root.as_mut() {
                *root = normalize_path_string(&resolve_repo_root_for_registry(Path::new(root)))?;
            }
        }

        if let Some(key) = next.key.as_mut() {
            let trimmed = key.trim().to_lowercase();
            if trimmed.is_empty() {
                next.key = None;
            } else {
                *key = trimmed;
            }
        }

        let now = now_rfc3339();
        next.updated_at = now.clone();
        if next.created_at.trim().is_empty() {
            next.created_at = now;
        }

        registry.workstreams[index] = next.clone();
        normalize_registry(&mut registry);

        match cas_update_json_with_key(&path, &lock_key, expected_version, registry.clone()) {
            Ok(_) => return Ok(next),
            Err(StorageError::Conflict(_)) => continue,
            Err(err) => return Err(anyhow!(err).context("update workstream registry")),
        }
    }

    Err(anyhow!(
        "unable to update workstream record after repeated CAS conflicts"
    ))
}

fn normalize_registry(registry: &mut WorkstreamRegistry) {
    registry.version = default_registry_schema_version();
    registry.workstreams.sort_by(|a, b| {
        a.repo_root
            .to_lowercase()
            .cmp(&b.repo_root.to_lowercase())
            .then_with(|| a.status.as_str().cmp(b.status.as_str()))
            .then_with(|| {
                a.key
                    .as_ref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default()
                    .cmp(&b.key.as_ref().map(|s| s.to_lowercase()).unwrap_or_default())
            })
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn global_registry_key(home: &Path) -> ResourceKey {
    ResourceKey::global(home, "workstreams.registry")
}

fn normalize_path_string(path: &Path) -> Result<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("resolve current dir")?
            .join(path)
    };
    let normalized = absolute.canonicalize().unwrap_or(absolute);
    Ok(normalized.to_string_lossy().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkstreamRestoreContextSource {
    ContextJson,
    FocusJson,
    WorkstreamSnapshot,
    None,
}

impl Default for WorkstreamRestoreContextSource {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkstreamRestoreContextView {
    #[serde(default)]
    pub source: WorkstreamRestoreContextSource,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub scope: ContextScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamRestoreNextTask {
    pub id: String,
    pub title: String,
    pub status: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamRestoreEntry {
    pub id: String,
    #[serde(default)]
    pub key: Option<String>,
    pub name: String,
    pub status: WorkstreamStatus,
    #[serde(default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub backlog_dir: Option<String>,
    #[serde(default)]
    pub context: WorkstreamRestoreContextView,
    #[serde(default)]
    pub next_task: Option<WorkstreamRestoreNextTask>,
    #[serde(default)]
    pub issues: Vec<String>,
    #[serde(default)]
    pub resume_script: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamRestorePlan {
    pub repo_root: String,
    pub registry_path: String,
    pub generated_at: String,
    #[serde(default)]
    pub include_inactive: bool,
    pub workstreams: Vec<WorkstreamRestoreEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkstreamRestoreOptions {
    /// Include paused/closed workstreams (default is active-only).
    #[serde(default)]
    pub include_inactive: bool,
}

fn best_session_for_worktree(sessions: &[AgentSession], worktree_path: &str) -> Option<String> {
    let path_norm = worktree_path.trim();
    if path_norm.is_empty() {
        return None;
    }
    sessions
        .iter()
        .find(|session| {
            session
                .worktree
                .as_ref()
                .map(|binding| binding.path == path_norm)
                .unwrap_or(false)
        })
        .map(|session| session.id.clone())
}

fn context_view_from_state(
    source: WorkstreamRestoreContextSource,
    state: &ContextState,
) -> WorkstreamRestoreContextView {
    WorkstreamRestoreContextView {
        source,
        project_id: state.project_id.clone(),
        objective: state.objective.clone(),
        scope: state.scope.clone(),
    }
}

fn context_state_from_snapshot(snapshot: &WorkstreamContextSnapshot) -> ContextState {
    ContextState {
        version: 1,
        project_id: snapshot.project_id.clone(),
        objective: snapshot.objective.clone(),
        workstream_id: None,
        scope: snapshot.scope.clone(),
        updated_at: None,
    }
}

fn resume_script_for_entry(entry: &WorkstreamRestoreEntry) -> Vec<String> {
    let mut script = Vec::new();
    if let Some(path) = entry.worktree_path.as_deref() {
        script.push(format!("cd {}", path));
        if let Some(session_id) = entry.session_id.as_deref() {
            script.push(format!(
                "workmesh --root . session resume {} --json",
                session_id
            ));
        }
        script.push("workmesh --root . context show --json".to_string());
        script.push("workmesh --root . next --json".to_string());
    }
    script
}

pub fn build_workstream_restore_plan(
    home: &Path,
    checkout_root: &Path,
    options: WorkstreamRestoreOptions,
) -> Result<WorkstreamRestorePlan> {
    let repo_key = resolve_repo_root_for_registry(checkout_root);
    let repo_root = normalize_path_string(&repo_key)?;
    let registry_path = workstreams_registry_path(home)
        .to_string_lossy()
        .to_string();

    let mut streams = list_workstreams_for_repo(home, &repo_key)?;
    if !options.include_inactive {
        streams.retain(|record| record.status == WorkstreamStatus::Active);
    }

    // Stable ordering independent of filesystem or JSON serialization.
    streams.sort_by(|a, b| {
        a.key
            .as_ref()
            .map(|s| s.to_lowercase())
            .unwrap_or_default()
            .cmp(&b.key.as_ref().map(|s| s.to_lowercase()).unwrap_or_default())
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });

    let sessions = load_sessions_latest_fast(home).unwrap_or_default();
    let session_ids: HashSet<String> = sessions.iter().map(|s| s.id.clone()).collect();

    let mut entries = Vec::new();
    for record in streams {
        let mut issues = Vec::new();

        let worktree_path = record
            .worktree
            .as_ref()
            .map(|w| w.path.clone())
            .filter(|value| !value.trim().is_empty());
        let branch = record.worktree.as_ref().and_then(|w| w.branch.clone());

        if let Some(path) = worktree_path.as_deref() {
            if !Path::new(path).exists() {
                issues.push(format!("missing_worktree_path: {}", path));
            }
        } else {
            issues.push("missing_worktree_path".to_string());
        }

        // Prefer the workstream pointer, but fall back to the most recent session bound to the
        // worktree path.
        let mut session_id = record.session_id.clone().filter(|id| !id.trim().is_empty());
        if session_id.is_none() {
            if let Some(path) = worktree_path.as_deref() {
                session_id = best_session_for_worktree(&sessions, path);
            }
        }

        if let Some(session_id) = session_id.as_deref() {
            if !session_ids.contains(session_id) {
                issues.push(format!("missing_session_id: {}", session_id));
            }
        }

        let mut backlog_dir_str = None;
        let mut context_view = WorkstreamRestoreContextView {
            source: WorkstreamRestoreContextSource::None,
            ..WorkstreamRestoreContextView::default()
        };
        let mut context_state: Option<ContextState> = None;

        if let Some(path) = worktree_path.as_deref() {
            match locate_backlog_dir(Path::new(path)) {
                Ok(backlog_dir) => {
                    backlog_dir_str = Some(backlog_dir.to_string_lossy().to_string());
                    if let Ok(Some(ctx)) = load_context(&backlog_dir) {
                        context_view = context_view_from_state(
                            WorkstreamRestoreContextSource::ContextJson,
                            &ctx,
                        );
                        context_state = Some(ctx);
                    } else if let Ok(Some(focus)) = load_focus(&backlog_dir) {
                        let converted = context_from_legacy_focus(
                            focus.project_id.clone(),
                            focus.epic_id.clone(),
                            focus.objective.clone(),
                            focus.working_set.clone(),
                        );
                        context_view = context_view_from_state(
                            WorkstreamRestoreContextSource::FocusJson,
                            &converted,
                        );
                        context_state = Some(converted);
                    } else if let Some(snapshot) = record.context.as_ref() {
                        let converted = context_state_from_snapshot(snapshot);
                        context_view = WorkstreamRestoreContextView {
                            source: WorkstreamRestoreContextSource::WorkstreamSnapshot,
                            project_id: snapshot.project_id.clone(),
                            objective: snapshot.objective.clone(),
                            scope: snapshot.scope.clone(),
                        };
                        context_state = Some(converted);
                    }
                }
                Err(_) => {
                    issues.push(format!("missing_backlog_dir_under: {}", path));
                }
            }
        }

        let next_task = if let Some(backlog_dir) = backlog_dir_str.as_deref().map(PathBuf::from) {
            let tasks = load_tasks(&backlog_dir);
            recommend_next_tasks_with_context(&tasks, context_state.as_ref())
                .first()
                .map(|task| WorkstreamRestoreNextTask {
                    id: task.id.clone(),
                    title: task.title.clone(),
                    status: task.status.clone(),
                    path: task
                        .file_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string()),
                })
        } else {
            None
        };

        let mut entry = WorkstreamRestoreEntry {
            id: record.id.clone(),
            key: record.key.clone(),
            name: record.name.clone(),
            status: record.status,
            worktree_path,
            branch,
            session_id,
            backlog_dir: backlog_dir_str,
            context: context_view,
            next_task,
            issues,
            resume_script: Vec::new(),
        };
        entry.resume_script = resume_script_for_entry(&entry);
        entries.push(entry);
    }

    Ok(WorkstreamRestorePlan {
        repo_root,
        registry_path,
        generated_at: now_rfc3339(),
        include_inactive: options.include_inactive,
        workstreams: entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use tempfile::TempDir;

    #[test]
    fn registry_round_trip_and_upsert() {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();
        let repo_root = home.join("repo");
        std::fs::create_dir_all(&repo_root).expect("repo dir");

        let record = WorkstreamRecord {
            id: "".to_string(),
            repo_root: repo_root.to_string_lossy().to_string(),
            key: Some("alpha".to_string()),
            name: "Alpha stream".to_string(),
            status: WorkstreamStatus::Active,
            created_at: "".to_string(),
            updated_at: "".to_string(),
            worktree: Some(WorktreeBinding {
                id: None,
                path: repo_root.to_string_lossy().to_string(),
                branch: Some("main".to_string()),
                repo_root: Some(repo_root.to_string_lossy().to_string()),
            }),
            session_id: None,
            context: Some(WorkstreamContextSnapshot {
                project_id: Some("workmesh".to_string()),
                objective: Some("Ship".to_string()),
                scope: ContextScope::default(),
            }),
            truth_refs: vec![],
            notes: None,
        };

        let inserted = upsert_workstream_record(home, record).expect("insert");
        assert!(!inserted.id.trim().is_empty());
        assert_eq!(inserted.key.as_deref(), Some("alpha"));
        assert_eq!(inserted.status, WorkstreamStatus::Active);

        let listed = list_workstreams_for_repo(home, &repo_root).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, inserted.id);

        let mut updated = listed[0].clone();
        updated.status = WorkstreamStatus::Paused;
        updated.notes = Some("paused".to_string());
        let upserted = upsert_workstream_record(home, updated).expect("update");
        assert_eq!(upserted.status, WorkstreamStatus::Paused);

        let listed_after = list_workstreams_for_repo(home, &repo_root).expect("list after");
        assert_eq!(listed_after.len(), 1);
        assert_eq!(listed_after[0].status, WorkstreamStatus::Paused);
        assert_eq!(listed_after[0].notes.as_deref(), Some("paused"));
    }

    #[test]
    fn upsert_is_safe_under_parallel_updates() {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();
        let repo_root = home.join("repo");
        std::fs::create_dir_all(&repo_root).expect("repo dir");

        let workers = 10usize;
        let barrier = Arc::new(Barrier::new(workers));
        let mut handles = Vec::new();

        for i in 0..workers {
            let barrier = Arc::clone(&barrier);
            let repo_root = repo_root.clone();
            let home = home.to_path_buf();
            handles.push(thread::spawn(move || {
                barrier.wait();
                let record = WorkstreamRecord {
                    id: "".to_string(),
                    repo_root: repo_root.to_string_lossy().to_string(),
                    key: Some(format!("ws{}", i)),
                    name: format!("Stream {}", i),
                    status: WorkstreamStatus::Active,
                    created_at: "".to_string(),
                    updated_at: "".to_string(),
                    worktree: None,
                    session_id: None,
                    context: None,
                    truth_refs: vec![],
                    notes: None,
                };
                upsert_workstream_record(&home, record).expect("upsert");
            }));
        }

        for handle in handles {
            handle.join().expect("join");
        }

        let listed = list_workstreams_for_repo(home, &repo_root).expect("list");
        assert_eq!(listed.len(), workers);
    }

    #[test]
    fn update_by_id_preserves_concurrent_field_updates() {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();
        let repo_root = home.join("repo");
        std::fs::create_dir_all(&repo_root).expect("repo dir");

        let inserted = upsert_workstream_record(
            home,
            WorkstreamRecord {
                id: "".to_string(),
                repo_root: repo_root.to_string_lossy().to_string(),
                key: Some("alpha".to_string()),
                name: "Alpha".to_string(),
                status: WorkstreamStatus::Active,
                created_at: "".to_string(),
                updated_at: "".to_string(),
                worktree: None,
                session_id: None,
                context: None,
                truth_refs: vec![],
                notes: None,
            },
        )
        .expect("insert");

        let id = inserted.id.clone();

        let workers = 12usize;
        let barrier = Arc::new(Barrier::new(workers));
        let mut handles = Vec::new();

        for i in 0..workers {
            let barrier = Arc::clone(&barrier);
            let repo_root = repo_root.clone();
            let home = home.to_path_buf();
            let id = id.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                match i % 3 {
                    0 => {
                        update_workstream_for_repo_by_id(&home, &repo_root, &id, |record| {
                            record.session_id = Some("session-1".to_string());
                        })
                        .expect("update session_id");
                    }
                    1 => {
                        update_workstream_for_repo_by_id(&home, &repo_root, &id, |record| {
                            record.notes = Some("notes-1".to_string());
                        })
                        .expect("update notes");
                    }
                    _ => {
                        update_workstream_for_repo_by_id(&home, &repo_root, &id, |record| {
                            if !record.truth_refs.iter().any(|t| t == "truth-1") {
                                record.truth_refs.push("truth-1".to_string());
                            }
                        })
                        .expect("update truth refs");
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().expect("join");
        }

        let updated = find_workstream_for_repo_by_id(home, &repo_root, &id)
            .expect("find")
            .expect("record");
        assert_eq!(updated.session_id.as_deref(), Some("session-1"));
        assert_eq!(updated.notes.as_deref(), Some("notes-1"));
        assert!(updated.truth_refs.iter().any(|t| t == "truth-1"));
    }

    #[test]
    fn restore_plan_falls_back_when_sessions_index_is_corrupt() {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();
        let repo_root = home.join("repo");
        std::fs::create_dir_all(repo_root.join("workmesh").join("tasks")).expect("tasks dir");

        // Seed one task so restore can compute a next-task suggestion.
        std::fs::write(
            repo_root
                .join("workmesh")
                .join("tasks")
                .join("task-001 - seed.md"),
            "---\nid: task-001\ntitle: Seed\nstatus: To Do\npriority: P2\nphase: Phase3\ndependencies: []\nlabels: []\nassignee: []\n---\n\n## Notes\n- seed\n",
        )
        .expect("write task");

        let stream = upsert_workstream_record(
            home,
            WorkstreamRecord {
                id: "".to_string(),
                repo_root: repo_root.to_string_lossy().to_string(),
                key: Some("alpha".to_string()),
                name: "Alpha".to_string(),
                status: WorkstreamStatus::Active,
                created_at: "".to_string(),
                updated_at: "".to_string(),
                worktree: Some(WorktreeBinding {
                    id: None,
                    path: repo_root.to_string_lossy().to_string(),
                    branch: None,
                    repo_root: Some(repo_root.to_string_lossy().to_string()),
                }),
                session_id: None,
                context: None,
                truth_refs: vec![],
                notes: None,
            },
        )
        .expect("insert stream");

        let session = AgentSession {
            id: "session-001".to_string(),
            created_at: now_rfc3339(),
            updated_at: now_rfc3339(),
            cwd: repo_root.to_string_lossy().to_string(),
            repo_root: Some(repo_root.to_string_lossy().to_string()),
            project_id: Some("demo".to_string()),
            epic_id: None,
            objective: "restore test".to_string(),
            working_set: Vec::new(),
            notes: None,
            git: None,
            checkpoint: None,
            recent_changes: None,
            handoff: None,
            worktree: Some(WorktreeBinding {
                id: None,
                path: repo_root.to_string_lossy().to_string(),
                branch: None,
                repo_root: Some(repo_root.to_string_lossy().to_string()),
            }),
            truth_refs: Vec::new(),
        };

        crate::global_sessions::append_session_saved(home, session.clone()).expect("save session");

        // Corrupt the sessions index so restore must fall back to session events.
        let index_path = crate::global_sessions::sessions_index_path(home);
        std::fs::write(&index_path, "not valid json\n").expect("write corrupt index");
        assert!(index_path.exists());

        let plan =
            build_workstream_restore_plan(home, &repo_root, WorkstreamRestoreOptions::default())
                .expect("restore plan");
        assert_eq!(plan.workstreams.len(), 1);
        assert_eq!(plan.workstreams[0].id, stream.id);
        assert_eq!(
            plan.workstreams[0].session_id.as_deref(),
            Some("session-001")
        );
        assert!(plan.workstreams[0].next_task.is_some());
    }
}
