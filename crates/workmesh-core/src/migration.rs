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
    if target_dir.exists() {
        return Err(MigrationError::DestinationExists(target_dir));
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

    #[test]
    fn migrate_errors_when_destination_exists() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path().join("backlog").join("tasks");
        fs::create_dir_all(&backlog_dir).expect("backlog");
        fs::create_dir_all(temp.path().join("workmesh")).expect("existing dest");

        let resolution = resolve_backlog(temp.path()).expect("resolve");
        let err = migrate_backlog(&resolution, "workmesh").expect_err("should fail");
        match err {
            MigrationError::DestinationExists(path) => {
                assert!(path.ends_with("workmesh"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn migrate_errors_when_already_migrated_workmesh() {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path().join("workmesh").join("tasks")).expect("workmesh");
        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::Workmesh);

        let err = migrate_backlog(&resolution, "workmesh").expect_err("already migrated");
        assert!(matches!(err, MigrationError::AlreadyMigrated(_)));
    }

    #[test]
    fn migrate_errors_when_already_migrated_hidden_workmesh() {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path().join(".workmesh").join("tasks")).expect("hidden");
        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::HiddenWorkmesh);

        let err = migrate_backlog(&resolution, ".workmesh").expect_err("already migrated");
        assert!(matches!(err, MigrationError::AlreadyMigrated(_)));
    }

    #[test]
    fn migrate_root_tasks_moves_tasks_and_artifacts() {
        let temp = TempDir::new().expect("tempdir");
        let tasks = temp.path().join("tasks");
        fs::create_dir_all(&tasks).expect("tasks");
        fs::write(tasks.join("task-001.md"), "---\nid: task-001\n---\n").expect("task");
        fs::write(temp.path().join(".audit.log"), "[]\n").expect("audit");
        fs::create_dir_all(temp.path().join(".index")).expect("index");

        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::RootTasks);

        let result = migrate_backlog(&resolution, "workmesh").expect("migrate");
        assert!(result.to.join("tasks").is_dir());
        assert!(result.to.join(".audit.log").is_file());
        assert!(result.to.join(".index").is_dir());
        assert!(!tasks.exists());
    }

    #[test]
    fn migrate_custom_layout_is_not_supported() {
        let temp = TempDir::new().expect("tempdir");
        // When the source is already a workmesh layout but the target differs, migration is not supported.
        fs::create_dir_all(temp.path().join("workmesh").join("tasks")).expect("workmesh");
        let resolution = resolve_backlog(temp.path()).expect("resolve");
        assert_eq!(resolution.layout, BacklogLayout::Workmesh);

        let err = migrate_backlog(&resolution, ".workmesh").expect_err("unsupported");
        assert!(matches!(err, MigrationError::UnsupportedLayout));

        let temp2 = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp2.path().join(".workmesh").join("tasks")).expect("hidden");
        let resolution2 = resolve_backlog(temp2.path()).expect("resolve");
        assert_eq!(resolution2.layout, BacklogLayout::HiddenWorkmesh);
        let err = migrate_backlog(&resolution2, "workmesh").expect_err("unsupported");
        assert!(matches!(err, MigrationError::UnsupportedLayout));
    }
}
