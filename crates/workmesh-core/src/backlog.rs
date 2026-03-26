use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::{find_config_root, load_config, WorkmeshConfig};
use crate::project::write_repo_root_metadata;

#[derive(Debug, Error)]
pub enum BacklogError {
    #[error("No tasks found under {0}")]
    NotFound(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacklogLayout {
    Split,
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
            BacklogLayout::Backlog
                | BacklogLayout::Project
                | BacklogLayout::RootTasks
                | BacklogLayout::TasksDir
        )
    }
}

#[derive(Debug, Clone)]
pub struct BacklogResolution {
    pub state_root: PathBuf,
    pub tasks_root: PathBuf,
    pub layout: BacklogLayout,
    pub repo_root: PathBuf,
    pub config: Option<WorkmeshConfig>,
}

impl BacklogResolution {
    pub fn backlog_dir(&self) -> &Path {
        &self.state_root
    }
}

pub fn resolve_backlog_dir(root: &Path) -> Result<PathBuf, BacklogError> {
    Ok(resolve_backlog(root)?.state_root)
}

pub fn resolve_tasks_dir(root: &Path) -> Result<PathBuf, BacklogError> {
    Ok(resolve_backlog(root)?.tasks_root)
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
    let start = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    if let Some(config_root) = find_config_root(&start) {
        if let Ok(resolution) = resolve_backlog(&config_root) {
            return Ok(resolution.state_root);
        }
    }
    for candidate in start.ancestors() {
        if is_named(candidate, ".workmesh") && candidate.parent().is_some() {
            let repo_root = candidate.parent().unwrap_or(candidate);
            if repo_root.join("tasks").is_dir() || candidate.join("tasks").is_dir() {
                return Ok(candidate.to_path_buf());
            }
        }
        if is_named(candidate, "workmesh") && candidate.join("tasks").is_dir() {
            return Ok(candidate.to_path_buf());
        }
        if is_named(candidate, "backlog") && candidate.join("tasks").is_dir() {
            return Ok(candidate.to_path_buf());
        }
        if is_named(candidate, "project") && candidate.join("tasks").is_dir() {
            return Ok(candidate.to_path_buf());
        }
        if is_named(candidate, "tasks") {
            let parent = candidate.parent().unwrap_or(candidate);
            if parent.join(".workmesh").is_dir() {
                return Ok(parent.join(".workmesh"));
            }
            if is_named(parent, "workmesh")
                || is_named(parent, ".workmesh")
                || is_named(parent, "backlog")
                || is_named(parent, "project")
            {
                return Ok(parent.to_path_buf());
            }
            return Ok(parent.to_path_buf());
        }
        if candidate.join(".workmesh").is_dir() && candidate.join("tasks").is_dir() {
            return Ok(candidate.join(".workmesh"));
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
    if let Some((state_root, tasks_root)) = configured_roots(repo_root, config) {
        if path_matches(root, &state_root)
            || path_matches(root, &tasks_root)
            || path_matches(root, repo_root)
        {
            if tasks_root.is_dir() || state_root.is_dir() {
                return Some(resolution_for_roots(
                    &state_root,
                    &tasks_root,
                    repo_root,
                    config,
                ));
            }
        }
    }

    if is_named(root, "tasks") && root.is_dir() {
        let parent = root.parent().unwrap_or(root).to_path_buf();
        if is_named(&parent, "workmesh")
            || is_named(&parent, ".workmesh")
            || is_named(&parent, "backlog")
            || is_named(&parent, "project")
        {
            return Some(resolution_for_roots(
                root.parent().unwrap_or(root),
                root,
                repo_root,
                config,
            ));
        }
        let split_state = parent.join(".workmesh");
        if split_state.is_dir() {
            return Some(resolution_for_roots(&split_state, root, repo_root, config));
        }
        return Some(resolution_for_roots(&parent, root, repo_root, config));
    }
    if is_named(root, "workmesh") && root.join("tasks").is_dir() {
        return Some(resolution_for_roots(
            root,
            &root.join("tasks"),
            repo_root,
            config,
        ));
    }
    if is_named(root, ".workmesh") && root.join("tasks").is_dir() {
        return Some(resolution_for_roots(
            root,
            &root.join("tasks"),
            repo_root,
            config,
        ));
    }
    if is_named(root, ".workmesh") && root.parent().unwrap_or(root).join("tasks").is_dir() {
        return Some(resolution_for_roots(
            root,
            &root.parent().unwrap_or(root).join("tasks"),
            repo_root,
            config,
        ));
    }
    if is_named(root, "backlog") && root.join("tasks").is_dir() {
        return Some(resolution_for_roots(
            root,
            &root.join("tasks"),
            repo_root,
            config,
        ));
    }
    if is_named(root, "project") && root.join("tasks").is_dir() {
        return Some(resolution_for_roots(
            root,
            &root.join("tasks"),
            repo_root,
            config,
        ));
    }
    if root.join("tasks").is_dir() {
        let split_state = root.join(".workmesh");
        if split_state.is_dir() {
            return Some(resolution_for_roots(
                &split_state,
                &root.join("tasks"),
                repo_root,
                config,
            ));
        }
        return Some(resolution_for_roots(
            root,
            &root.join("tasks"),
            repo_root,
            config,
        ));
    }
    None
}

fn resolve_from_config(
    repo_root: &Path,
    config: Option<&WorkmeshConfig>,
) -> Option<BacklogResolution> {
    let (state_root, tasks_root) = configured_roots(repo_root, config)?;
    if tasks_root.is_dir() || state_root.is_dir() {
        if state_root.is_dir() && !state_root.starts_with(repo_root) {
            let _ = write_repo_root_metadata(&state_root, repo_root);
        }
        return Some(resolution_for_roots(
            &state_root,
            &tasks_root,
            repo_root,
            config,
        ));
    }
    None
}

fn resolve_default_dirs(
    repo_root: &Path,
    config: Option<&WorkmeshConfig>,
) -> Option<BacklogResolution> {
    let split_state = repo_root.join(".workmesh");
    let split_tasks = repo_root.join("tasks");
    if split_state.is_dir() && split_tasks.is_dir() {
        return Some(resolution_for_roots(
            &split_state,
            &split_tasks,
            repo_root,
            config,
        ));
    }

    let workmesh = repo_root.join("workmesh");
    if workmesh.join("tasks").is_dir() {
        return Some(resolution_for_roots(
            &workmesh,
            &workmesh.join("tasks"),
            repo_root,
            config,
        ));
    }
    let hidden = repo_root.join(".workmesh");
    if hidden.join("tasks").is_dir() {
        return Some(resolution_for_roots(
            &hidden,
            &hidden.join("tasks"),
            repo_root,
            config,
        ));
    }
    let backlog = repo_root.join("backlog");
    if backlog.join("tasks").is_dir() {
        return Some(resolution_for_roots(
            &backlog,
            &backlog.join("tasks"),
            repo_root,
            config,
        ));
    }
    let project = repo_root.join("project");
    if project.join("tasks").is_dir() {
        return Some(resolution_for_roots(
            &project,
            &project.join("tasks"),
            repo_root,
            config,
        ));
    }
    if split_tasks.is_dir() {
        return Some(resolution_for_roots(
            repo_root,
            &split_tasks,
            repo_root,
            config,
        ));
    }
    None
}

fn configured_roots(
    repo_root: &Path,
    config: Option<&WorkmeshConfig>,
) -> Option<(PathBuf, PathBuf)> {
    let config = config?;
    let legacy_root =
        trim_config_value(config.root_dir.as_deref()).map(|value| rooted_path(repo_root, value));
    let explicit_tasks =
        trim_config_value(config.tasks_root.as_deref()).map(|value| rooted_path(repo_root, value));
    let explicit_state =
        trim_config_value(config.state_root.as_deref()).map(|value| rooted_path(repo_root, value));

    if legacy_root.is_none() && explicit_tasks.is_none() && explicit_state.is_none() {
        return None;
    }

    let tasks_root = explicit_tasks.unwrap_or_else(|| {
        legacy_root
            .as_ref()
            .map(|path| path.join("tasks"))
            .unwrap_or_else(|| repo_root.join("tasks"))
    });
    let state_root = explicit_state.unwrap_or_else(|| {
        legacy_root
            .clone()
            .unwrap_or_else(|| repo_root.join(".workmesh"))
    });

    Some((state_root, tasks_root))
}

fn trim_config_value(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn rooted_path(repo_root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    }
}

fn derive_repo_root(root: &Path) -> PathBuf {
    if is_named(root, "tasks")
        || is_named(root, "backlog")
        || is_named(root, "project")
        || is_named(root, "workmesh")
        || is_named(root, ".workmesh")
    {
        if is_named(root, "tasks") {
            let parent = root.parent().unwrap_or(root);
            if is_named(parent, "workmesh")
                || is_named(parent, ".workmesh")
                || is_named(parent, "backlog")
                || is_named(parent, "project")
            {
                return parent.parent().unwrap_or(parent).to_path_buf();
            }
            return parent.to_path_buf();
        }
        return root.parent().unwrap_or(root).to_path_buf();
    }
    root.to_path_buf()
}

fn resolution_for_roots(
    state_root: &Path,
    tasks_root: &Path,
    repo_root: &Path,
    config: Option<&WorkmeshConfig>,
) -> BacklogResolution {
    BacklogResolution {
        state_root: state_root.to_path_buf(),
        tasks_root: tasks_root.to_path_buf(),
        layout: layout_from_roots(state_root, tasks_root, repo_root),
        repo_root: repo_root.to_path_buf(),
        config: config.cloned(),
    }
}

fn layout_from_roots(state_root: &Path, tasks_root: &Path, repo_root: &Path) -> BacklogLayout {
    if state_root == repo_root.join(".workmesh") && tasks_root == repo_root.join("tasks") {
        return BacklogLayout::Split;
    }
    if tasks_root == state_root.join("tasks") {
        if is_named(state_root, "workmesh") {
            BacklogLayout::Workmesh
        } else if is_named(state_root, ".workmesh") {
            BacklogLayout::HiddenWorkmesh
        } else if is_named(state_root, "backlog") {
            BacklogLayout::Backlog
        } else if is_named(state_root, "project") {
            BacklogLayout::Project
        } else if state_root == repo_root {
            BacklogLayout::RootTasks
        } else {
            BacklogLayout::Custom
        }
    } else if is_named(tasks_root, "tasks") {
        BacklogLayout::TasksDir
    } else {
        BacklogLayout::Custom
    }
}

fn path_matches(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .map(|(a, b)| a == b)
            .unwrap_or(false)
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

    fn canon(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }

    #[test]
    fn prefers_split_layout_over_legacy_single_root() {
        let temp = TempDir::new().expect("tempdir");
        std::fs::create_dir_all(temp.path().join(".workmesh")).expect("state");
        std::fs::create_dir_all(temp.path().join("tasks")).expect("tasks");
        std::fs::create_dir_all(temp.path().join("backlog").join("tasks")).expect("backlog");

        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::Split);
        assert_eq!(resolution.state_root, temp.path().join(".workmesh"));
        assert_eq!(resolution.tasks_root, temp.path().join("tasks"));
    }

    #[test]
    fn falls_back_to_backlog_when_only_legacy_exists() {
        let temp = TempDir::new().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("backlog").join("tasks")).expect("backlog");

        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::Backlog);
        assert_eq!(resolution.state_root, temp.path().join("backlog"));
        assert_eq!(
            resolution.tasks_root,
            temp.path().join("backlog").join("tasks")
        );
    }

    #[test]
    fn resolve_backlog_accepts_explicit_tasks_dir() {
        let temp = TempDir::new().expect("tempdir");
        let tasks_dir = temp.path().join("tasks");
        std::fs::create_dir_all(&tasks_dir).expect("tasks");
        std::fs::create_dir_all(temp.path().join(".workmesh")).expect("state");

        let resolution = resolve_backlog(&tasks_dir).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::Split);
        assert_eq!(resolution.state_root, temp.path().join(".workmesh"));
        assert_eq!(resolution.tasks_root, tasks_dir);
        assert_eq!(resolution.repo_root, temp.path().to_path_buf());
    }

    #[test]
    fn resolve_backlog_falls_back_to_hidden_workmesh_then_project_then_root_tasks() {
        let temp = TempDir::new().expect("tempdir");
        let hidden = temp.path().join(".workmesh").join("tasks");
        std::fs::create_dir_all(&hidden).expect("hidden");
        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::HiddenWorkmesh);

        let temp2 = TempDir::new().expect("tempdir");
        let project = temp2.path().join("project").join("tasks");
        std::fs::create_dir_all(&project).expect("project");
        let resolution = resolve_backlog(temp2.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::Project);

        let temp3 = TempDir::new().expect("tempdir");
        std::fs::create_dir_all(temp3.path().join("tasks")).expect("root tasks");
        let resolution = resolve_backlog(temp3.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::RootTasks);
        assert_eq!(resolution.state_root, temp3.path().to_path_buf());
    }

    #[test]
    fn resolve_backlog_uses_config_split_root_override() {
        let temp = TempDir::new().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("state")).expect("state");
        std::fs::create_dir_all(temp.path().join("tracker")).expect("tasks");
        std::fs::write(
            temp.path().join(".workmesh.toml"),
            "state_root = \"state\"\ntasks_root = \"tracker\"\n",
        )
        .expect("config");

        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::Custom);
        assert_eq!(
            canon(&resolution.state_root),
            canon(&temp.path().join("state"))
        );
        assert_eq!(
            canon(&resolution.tasks_root),
            canon(&temp.path().join("tracker"))
        );
        assert!(resolution.config.is_some());
    }

    #[test]
    fn locate_backlog_dir_prefers_config_root_when_present() {
        let temp = TempDir::new().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("state")).expect("state");
        std::fs::create_dir_all(temp.path().join("tracker")).expect("tasks");
        std::fs::write(
            temp.path().join(".workmesh.toml"),
            "state_root = \"state\"\ntasks_root = \"tracker\"\n",
        )
        .expect("config");

        let deep = temp.path().join("src").join("pkg");
        std::fs::create_dir_all(&deep).expect("deep");
        let located = locate_backlog_dir(&deep).expect("locate");
        assert_eq!(canon(&located), canon(&temp.path().join("state")));
    }

    #[test]
    fn backlog_layout_is_legacy_matches_expected() {
        assert!(BacklogLayout::Backlog.is_legacy());
        assert!(BacklogLayout::Project.is_legacy());
        assert!(BacklogLayout::RootTasks.is_legacy());
        assert!(BacklogLayout::TasksDir.is_legacy());
        assert!(!BacklogLayout::Split.is_legacy());
        assert!(!BacklogLayout::Workmesh.is_legacy());
        assert!(!BacklogLayout::HiddenWorkmesh.is_legacy());
        assert!(!BacklogLayout::Custom.is_legacy());
    }
}
