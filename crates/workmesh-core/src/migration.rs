use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::backlog::{BacklogLayout, BacklogResolution};

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Backlog already at {0}")]
    AlreadyMigrated(PathBuf),
    #[error("Destination exists: {0}")]
    DestinationExists(PathBuf),
    #[error("Unsupported layout for migration")]
    UnsupportedLayout,
    #[error("Failed to migrate: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct MigrationResult {
    pub from: PathBuf,
    pub to: PathBuf,
}

pub fn migrate_backlog(
    resolution: &BacklogResolution,
    target_root: &str,
) -> Result<MigrationResult, MigrationError> {
    let repo_root = &resolution.repo_root;
    let target_dir = repo_root.join(target_root);
    if target_dir.exists() {
        return Err(MigrationError::DestinationExists(target_dir));
    }
    if resolution.layout == BacklogLayout::Workmesh && target_root == "workmesh" {
        return Err(MigrationError::AlreadyMigrated(
            resolution.backlog_dir.clone(),
        ));
    }
    if resolution.layout == BacklogLayout::HiddenWorkmesh && target_root == ".workmesh" {
        return Err(MigrationError::AlreadyMigrated(
            resolution.backlog_dir.clone(),
        ));
    }

    match resolution.layout {
        BacklogLayout::Backlog | BacklogLayout::Project => {
            fs::rename(&resolution.backlog_dir, &target_dir)?;
        }
        BacklogLayout::RootTasks | BacklogLayout::TasksDir => {
            fs::create_dir_all(&target_dir)?;
            let tasks_dir = resolution.backlog_dir.join("tasks");
            if tasks_dir.is_dir() {
                fs::rename(&tasks_dir, target_dir.join("tasks"))?;
            }
            move_if_exists(&resolution.backlog_dir, &target_dir, ".audit.log")?;
            move_if_exists(&resolution.backlog_dir, &target_dir, ".index")?;
        }
        BacklogLayout::Workmesh | BacklogLayout::HiddenWorkmesh | BacklogLayout::Custom => {
            return Err(MigrationError::UnsupportedLayout);
        }
    }

    Ok(MigrationResult {
        from: resolution.backlog_dir.clone(),
        to: target_dir,
    })
}

fn move_if_exists(src_root: &Path, dest_root: &Path, name: &str) -> Result<(), std::io::Error> {
    let src = src_root.join(name);
    if src.exists() {
        fs::rename(&src, dest_root.join(name))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backlog::{resolve_backlog, BacklogLayout};
    use tempfile::TempDir;

    #[test]
    fn migrate_backlog_dir_to_workmesh() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path().join("backlog").join("tasks");
        fs::create_dir_all(&backlog_dir).expect("backlog");

        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::Backlog);

        let result = migrate_backlog(&resolution, "workmesh").expect("migrate");
        assert!(result.to.join("tasks").is_dir());
        assert!(!result.from.exists());
    }
}
