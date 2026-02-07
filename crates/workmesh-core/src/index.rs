use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::project::repo_root_from_backlog;
use crate::task::{load_tasks, Task};

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("Failed to access index: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to serialize index: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexEntry {
    pub id: String,
    pub uid: Option<String>,
    pub path: String,
    pub status: String,
    pub priority: String,
    pub phase: String,
    pub dependencies: Vec<String>,
    pub relationships: RelationshipsIndex,
    pub labels: Vec<String>,
    pub assignee: Vec<String>,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<String>,
    pub project: Option<String>,
    pub initiative: Option<String>,
    pub updated_date: Option<String>,
    pub mtime: i64,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RelationshipsIndex {
    pub blocked_by: Vec<String>,
    pub parent: Vec<String>,
    pub child: Vec<String>,
    pub discovered_from: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct IndexSummary {
    pub path: String,
    pub entries: usize,
}

#[derive(Debug, Serialize)]
pub struct IndexReport {
    pub ok: bool,
    pub missing: Vec<String>,
    pub stale: Vec<String>,
    pub extra: Vec<String>,
}

pub fn index_dir(backlog_dir: &Path) -> PathBuf {
    backlog_dir.join(".index")
}

pub fn index_path(backlog_dir: &Path) -> PathBuf {
    index_dir(backlog_dir).join("tasks.jsonl")
}

pub fn rebuild_index(backlog_dir: &Path) -> Result<IndexSummary, IndexError> {
    let entries = build_entries(backlog_dir)?;
    let path = index_path(backlog_dir);
    write_index(&path, &entries)?;
    Ok(IndexSummary {
        path: path.to_string_lossy().to_string(),
        entries: entries.len(),
    })
}

pub fn refresh_index(backlog_dir: &Path) -> Result<IndexSummary, IndexError> {
    let path = index_path(backlog_dir);
    if !path.exists() {
        return rebuild_index(backlog_dir);
    }
    let mut entries = read_index(&path)?;
    let mut entry_map: HashMap<String, IndexEntry> = entries
        .drain(..)
        .map(|entry| (entry.path.clone(), entry))
        .collect();

    let tasks = load_tasks(backlog_dir);
    let mut seen = HashSet::new();
    let repo_root = repo_root_from_backlog(backlog_dir);
    for task in tasks {
        let Some(task_path) = task.file_path.as_ref() else {
            continue;
        };
        let mtime = file_mtime(task_path)?;
        let hash = hash_file(task_path)?;
        let rel = normalize_rel_path(&repo_root, backlog_dir, task_path);
        let updated = build_entry(&task, rel.clone(), mtime, hash);
        entry_map.insert(rel.clone(), updated);
        seen.insert(rel);
    }

    entry_map.retain(|key, _| seen.contains(key));
    let mut updated_entries: Vec<IndexEntry> = entry_map.into_values().collect();
    sort_entries(&mut updated_entries);
    write_index(&path, &updated_entries)?;

    Ok(IndexSummary {
        path: path.to_string_lossy().to_string(),
        entries: updated_entries.len(),
    })
}

pub fn verify_index(backlog_dir: &Path) -> Result<IndexReport, IndexError> {
    let path = index_path(backlog_dir);
    if !path.exists() {
        return Ok(IndexReport {
            ok: false,
            missing: Vec::new(),
            stale: Vec::new(),
            extra: Vec::new(),
        });
    }
    let entries = read_index(&path)?;
    let entry_map: HashMap<String, IndexEntry> = entries
        .into_iter()
        .map(|entry| (entry.path.clone(), entry))
        .collect();

    let tasks = load_tasks(backlog_dir);
    let mut missing = Vec::new();
    let mut stale = Vec::new();
    let mut seen = HashSet::new();
    let repo_root = repo_root_from_backlog(backlog_dir);

    for task in tasks {
        let Some(task_path) = task.file_path.as_ref() else {
            continue;
        };
        let rel = normalize_rel_path(&repo_root, backlog_dir, task_path);
        seen.insert(rel.clone());
        let entry = match entry_map.get(&rel) {
            Some(entry) => entry,
            None => {
                missing.push(rel);
                continue;
            }
        };
        let hash = hash_file(task_path)?;
        if entry.hash != hash {
            stale.push(rel);
        }
    }

    let mut extra = Vec::new();
    for key in entry_map.keys() {
        if !seen.contains(key) {
            extra.push(key.clone());
        }
    }

    let ok = missing.is_empty() && stale.is_empty() && extra.is_empty();
    Ok(IndexReport {
        ok,
        missing,
        stale,
        extra,
    })
}

fn build_entries(backlog_dir: &Path) -> Result<Vec<IndexEntry>, IndexError> {
    let tasks = load_tasks(backlog_dir);
    let mut entries = Vec::new();
    let repo_root = repo_root_from_backlog(backlog_dir);
    for task in tasks {
        let Some(task_path) = task.file_path.as_ref() else {
            continue;
        };
        let mtime = file_mtime(task_path)?;
        let hash = hash_file(task_path)?;
        let rel = normalize_rel_path(&repo_root, backlog_dir, task_path);
        entries.push(build_entry(&task, rel, mtime, hash));
    }
    sort_entries(&mut entries);
    Ok(entries)
}

fn build_entry(task: &Task, rel_path: String, mtime: i64, hash: String) -> IndexEntry {
    IndexEntry {
        id: task.id.clone(),
        uid: task.uid.clone(),
        path: rel_path,
        status: task.status.clone(),
        priority: task.priority.clone(),
        phase: task.phase.clone(),
        dependencies: task.dependencies.clone(),
        relationships: RelationshipsIndex {
            blocked_by: task.relationships.blocked_by.clone(),
            parent: task.relationships.parent.clone(),
            child: task.relationships.child.clone(),
            discovered_from: task.relationships.discovered_from.clone(),
        },
        labels: task.labels.clone(),
        assignee: task.assignee.clone(),
        lease_owner: task.lease.as_ref().map(|lease| lease.owner.clone()),
        lease_expires_at: task
            .lease
            .as_ref()
            .and_then(|lease| lease.expires_at.clone()),
        project: task.project.clone(),
        initiative: task.initiative.clone(),
        updated_date: task.updated_date.clone(),
        mtime,
        hash,
    }
}

fn sort_entries(entries: &mut Vec<IndexEntry>) {
    entries.sort_by(|a, b| {
        let key_a = (&a.id, a.uid.as_deref().unwrap_or(""), &a.path);
        let key_b = (&b.id, b.uid.as_deref().unwrap_or(""), &b.path);
        key_a.cmp(&key_b)
    });
}

fn normalize_rel_path(repo_root: &Path, backlog_dir: &Path, task_path: &Path) -> String {
    // Prefer repo-root-relative paths so indexes never leak absolute user paths.
    // Fallbacks keep behavior stable in unusual layouts.
    let rel = task_path
        .strip_prefix(repo_root)
        .or_else(|_| task_path.strip_prefix(backlog_dir))
        .unwrap_or(task_path);
    rel.to_string_lossy().replace('\\', "/")
}

fn read_index(path: &Path) -> Result<Vec<IndexEntry>, IndexError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: IndexEntry = serde_json::from_str(&line)?;
        entries.push(entry);
    }
    Ok(entries)
}

fn write_index(path: &Path, entries: &[IndexEntry]) -> Result<(), IndexError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    for entry in entries {
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{}", line)?;
    }
    Ok(())
}

fn file_mtime(path: &Path) -> Result<i64, std::io::Error> {
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    Ok(to_unix_nanos(modified))
}

fn to_unix_nanos(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as i64)
        .unwrap_or(0)
}

fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok(format!("{:x}", digest))
}
