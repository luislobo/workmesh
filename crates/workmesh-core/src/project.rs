use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::find_config_root;

const REPO_ROOT_MARKER: &str = ".repo-root";

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("Project id is required")]
    MissingId,
    #[error("Project id contains invalid path characters: {0}")]
    InvalidId(String),
    #[error("Failed to create project docs: {0}")]
    Io(#[from] std::io::Error),
}

pub fn repo_root_from_state_root(state_root: &Path) -> PathBuf {
    if let Some(repo_root) = find_config_root(state_root) {
        return repo_root;
    }
    if let Some(repo_root) = read_repo_root_metadata(state_root) {
        return repo_root;
    }
    let name = state_root
        .file_name()
        .and_then(|segment| segment.to_str())
        .unwrap_or("")
        .to_lowercase();
    if name == "backlog" || name == "project" || name == "workmesh" || name == ".workmesh" {
        return state_root.parent().unwrap_or(state_root).to_path_buf();
    }
    if name == "tasks" {
        return state_root.parent().unwrap_or(state_root).to_path_buf();
    }
    state_root.to_path_buf()
}

pub fn repo_root_from_backlog(backlog_dir: &Path) -> PathBuf {
    repo_root_from_state_root(backlog_dir)
}

pub fn repo_root_metadata_path(state_root: &Path) -> PathBuf {
    state_root.join(REPO_ROOT_MARKER)
}

pub fn write_repo_root_metadata(state_root: &Path, repo_root: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(state_root)?;
    fs::write(repo_root_metadata_path(state_root), repo_root.to_string_lossy().as_bytes())
}

pub fn read_repo_root_metadata(state_root: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(repo_root_metadata_path(state_root)).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

pub fn project_docs_dir(repo_root: &Path, project_id: &str) -> PathBuf {
    repo_root
        .join("docs")
        .join("projects")
        .join(project_id.trim())
}

pub fn ensure_project_docs(
    repo_root: &Path,
    project_id: &str,
    name: Option<&str>,
) -> Result<PathBuf, ProjectError> {
    let project_id = project_id.trim();
    if project_id.is_empty() {
        return Err(ProjectError::MissingId);
    }
    if project_id.contains('/') || project_id.contains('\\') {
        return Err(ProjectError::InvalidId(project_id.to_string()));
    }

    let project_dir = project_docs_dir(repo_root, project_id);
    let prds_dir = project_dir.join("prds");
    let decisions_dir = project_dir.join("decisions");
    let updates_dir = project_dir.join("updates");
    let initiatives_dir = project_dir.join("initiatives");

    fs::create_dir_all(&prds_dir)?;
    fs::create_dir_all(&decisions_dir)?;
    fs::create_dir_all(&updates_dir)?;
    fs::create_dir_all(&initiatives_dir)?;

    let project_name = name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(project_id);
    write_if_missing(
        &project_dir.join("README.md"),
        project_readme(project_id, project_name),
    )?;
    write_if_missing(&prds_dir.join("README.md"), section_readme("PRDs"))?;
    write_if_missing(
        &decisions_dir.join("README.md"),
        section_readme("Decisions"),
    )?;
    write_if_missing(&updates_dir.join("README.md"), section_readme("Updates"))?;

    Ok(project_dir)
}

fn write_if_missing(path: &Path, content: String) -> Result<(), std::io::Error> {
    if path.exists() {
        return Ok(());
    }
    fs::write(path, content)
}

fn project_readme(project_id: &str, name: &str) -> String {
    format!(
        "# Project: {name}\n\nID: {project_id}\n\n## Summary\n- \n\n## Goals\n- \n\n## Links\n- PRDs: ./prds/\n- Decisions: ./decisions/\n- Updates: ./updates/\n- Initiatives: ./initiatives/\n",
        name = name,
        project_id = project_id
    )
}

fn section_readme(section: &str) -> String {
    format!("# {section}\n\n- Add entries here.\n", section = section)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn repo_root_metadata_round_trip_is_used_for_external_state_roots() {
        let repo = TempDir::new().expect("repo");
        let state = TempDir::new().expect("state");

        write_repo_root_metadata(state.path(), repo.path()).expect("write metadata");
        assert_eq!(read_repo_root_metadata(state.path()).as_deref(), Some(repo.path()));
        assert_eq!(repo_root_from_state_root(state.path()), repo.path().to_path_buf());
    }
}
