use std::fs;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Config IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkmeshConfig {
    pub root_dir: Option<String>,
    pub do_not_migrate: Option<bool>,
    /// Known initiative slugs used to namespace task ids (e.g. "login", "billing")
    pub initiatives: Option<Vec<String>>,
    /// Map of git branch name -> initiative slug frozen for that branch
    pub branch_initiatives: Option<HashMap<String, String>>,
}

pub fn config_filename_candidates() -> [&'static str; 2] {
    [".workmesh.toml", ".workmeshrc"]
}

pub fn config_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".workmesh.toml")
}

pub fn find_config_root(start: &Path) -> Option<PathBuf> {
    let start = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    for candidate in start.ancestors() {
        for name in config_filename_candidates() {
            if candidate.join(name).is_file() {
                return Some(candidate.to_path_buf());
            }
        }
    }
    None
}

pub fn load_config(repo_root: &Path) -> Option<WorkmeshConfig> {
    for name in config_filename_candidates() {
        let path = repo_root.join(name);
        if path.is_file() {
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<WorkmeshConfig>(&text) {
                    return Some(config);
                }
            }
        }
    }
    None
}

pub fn write_config(repo_root: &Path, config: &WorkmeshConfig) -> Result<PathBuf, ConfigError> {
    let path = config_path(repo_root);
    let body = toml::to_string_pretty(config)?;
    fs::write(&path, body)?;
    Ok(path)
}

pub fn update_do_not_migrate(
    repo_root: &Path,
    value: bool,
) -> Result<Option<PathBuf>, ConfigError> {
    let mut config = load_config(repo_root).unwrap_or_default();
    config.do_not_migrate = Some(value);
    let has_other_fields = config
        .root_dir
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    if !value && !has_other_fields {
        let path = config_path(repo_root);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        return Ok(None);
    }
    write_config(repo_root, &config).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_and_read_config() {
        let temp = TempDir::new().expect("tempdir");
        let config = WorkmeshConfig {
            root_dir: Some("workmesh".to_string()),
            do_not_migrate: Some(true),
            initiatives: None,
            branch_initiatives: None,
        };
        write_config(temp.path(), &config).expect("write config");
        let loaded = load_config(temp.path()).expect("load config");
        assert_eq!(loaded.root_dir.as_deref(), Some("workmesh"));
        assert_eq!(loaded.do_not_migrate, Some(true));
    }

    #[test]
    fn update_do_not_migrate_removes_file_when_cleared() {
        let temp = TempDir::new().expect("tempdir");
        let config = WorkmeshConfig {
            root_dir: None,
            do_not_migrate: Some(true),
            initiatives: None,
            branch_initiatives: None,
        };
        let path = write_config(temp.path(), &config).expect("write config");
        assert!(path.exists());
        update_do_not_migrate(temp.path(), false).expect("clear");
        assert!(!path.exists());
    }
}
