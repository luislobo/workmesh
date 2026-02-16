use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::context::ContextScope;
use crate::global_sessions::WorktreeBinding;
use crate::storage::{
    cas_update_json_with_key, read_versioned_or_legacy_json, ResourceKey, StorageError,
    VersionedState,
};

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

fn read_workstream_registry_state(path: &Path) -> Result<Option<VersionedState<WorkstreamRegistry>>> {
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

pub fn upsert_workstream_record(home: &Path, mut record: WorkstreamRecord) -> Result<WorkstreamRecord> {
    let path = workstreams_registry_path(home);
    record.repo_root = normalize_path_string(Path::new(&record.repo_root))?;

    if let Some(binding) = record.worktree.as_mut() {
        binding.path = normalize_path_string(Path::new(&binding.path))?;
        if let Some(root) = binding.repo_root.as_mut() {
            *root = normalize_path_string(Path::new(root))?;
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
    let normalized_repo_root = normalize_path_string(repo_root)?;
    Ok(registry
        .workstreams
        .into_iter()
        .filter(|record| record.repo_root.eq_ignore_ascii_case(&normalized_repo_root))
        .collect())
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
}

