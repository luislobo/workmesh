use std::collections::HashMap;
use std::fs;
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
    /// Default behavior for promoting worktree-based parallel workflows.
    /// true = promote worktrees by default, false = suppress default worktree guidance.
    pub worktrees_default: Option<bool>,
    /// Default behavior for auto-updating global sessions after mutating commands.
    /// true = enable by default, false = disable by default.
    pub auto_session_default: Option<bool>,
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

pub fn resolve_user_home_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let trimmed = profile.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    None
}

pub fn resolve_workmesh_home_dir() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("WORKMESH_HOME") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    resolve_user_home_dir().map(|home| home.join(".workmesh"))
}

pub fn global_config_path() -> Option<PathBuf> {
    resolve_workmesh_home_dir().map(|home| home.join("config.toml"))
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

pub fn load_global_config() -> Option<WorkmeshConfig> {
    let path = global_config_path()?;
    if !path.is_file() {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    toml::from_str::<WorkmeshConfig>(&text).ok()
}

pub fn resolve_worktrees_default_with_source(repo_root: &Path) -> (bool, &'static str) {
    if let Some(value) = load_config(repo_root).and_then(|config| config.worktrees_default) {
        return (value, "project");
    }
    if let Some(value) = load_global_config().and_then(|config| config.worktrees_default) {
        return (value, "global");
    }
    (true, "default")
}

pub fn resolve_worktrees_default(repo_root: &Path) -> bool {
    resolve_worktrees_default_with_source(repo_root).0
}

pub fn resolve_auto_session_default_with_source(repo_root: &Path) -> (Option<bool>, &'static str) {
    if let Some(value) = load_config(repo_root).and_then(|config| config.auto_session_default) {
        return (Some(value), "project");
    }
    if let Some(value) = load_global_config().and_then(|config| config.auto_session_default) {
        return (Some(value), "global");
    }
    (None, "default")
}

pub fn resolve_auto_session_default(repo_root: &Path) -> Option<bool> {
    resolve_auto_session_default_with_source(repo_root).0
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
        .unwrap_or(false)
        || config.worktrees_default.is_some()
        || config.auto_session_default.is_some()
        || config
            .initiatives
            .as_ref()
            .map(|values| !values.is_empty())
            .unwrap_or(false)
        || config
            .branch_initiatives
            .as_ref()
            .map(|values| !values.is_empty())
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
    use std::ffi::OsString;
    use tempfile::TempDir;

    fn with_env_lock<T>(f: impl FnOnce() -> T) -> T {
        let _guard = crate::test_env::lock();
        f()
    }

    struct EnvGuard {
        workmesh_home: Option<OsString>,
        home: Option<OsString>,
        userprofile: Option<OsString>,
    }

    impl EnvGuard {
        fn capture() -> Self {
            Self {
                workmesh_home: std::env::var_os("WORKMESH_HOME"),
                home: std::env::var_os("HOME"),
                userprofile: std::env::var_os("USERPROFILE"),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = self.workmesh_home.as_ref() {
                std::env::set_var("WORKMESH_HOME", value);
            } else {
                std::env::remove_var("WORKMESH_HOME");
            }

            if let Some(value) = self.home.as_ref() {
                std::env::set_var("HOME", value);
            } else {
                std::env::remove_var("HOME");
            }

            if let Some(value) = self.userprofile.as_ref() {
                std::env::set_var("USERPROFILE", value);
            } else {
                std::env::remove_var("USERPROFILE");
            }
        }
    }

    #[test]
    fn write_and_read_config() {
        let temp = TempDir::new().expect("tempdir");
        let config = WorkmeshConfig {
            root_dir: Some("workmesh".to_string()),
            do_not_migrate: Some(true),
            worktrees_default: Some(true),
            auto_session_default: Some(true),
            initiatives: None,
            branch_initiatives: None,
        };
        write_config(temp.path(), &config).expect("write config");
        let loaded = load_config(temp.path()).expect("load config");
        assert_eq!(loaded.root_dir.as_deref(), Some("workmesh"));
        assert_eq!(loaded.do_not_migrate, Some(true));
        assert_eq!(loaded.worktrees_default, Some(true));
        assert_eq!(loaded.auto_session_default, Some(true));
    }

    #[test]
    fn update_do_not_migrate_removes_file_when_cleared() {
        let temp = TempDir::new().expect("tempdir");
        let config = WorkmeshConfig {
            root_dir: None,
            do_not_migrate: Some(true),
            worktrees_default: None,
            auto_session_default: None,
            initiatives: None,
            branch_initiatives: None,
        };
        let path = write_config(temp.path(), &config).expect("write config");
        assert!(path.exists());
        update_do_not_migrate(temp.path(), false).expect("clear");
        assert!(!path.exists());
    }

    #[test]
    fn update_do_not_migrate_preserves_file_when_other_fields_exist() {
        let temp = TempDir::new().expect("tempdir");
        let config = WorkmeshConfig {
            root_dir: None,
            do_not_migrate: Some(true),
            worktrees_default: Some(false),
            auto_session_default: None,
            initiatives: None,
            branch_initiatives: None,
        };
        let path = write_config(temp.path(), &config).expect("write config");
        assert!(path.exists());
        let updated = update_do_not_migrate(temp.path(), false).expect("clear");
        assert!(updated.is_some());
        let loaded = load_config(temp.path()).expect("load config");
        assert_eq!(loaded.worktrees_default, Some(false));
        assert_eq!(loaded.do_not_migrate, Some(false));
    }

    #[test]
    fn resolve_worktrees_default_prefers_project_over_global_then_default() {
        with_env_lock(|| {
            let _env = EnvGuard::capture();
            let repo = TempDir::new().expect("repo tempdir");
            let home = TempDir::new().expect("home tempdir");
            std::env::set_var("WORKMESH_HOME", home.path());

            // No config at all -> built-in default true.
            let (value, source) = resolve_worktrees_default_with_source(repo.path());
            assert!(value);
            assert_eq!(source, "default");

            // Global config applies when project config is absent.
            std::fs::create_dir_all(home.path()).expect("home dir");
            std::fs::write(
                home.path().join("config.toml"),
                "worktrees_default = false\n",
            )
            .expect("global config");
            let (value, source) = resolve_worktrees_default_with_source(repo.path());
            assert!(!value);
            assert_eq!(source, "global");

            // Project config overrides global config.
            std::fs::write(
                repo.path().join(".workmesh.toml"),
                "worktrees_default = true\n",
            )
            .expect("project config");
            let (value, source) = resolve_worktrees_default_with_source(repo.path());
            assert!(value);
            assert_eq!(source, "project");
        });
    }

    #[test]
    fn resolve_auto_session_default_prefers_project_over_global_then_unset() {
        with_env_lock(|| {
            let _env = EnvGuard::capture();
            let repo = TempDir::new().expect("repo tempdir");
            let home = TempDir::new().expect("home tempdir");
            std::env::set_var("WORKMESH_HOME", home.path());

            // No config at all -> unset.
            let (value, source) = resolve_auto_session_default_with_source(repo.path());
            assert_eq!(value, None);
            assert_eq!(source, "default");

            // Global config applies when project config is absent.
            std::fs::create_dir_all(home.path()).expect("home dir");
            std::fs::write(
                home.path().join("config.toml"),
                "auto_session_default = false\n",
            )
            .expect("global config");
            let (value, source) = resolve_auto_session_default_with_source(repo.path());
            assert_eq!(value, Some(false));
            assert_eq!(source, "global");

            // Project config overrides global config.
            std::fs::write(
                repo.path().join(".workmesh.toml"),
                "auto_session_default = true\n",
            )
            .expect("project config");
            let (value, source) = resolve_auto_session_default_with_source(repo.path());
            assert_eq!(value, Some(true));
            assert_eq!(source, "project");
        });
    }
}
