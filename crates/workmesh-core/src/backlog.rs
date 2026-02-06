use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::{find_config_root, load_config, WorkmeshConfig};

#[derive(Debug, Error)]
pub enum BacklogError {
    #[error("No tasks found under {0}")]
    NotFound(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacklogLayout {
    Workmesh,
    HiddenWorkmesh,
    Backlog,
    Project,
    RootTasks,
    TasksDir,
    Custom,
}

impl BacklogLayout {
    pub fn is_legacy(self) -> bool {
        matches!(
            self,
            BacklogLayout::Backlog | BacklogLayout::Project | BacklogLayout::RootTasks | BacklogLayout::TasksDir
        )
    }
}

#[derive(Debug, Clone)]
pub struct BacklogResolution {
    pub backlog_dir: PathBuf,
    pub layout: BacklogLayout,
    pub repo_root: PathBuf,
    pub config: Option<WorkmeshConfig>,
}

pub fn resolve_backlog_dir(root: &Path) -> Result<PathBuf, BacklogError> {
    Ok(resolve_backlog(root)?.backlog_dir)
}

pub fn resolve_backlog(root: &Path) -> Result<BacklogResolution, BacklogError> {
    let repo_root = derive_repo_root(root);
    let config_root = find_config_root(root).unwrap_or_else(|| repo_root.clone());
    let config = load_config(&config_root);

    if let Some(resolution) = resolve_explicit_root(root, &config_root, config.as_ref()) {
        return Ok(resolution);
    }

    if let Some(resolution) = resolve_from_config(&config_root, config.as_ref()) {
        return Ok(resolution);
    }

    if let Some(resolution) = resolve_default_dirs(&config_root, config.as_ref()) {
        return Ok(resolution);
    }

    Err(BacklogError::NotFound(root.to_path_buf()))
}

pub fn locate_backlog_dir(start: &Path) -> Result<PathBuf, BacklogError> {
    let start = start
        .canonicalize()
        .unwrap_or_else(|_| start.to_path_buf());
    if let Some(config_root) = find_config_root(&start) {
        if let Ok(resolution) = resolve_backlog(&config_root) {
            return Ok(resolution.backlog_dir);
        }
    }
    for candidate in start.ancestors() {
        if is_named(candidate, "workmesh") && candidate.join("tasks").is_dir() {
            return Ok(candidate.to_path_buf());
        }
        if is_named(candidate, ".workmesh") && candidate.join("tasks").is_dir() {
            return Ok(candidate.to_path_buf());
        }
        if is_named(candidate, "backlog") && candidate.join("tasks").is_dir() {
            return Ok(candidate.to_path_buf());
        }
        if is_named(candidate, "project") && candidate.join("tasks").is_dir() {
            return Ok(candidate.to_path_buf());
        }
        if is_named(candidate, "tasks") {
            return Ok(candidate.parent().unwrap_or(candidate).to_path_buf());
        }
        if candidate.join("workmesh").join("tasks").is_dir() {
            return Ok(candidate.join("workmesh"));
        }
        if candidate.join(".workmesh").join("tasks").is_dir() {
            return Ok(candidate.join(".workmesh"));
        }
        if candidate.join("backlog").join("tasks").is_dir() {
            return Ok(candidate.join("backlog"));
        }
        if candidate.join("project").join("tasks").is_dir() {
            return Ok(candidate.join("project"));
        }
        if candidate.join("tasks").is_dir() {
            return Ok(candidate.to_path_buf());
        }
    }
    Err(BacklogError::NotFound(start))
}

fn resolve_explicit_root(
    root: &Path,
    repo_root: &Path,
    config: Option<&WorkmeshConfig>,
) -> Option<BacklogResolution> {
    if is_named(root, "tasks") && root.is_dir() {
        let parent = root.parent().unwrap_or(root).to_path_buf();
        let layout = layout_from_dir(&parent);
        return Some(BacklogResolution {
            backlog_dir: parent,
            layout,
            repo_root: repo_root.to_path_buf(),
            config: config.cloned(),
        });
    }
    if is_named(root, "workmesh") && root.join("tasks").is_dir() {
        return Some(resolution_for(root, BacklogLayout::Workmesh, repo_root, config));
    }
    if is_named(root, ".workmesh") && root.join("tasks").is_dir() {
        return Some(resolution_for(
            root,
            BacklogLayout::HiddenWorkmesh,
            repo_root,
            config,
        ));
    }
    if is_named(root, "backlog") && root.join("tasks").is_dir() {
        return Some(resolution_for(root, BacklogLayout::Backlog, repo_root, config));
    }
    if is_named(root, "project") && root.join("tasks").is_dir() {
        return Some(resolution_for(root, BacklogLayout::Project, repo_root, config));
    }
    if root.join("tasks").is_dir() {
        return Some(resolution_for(root, BacklogLayout::RootTasks, repo_root, config));
    }
    None
}

fn resolve_from_config(
    repo_root: &Path,
    config: Option<&WorkmeshConfig>,
) -> Option<BacklogResolution> {
    let root_dir = config
        .and_then(|cfg| cfg.root_dir.as_deref())
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())?;
    let candidate = repo_root.join(root_dir);
    if candidate.join("tasks").is_dir() {
        let layout = layout_from_dir(&candidate);
        return Some(resolution_for(&candidate, layout, repo_root, config));
    }
    None
}

fn resolve_default_dirs(
    repo_root: &Path,
    config: Option<&WorkmeshConfig>,
) -> Option<BacklogResolution> {
    let workmesh = repo_root.join("workmesh");
    if workmesh.join("tasks").is_dir() {
        return Some(resolution_for(&workmesh, BacklogLayout::Workmesh, repo_root, config));
    }
    let hidden = repo_root.join(".workmesh");
    if hidden.join("tasks").is_dir() {
        return Some(resolution_for(
            &hidden,
            BacklogLayout::HiddenWorkmesh,
            repo_root,
            config,
        ));
    }
    let backlog = repo_root.join("backlog");
    if backlog.join("tasks").is_dir() {
        return Some(resolution_for(&backlog, BacklogLayout::Backlog, repo_root, config));
    }
    let project = repo_root.join("project");
    if project.join("tasks").is_dir() {
        return Some(resolution_for(&project, BacklogLayout::Project, repo_root, config));
    }
    let tasks_root = repo_root.join("tasks");
    if tasks_root.is_dir() {
        return Some(resolution_for(repo_root, BacklogLayout::RootTasks, repo_root, config));
    }
    None
}

fn derive_repo_root(root: &Path) -> PathBuf {
    if is_named(root, "tasks")
        || is_named(root, "backlog")
        || is_named(root, "project")
        || is_named(root, "workmesh")
        || is_named(root, ".workmesh")
    {
        return root.parent().unwrap_or(root).to_path_buf();
    }
    root.to_path_buf()
}

fn layout_from_dir(dir: &Path) -> BacklogLayout {
    if is_named(dir, "workmesh") {
        BacklogLayout::Workmesh
    } else if is_named(dir, ".workmesh") {
        BacklogLayout::HiddenWorkmesh
    } else if is_named(dir, "backlog") {
        BacklogLayout::Backlog
    } else if is_named(dir, "project") {
        BacklogLayout::Project
    } else {
        BacklogLayout::RootTasks
    }
}

fn resolution_for(
    dir: &Path,
    layout: BacklogLayout,
    repo_root: &Path,
    config: Option<&WorkmeshConfig>,
) -> BacklogResolution {
    BacklogResolution {
        backlog_dir: dir.to_path_buf(),
        layout,
        repo_root: repo_root.to_path_buf(),
        config: config.cloned(),
    }
}

fn is_named(path: &Path, name: &str) -> bool {
    path.file_name()
        .map(|segment| segment.to_string_lossy().eq_ignore_ascii_case(name))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn prefers_workmesh_over_backlog() {
        let temp = TempDir::new().expect("tempdir");
        let workmesh = temp.path().join("workmesh").join("tasks");
        let backlog = temp.path().join("backlog").join("tasks");
        std::fs::create_dir_all(&workmesh).expect("workmesh");
        std::fs::create_dir_all(&backlog).expect("backlog");

        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::Workmesh);
        assert_eq!(resolution.backlog_dir, temp.path().join("workmesh"));
    }

    #[test]
    fn falls_back_to_backlog_when_only_legacy_exists() {
        let temp = TempDir::new().expect("tempdir");
        let backlog = temp.path().join("backlog").join("tasks");
        std::fs::create_dir_all(&backlog).expect("backlog");

        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::Backlog);
        assert_eq!(resolution.backlog_dir, temp.path().join("backlog"));
    }
}
