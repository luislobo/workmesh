use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::storage::{
    cas_update_json_with_key, read_versioned_or_legacy_json, ResourceKey, StorageError,
    VersionedState,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitWorktreeEntry {
    pub path: String,
    #[serde(default)]
    pub head: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub detached: bool,
    #[serde(default)]
    pub bare: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub prunable: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeRecord {
    pub id: String,
    pub repo_root: String,
    pub path: String,
    #[serde(default)]
    pub branch: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub attached_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeRegistry {
    #[serde(default = "default_registry_version")]
    pub version: u32,
    #[serde(default)]
    pub worktrees: Vec<WorktreeRecord>,
}

impl Default for WorktreeRegistry {
    fn default() -> Self {
        Self {
            version: default_registry_version(),
            worktrees: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeView {
    #[serde(default)]
    pub id: Option<String>,
    pub path: String,
    #[serde(default)]
    pub repo_root: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub head: Option<String>,
    pub exists: bool,
    pub in_git: bool,
    pub source: Vec<String>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeDoctorReport {
    pub repo_root: String,
    pub registry_path: String,
    pub entries: Vec<WorktreeView>,
    pub issues: Vec<String>,
}

fn default_registry_version() -> u32 {
    1
}

pub fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

pub fn worktrees_registry_path(home: &Path) -> PathBuf {
    home.join("worktrees").join("registry.json")
}

pub fn load_worktree_registry(home: &Path) -> Result<WorktreeRegistry> {
    let path = worktrees_registry_path(home);
    read_worktree_registry(&path).with_context(|| format!("load {}", path.display()))
}

fn read_worktree_registry(path: &Path) -> Result<WorktreeRegistry> {
    let state = read_worktree_registry_state(path)?;
    let mut registry = state
        .map(|snapshot| snapshot.payload)
        .unwrap_or_else(WorktreeRegistry::default);
    normalize_registry(&mut registry);
    Ok(registry)
}

pub fn save_worktree_registry(home: &Path, mut registry: WorktreeRegistry) -> Result<PathBuf> {
    let path = worktrees_registry_path(home);
    normalize_registry(&mut registry);
    cas_retry_save_registry(home, registry)?;
    Ok(path)
}

pub fn find_worktree_record_by_path(home: &Path, path: &Path) -> Result<Option<WorktreeRecord>> {
    let registry = load_worktree_registry(home)?;
    let key = normalize_path_string(path)?;
    Ok(registry
        .worktrees
        .into_iter()
        .find(|record| record.path.eq_ignore_ascii_case(&key)))
}

pub fn upsert_worktree_record(home: &Path, mut record: WorktreeRecord) -> Result<WorktreeRecord> {
    let path = worktrees_registry_path(home);
    record.path = normalize_path_string(Path::new(&record.path))?;
    record.repo_root = normalize_path_string(Path::new(&record.repo_root))?;
    if record.id.trim().is_empty() {
        record.id = Ulid::new().to_string();
    }

    const MAX_RETRIES: usize = 8;
    let lock_key = global_registry_key(home);
    for _ in 0..MAX_RETRIES {
        let (expected_version, mut registry) = load_worktree_registry_with_version(home)?;
        let now = now_rfc3339();
        let updated = if let Some(index) = registry.worktrees.iter().position(|entry| {
            entry.id == record.id || entry.path.eq_ignore_ascii_case(&record.path)
        }) {
            let existing = &mut registry.worktrees[index];
            let created_at = existing.created_at.clone();
            existing.repo_root = record.repo_root.clone();
            existing.path = record.path.clone();
            existing.branch = record.branch.clone();
            existing.attached_session_id = record.attached_session_id.clone();
            existing.updated_at = now.clone();
            existing.created_at = created_at;
            existing.clone()
        } else {
            let mut inserted = record.clone();
            if inserted.created_at.trim().is_empty() {
                inserted.created_at = now.clone();
            }
            inserted.updated_at = now;
            registry.worktrees.push(inserted.clone());
            inserted
        };
        normalize_registry(&mut registry);
        match cas_update_json_with_key(&path, &lock_key, expected_version, registry.clone()) {
            Ok(_) => return Ok(updated),
            Err(StorageError::Conflict(_)) => continue,
            Err(err) => return Err(anyhow!(err).context("update worktree registry")),
        }
    }
    Err(anyhow!(
        "unable to upsert worktree record after repeated CAS conflicts"
    ))
}

pub fn remove_worktree_record(home: &Path, id: &str) -> Result<bool> {
    let path = worktrees_registry_path(home);
    let lock_key = global_registry_key(home);
    const MAX_RETRIES: usize = 8;
    for _ in 0..MAX_RETRIES {
        let (expected_version, mut registry) = load_worktree_registry_with_version(home)?;
        let before = registry.worktrees.len();
        registry.worktrees.retain(|record| record.id != id);
        if registry.worktrees.len() == before {
            return Ok(false);
        }
        normalize_registry(&mut registry);
        match cas_update_json_with_key(&path, &lock_key, expected_version, registry.clone()) {
            Ok(_) => return Ok(true),
            Err(StorageError::Conflict(_)) => continue,
            Err(err) => return Err(anyhow!(err).context("remove worktree record")),
        }
    }
    Err(anyhow!(
        "unable to remove worktree record after repeated CAS conflicts"
    ))
}

fn normalize_registry(registry: &mut WorktreeRegistry) {
    registry.version = default_registry_version();
    registry
        .worktrees
        .sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
}

fn read_worktree_registry_state(path: &Path) -> Result<Option<VersionedState<WorktreeRegistry>>> {
    read_versioned_or_legacy_json::<WorktreeRegistry>(path)
        .with_context(|| format!("read {}", path.display()))
}

fn load_worktree_registry_with_version(home: &Path) -> Result<(u64, WorktreeRegistry)> {
    let path = worktrees_registry_path(home);
    let state = read_worktree_registry_state(&path)?;
    let expected_version = state.as_ref().map(|snapshot| snapshot.version).unwrap_or(0);
    let mut registry = state
        .map(|snapshot| snapshot.payload)
        .unwrap_or_else(WorktreeRegistry::default);
    normalize_registry(&mut registry);
    Ok((expected_version, registry))
}

fn cas_retry_save_registry(home: &Path, mut registry: WorktreeRegistry) -> Result<()> {
    let path = worktrees_registry_path(home);
    let lock_key = global_registry_key(home);
    const MAX_RETRIES: usize = 8;
    for _ in 0..MAX_RETRIES {
        let (expected_version, _) = load_worktree_registry_with_version(home)?;
        normalize_registry(&mut registry);
        match cas_update_json_with_key(&path, &lock_key, expected_version, registry.clone()) {
            Ok(_) => return Ok(()),
            Err(StorageError::Conflict(_)) => continue,
            Err(err) => return Err(anyhow!(err).context("save worktree registry")),
        }
    }
    Err(anyhow!(
        "unable to save worktree registry after repeated CAS conflicts"
    ))
}

fn global_registry_key(home: &Path) -> ResourceKey {
    ResourceKey::global(home, "worktrees.registry")
}

pub fn list_git_worktrees(repo_root: &Path) -> Result<Vec<GitWorktreeEntry>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("worktree")
        .arg("list")
        .arg("--porcelain")
        .output()
        .with_context(|| format!("run git worktree list under {}", repo_root.display()))?;
    if !output.status.success() {
        return Err(anyhow!(
            "git worktree list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_git_worktree_list(&String::from_utf8_lossy(&output.stdout), Some(repo_root))
}

pub fn create_git_worktree(
    repo_root: &Path,
    path: &Path,
    branch: &str,
    from_ref: Option<&str>,
) -> Result<GitWorktreeEntry> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo_root)
        .arg("worktree")
        .arg("add")
        .arg("-b")
        .arg(branch)
        .arg(path);
    if let Some(from_ref) = from_ref {
        let trimmed = from_ref.trim();
        if !trimmed.is_empty() {
            cmd.arg(trimmed);
        }
    }
    let output = cmd
        .output()
        .with_context(|| format!("run git worktree add under {}", repo_root.display()))?;
    if !output.status.success() {
        return Err(anyhow!(
            "git worktree add failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let normalized = normalize_path_string(path)?;
    let entries = list_git_worktrees(repo_root)?;
    if let Some(entry) = entries
        .into_iter()
        .find(|entry| entry.path.eq_ignore_ascii_case(&normalized))
    {
        return Ok(entry);
    }
    Ok(GitWorktreeEntry {
        path: normalized,
        head: None,
        branch: Some(branch.trim().to_string()),
        detached: false,
        bare: false,
        locked: false,
        prunable: None,
    })
}

pub fn list_worktree_views(repo_root: &Path, home: &Path) -> Result<Vec<WorktreeView>> {
    let repo_root_norm = normalize_path_string(repo_root)?;
    let registry = load_worktree_registry(home)?;
    let git_entries = list_git_worktrees(repo_root).unwrap_or_default();

    let mut by_path: BTreeMap<String, WorktreeView> = BTreeMap::new();
    for git_entry in git_entries {
        let key = normalize_path_string(Path::new(&git_entry.path)).unwrap_or(git_entry.path);
        by_path.insert(
            key.clone(),
            WorktreeView {
                id: None,
                path: key,
                repo_root: Some(repo_root_norm.clone()),
                branch: git_entry.branch,
                head: git_entry.head,
                exists: true,
                in_git: true,
                source: vec!["git".to_string()],
                issues: Vec::new(),
            },
        );
    }

    for record in registry
        .worktrees
        .into_iter()
        .filter(|record| record.repo_root.eq_ignore_ascii_case(&repo_root_norm))
    {
        let key = record.path.clone();
        let existing = by_path.remove(&key);
        let mut source = vec!["registry".to_string()];
        let mut in_git = false;
        let mut head = None;
        let mut branch = record.branch.clone();
        if let Some(existing) = existing {
            in_git = existing.in_git;
            head = existing.head.clone();
            if branch.is_none() {
                branch = existing.branch.clone();
            }
            source.extend(existing.source);
        }
        source.sort();
        source.dedup();
        by_path.insert(
            key.clone(),
            WorktreeView {
                id: Some(record.id),
                path: key,
                repo_root: Some(record.repo_root),
                branch,
                head,
                exists: Path::new(&record.path).exists(),
                in_git,
                source,
                issues: Vec::new(),
            },
        );
    }

    let mut entries: Vec<WorktreeView> = by_path.into_values().collect();
    for entry in &mut entries {
        let mut issues = BTreeSet::new();
        if !entry.exists {
            issues.insert("path_missing".to_string());
        }
        if entry.source.iter().any(|src| src == "registry") && !entry.in_git {
            issues.insert("not_in_git_worktree_list".to_string());
        }
        entry.issues = issues.into_iter().collect();
    }
    entries.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
    Ok(entries)
}

pub fn doctor_worktrees(repo_root: &Path, home: &Path) -> Result<WorktreeDoctorReport> {
    let entries = list_worktree_views(repo_root, home)?;
    let mut issues = Vec::new();
    for entry in &entries {
        for issue in &entry.issues {
            issues.push(format!("{}: {}", entry.path, issue));
        }
    }
    Ok(WorktreeDoctorReport {
        repo_root: normalize_path_string(repo_root)?,
        registry_path: worktrees_registry_path(home).to_string_lossy().to_string(),
        entries,
        issues,
    })
}

pub fn current_branch(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() || raw == "HEAD" {
        None
    } else {
        Some(raw)
    }
}

fn parse_git_worktree_list(raw: &str, repo_root: Option<&Path>) -> Result<Vec<GitWorktreeEntry>> {
    let mut entries = Vec::new();
    let mut current: Option<GitWorktreeEntry> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("worktree ") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let path = match repo_root {
                Some(root) => normalize_path_string(&root.join(value))
                    .or_else(|_| normalize_path_string(Path::new(value)))?,
                None => normalize_path_string(Path::new(value))?,
            };
            current = Some(GitWorktreeEntry {
                path,
                head: None,
                branch: None,
                detached: false,
                bare: false,
                locked: false,
                prunable: None,
            });
            continue;
        }
        let Some(entry) = current.as_mut() else {
            continue;
        };
        if let Some(value) = trimmed.strip_prefix("HEAD ") {
            entry.head = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("branch ") {
            entry.branch = Some(strip_branch_ref(value.trim()));
        } else if trimmed == "detached" {
            entry.detached = true;
        } else if trimmed == "bare" {
            entry.bare = true;
        } else if trimmed.starts_with("locked") {
            entry.locked = true;
        } else if let Some(value) = trimmed.strip_prefix("prunable ") {
            entry.prunable = Some(value.trim().to_string());
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    Ok(entries)
}

fn strip_branch_ref(value: &str) -> String {
    let trimmed = value.trim();
    trimmed
        .strip_prefix("refs/heads/")
        .unwrap_or(trimmed)
        .to_string()
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
    use std::fs;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use tempfile::TempDir;

    #[test]
    fn parse_git_worktree_list_parses_entries() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        let raw = "\
worktree /repo/main
HEAD abcdef
branch refs/heads/main

worktree /repo/feature
HEAD 123456
branch refs/heads/feature/x
locked
";
        let parsed = parse_git_worktree_list(raw, Some(root)).expect("parse");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].branch.as_deref(), Some("main"));
        assert_eq!(parsed[1].branch.as_deref(), Some("feature/x"));
        assert!(parsed[1].locked);
    }

    #[test]
    fn registry_round_trip_and_upsert() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path().join("repo");
        let wt = temp.path().join("repo-wt");
        fs::create_dir_all(&repo).expect("repo");
        fs::create_dir_all(&wt).expect("wt");

        let created = upsert_worktree_record(
            temp.path(),
            WorktreeRecord {
                id: String::new(),
                repo_root: repo.to_string_lossy().to_string(),
                path: wt.to_string_lossy().to_string(),
                branch: Some("feature/test".to_string()),
                created_at: String::new(),
                updated_at: String::new(),
                attached_session_id: None,
            },
        )
        .expect("upsert");
        assert!(!created.id.is_empty());

        let loaded = load_worktree_registry(temp.path()).expect("load");
        assert_eq!(loaded.worktrees.len(), 1);
        assert_eq!(loaded.worktrees[0].branch.as_deref(), Some("feature/test"));

        let found = find_worktree_record_by_path(temp.path(), &wt)
            .expect("find")
            .expect("record");
        assert_eq!(found.id, created.id);
    }

    #[test]
    fn upsert_is_safe_under_parallel_updates() {
        let temp = TempDir::new().expect("tempdir");
        let home = Arc::new(temp.path().to_path_buf());
        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");

        let workers = 6usize;
        let barrier = Arc::new(Barrier::new(workers));
        let mut handles = Vec::new();
        for i in 0..workers {
            let home = Arc::clone(&home);
            let barrier = Arc::clone(&barrier);
            let wt = temp.path().join(format!("repo-wt-{}", i));
            fs::create_dir_all(&wt).expect("wt");
            let repo = repo.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                upsert_worktree_record(
                    home.as_path(),
                    WorktreeRecord {
                        id: String::new(),
                        repo_root: repo.to_string_lossy().to_string(),
                        path: wt.to_string_lossy().to_string(),
                        branch: Some(format!("feature/{}", i)),
                        created_at: String::new(),
                        updated_at: String::new(),
                        attached_session_id: None,
                    },
                )
                .expect("upsert");
            }));
        }

        for handle in handles {
            handle.join().expect("join");
        }

        let loaded = load_worktree_registry(home.as_path()).expect("load");
        assert_eq!(loaded.worktrees.len(), workers);
    }
}
