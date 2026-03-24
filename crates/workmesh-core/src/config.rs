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
    /// Deprecated single-root setting. Prefer `tasks_root` + `state_root`.
    pub root_dir: Option<String>,
    /// Repo-relative or absolute path for task markdown files.
    pub tasks_root: Option<String>,
    /// Repo-relative or absolute path for repo-local WorkMesh state.
    pub state_root: Option<String>,
    /// Whether actionable and done tasks must include a substantive Description section.
    pub task_require_description: Option<bool>,
    /// Whether actionable and done tasks must include substantive Acceptance Criteria.
    pub task_require_acceptance_criteria: Option<bool>,
    /// Whether actionable and done tasks must include a substantive Definition of Done section.
    pub task_require_definition_of_done: Option<bool>,
    /// Whether Definition of Done must include outcome-based criteria instead of hygiene-only items.
    pub task_require_outcome_based_definition_of_done: Option<bool>,
    pub do_not_migrate: Option<bool>,
    /// Default behavior for promoting worktree-based parallel workflows.
    /// true = promote worktrees by default, false = suppress default worktree guidance.
    pub worktrees_default: Option<bool>,
    /// Default directory (absolute or repo-relative) used when WorkMesh needs to pick a worktree
    /// path automatically (for example, `workstream create` from the canonical checkout).
    ///
    /// If unset, WorkMesh falls back to a deterministic default:
    /// `<repo_parent>/<repo_name>.worktrees/`.
    pub worktrees_dir: Option<String>,
    /// Default behavior for auto-updating global sessions after mutating commands.
    /// true = enable by default, false = disable by default.
    pub auto_session_default: Option<bool>,
    /// Known initiative slugs used to namespace task ids (e.g. "login", "billing")
    pub initiatives: Option<Vec<String>>,
    /// Map of git branch name -> initiative slug frozen for that branch
    pub branch_initiatives: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskValidationRules {
    pub require_description: bool,
    pub require_acceptance_criteria: bool,
    pub require_definition_of_done: bool,
    pub require_outcome_based_definition_of_done: bool,
}

impl Default for TaskValidationRules {
    fn default() -> Self {
        Self {
            require_description: true,
            require_acceptance_criteria: true,
            require_definition_of_done: true,
            require_outcome_based_definition_of_done: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct TaskValidationRuleSources {
    pub require_description: &'static str,
    pub require_acceptance_criteria: &'static str,
    pub require_definition_of_done: &'static str,
    pub require_outcome_based_definition_of_done: &'static str,
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

pub fn load_config_with_path(repo_root: &Path) -> Option<(WorkmeshConfig, PathBuf)> {
    for name in config_filename_candidates() {
        let path = repo_root.join(name);
        if path.is_file() {
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<WorkmeshConfig>(&text) {
                    return Some((config, path));
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

pub fn load_global_config_with_path() -> Option<(WorkmeshConfig, PathBuf)> {
    let path = global_config_path()?;
    if !path.is_file() {
        return None;
    }
    let text = fs::read_to_string(&path).ok()?;
    let config = toml::from_str::<WorkmeshConfig>(&text).ok()?;
    Some((config, path))
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

pub fn resolve_worktrees_dir_with_source(repo_root: &Path) -> (Option<PathBuf>, &'static str) {
    if let Some(value) = load_config(repo_root).and_then(|config| config.worktrees_dir) {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return (Some(PathBuf::from(trimmed)), "project");
        }
    }
    if let Some(value) = load_global_config().and_then(|config| config.worktrees_dir) {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return (Some(PathBuf::from(trimmed)), "global");
        }
    }
    (None, "default")
}

pub fn resolve_worktrees_dir(repo_root: &Path) -> Option<PathBuf> {
    resolve_worktrees_dir_with_source(repo_root).0
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

fn resolve_bool_with_source(
    project_value: Option<bool>,
    global_value: Option<bool>,
    default: bool,
) -> (bool, &'static str) {
    if let Some(value) = project_value {
        return (value, "project");
    }
    if let Some(value) = global_value {
        return (value, "global");
    }
    (default, "default")
}

pub fn resolve_task_validation_rules_with_source(
    repo_root: &Path,
) -> (TaskValidationRules, TaskValidationRuleSources) {
    let project = load_config(repo_root);
    let global = load_global_config();

    let (require_description, require_description_source) = resolve_bool_with_source(
        project.as_ref().and_then(|cfg| cfg.task_require_description),
        global.as_ref().and_then(|cfg| cfg.task_require_description),
        true,
    );
    let (require_acceptance_criteria, require_acceptance_criteria_source) =
        resolve_bool_with_source(
            project
                .as_ref()
                .and_then(|cfg| cfg.task_require_acceptance_criteria),
            global
                .as_ref()
                .and_then(|cfg| cfg.task_require_acceptance_criteria),
            true,
        );
    let (require_definition_of_done, require_definition_of_done_source) =
        resolve_bool_with_source(
            project
                .as_ref()
                .and_then(|cfg| cfg.task_require_definition_of_done),
            global
                .as_ref()
                .and_then(|cfg| cfg.task_require_definition_of_done),
            true,
        );
    let (
        require_outcome_based_definition_of_done,
        require_outcome_based_definition_of_done_source,
    ) = resolve_bool_with_source(
        project
            .as_ref()
            .and_then(|cfg| cfg.task_require_outcome_based_definition_of_done),
        global
            .as_ref()
            .and_then(|cfg| cfg.task_require_outcome_based_definition_of_done),
        true,
    );

    (
        TaskValidationRules {
            require_description,
            require_acceptance_criteria,
            require_definition_of_done,
            require_outcome_based_definition_of_done,
        },
        TaskValidationRuleSources {
            require_description: require_description_source,
            require_acceptance_criteria: require_acceptance_criteria_source,
            require_definition_of_done: require_definition_of_done_source,
            require_outcome_based_definition_of_done:
                require_outcome_based_definition_of_done_source,
        },
    )
}

pub fn resolve_task_validation_rules(repo_root: &Path) -> TaskValidationRules {
    resolve_task_validation_rules_with_source(repo_root).0
}

pub fn write_config(repo_root: &Path, config: &WorkmeshConfig) -> Result<PathBuf, ConfigError> {
    let path = config_path(repo_root);
    let body = toml::to_string_pretty(config)?;
    fs::write(&path, body)?;
    Ok(path)
}

pub fn write_global_config(config: &WorkmeshConfig) -> Result<PathBuf, ConfigError> {
    let Some(path) = global_config_path() else {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "home dir not set").into());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
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
        || config
            .tasks_root
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        || config
            .state_root
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        || config.task_require_description.is_some()
        || config.task_require_acceptance_criteria.is_some()
        || config.task_require_definition_of_done.is_some()
        || config
            .task_require_outcome_based_definition_of_done
            .is_some()
        || config.worktrees_default.is_some()
        || config
            .worktrees_dir
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
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
            tasks_root: None,
            state_root: None,
            task_require_description: Some(true),
            task_require_acceptance_criteria: Some(true),
            task_require_definition_of_done: Some(true),
            task_require_outcome_based_definition_of_done: Some(true),
            do_not_migrate: Some(true),
            worktrees_default: Some(true),
            worktrees_dir: None,
            auto_session_default: Some(true),
            initiatives: None,
            branch_initiatives: None,
        };
        write_config(temp.path(), &config).expect("write config");
        let loaded = load_config(temp.path()).expect("load config");
        assert_eq!(loaded.root_dir.as_deref(), Some("workmesh"));
        assert_eq!(loaded.tasks_root, None);
        assert_eq!(loaded.state_root, None);
        assert_eq!(loaded.task_require_description, Some(true));
        assert_eq!(loaded.task_require_acceptance_criteria, Some(true));
        assert_eq!(loaded.task_require_definition_of_done, Some(true));
        assert_eq!(
            loaded.task_require_outcome_based_definition_of_done,
            Some(true)
        );
        assert_eq!(loaded.do_not_migrate, Some(true));
        assert_eq!(loaded.worktrees_default, Some(true));
        assert_eq!(loaded.auto_session_default, Some(true));
    }

    #[test]
    fn update_do_not_migrate_removes_file_when_cleared() {
        let temp = TempDir::new().expect("tempdir");
        let config = WorkmeshConfig {
            root_dir: None,
            tasks_root: None,
            state_root: None,
            task_require_description: None,
            task_require_acceptance_criteria: None,
            task_require_definition_of_done: None,
            task_require_outcome_based_definition_of_done: None,
            do_not_migrate: Some(true),
            worktrees_default: None,
            worktrees_dir: None,
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
            tasks_root: None,
            state_root: None,
            task_require_description: None,
            task_require_acceptance_criteria: None,
            task_require_definition_of_done: None,
            task_require_outcome_based_definition_of_done: None,
            do_not_migrate: Some(true),
            worktrees_default: Some(false),
            worktrees_dir: None,
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

    #[test]
    fn resolve_task_validation_rules_prefers_project_over_global_then_default() {
        with_env_lock(|| {
            let _env = EnvGuard::capture();
            let repo = TempDir::new().expect("repo tempdir");
            let home = TempDir::new().expect("home tempdir");
            std::env::set_var("WORKMESH_HOME", home.path());

            let (rules, sources) = resolve_task_validation_rules_with_source(repo.path());
            assert_eq!(rules, TaskValidationRules::default());
            assert_eq!(sources.require_description, "default");
            assert_eq!(sources.require_acceptance_criteria, "default");
            assert_eq!(sources.require_definition_of_done, "default");
            assert_eq!(sources.require_outcome_based_definition_of_done, "default");

            std::fs::create_dir_all(home.path()).expect("home dir");
            std::fs::write(
                home.path().join("config.toml"),
                "task_require_description = false\n\
task_require_acceptance_criteria = false\n\
task_require_definition_of_done = true\n\
task_require_outcome_based_definition_of_done = true\n",
            )
            .expect("global config");
            let (rules, sources) = resolve_task_validation_rules_with_source(repo.path());
            assert!(!rules.require_description);
            assert!(!rules.require_acceptance_criteria);
            assert!(rules.require_definition_of_done);
            assert!(rules.require_outcome_based_definition_of_done);
            assert_eq!(sources.require_description, "global");
            assert_eq!(sources.require_acceptance_criteria, "global");

            std::fs::write(
                repo.path().join(".workmesh.toml"),
                "task_require_acceptance_criteria = true\n\
task_require_definition_of_done = false\n",
            )
            .expect("project config");
            let (rules, sources) = resolve_task_validation_rules_with_source(repo.path());
            assert!(!rules.require_description);
            assert!(rules.require_acceptance_criteria);
            assert!(!rules.require_definition_of_done);
            assert!(rules.require_outcome_based_definition_of_done);
            assert_eq!(sources.require_description, "global");
            assert_eq!(sources.require_acceptance_criteria, "project");
            assert_eq!(sources.require_definition_of_done, "project");
            assert_eq!(
                sources.require_outcome_based_definition_of_done,
                "global"
            );
        });
    }
}
