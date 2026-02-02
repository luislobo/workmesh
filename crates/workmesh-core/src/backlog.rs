use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BacklogError {
    #[error("No tasks found under {0}")]
    NotFound(PathBuf),
}

pub fn resolve_backlog_dir(root: &Path) -> Result<PathBuf, BacklogError> {
    let root = root;
    if is_named(root, "tasks") && root.is_dir() {
        return Ok(root.parent().unwrap_or(root).to_path_buf());
    }
    if is_named(root, "backlog") && root.join("tasks").is_dir() {
        return Ok(root.to_path_buf());
    }
    if is_named(root, "project") && root.join("tasks").is_dir() {
        return Ok(root.to_path_buf());
    }
    if root.join("backlog").join("tasks").is_dir() {
        return Ok(root.join("backlog"));
    }
    if root.join("tasks").is_dir() {
        return Ok(root.to_path_buf());
    }
    if root.join("project").join("tasks").is_dir() {
        return Ok(root.join("project"));
    }
    Err(BacklogError::NotFound(root.to_path_buf()))
}

pub fn locate_backlog_dir(start: &Path) -> Result<PathBuf, BacklogError> {
    let start = start
        .canonicalize()
        .unwrap_or_else(|_| start.to_path_buf());
    for candidate in start.ancestors() {
        if is_named(candidate, "backlog") {
            return Ok(candidate.to_path_buf());
        }
        if is_named(candidate, "tasks") {
            return Ok(candidate.parent().unwrap_or(candidate).to_path_buf());
        }
        if candidate.join("backlog").join("tasks").is_dir() {
            return Ok(candidate.join("backlog"));
        }
        if candidate.join("tasks").is_dir() {
            return Ok(candidate.to_path_buf());
        }
        if candidate.join("project").join("tasks").is_dir() {
            return Ok(candidate.join("project"));
        }
    }
    Err(BacklogError::NotFound(start))
}

fn is_named(path: &Path, name: &str) -> bool {
    path.file_name()
        .map(|segment| segment.to_string_lossy().eq_ignore_ascii_case(name))
        .unwrap_or(false)
}
