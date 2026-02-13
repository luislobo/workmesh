use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::backlog::{resolve_backlog, BacklogLayout};
use crate::config::{config_filename_candidates, find_config_root};
use crate::focus::{focus_path, load_focus};
use crate::index::index_path;
use crate::skills::{detect_user_agents_in_home, embedded_skill_ids, SkillAgent};

fn layout_name(layout: BacklogLayout) -> &'static str {
    match layout {
        BacklogLayout::Workmesh => "workmesh",
        BacklogLayout::HiddenWorkmesh => ".workmesh",
        BacklogLayout::Backlog => "backlog",
        BacklogLayout::Project => "project",
        BacklogLayout::RootTasks => "root/tasks",
        BacklogLayout::TasksDir => "tasks-dir",
        BacklogLayout::Custom => "custom",
    }
}

fn agent_name(agent: SkillAgent) -> &'static str {
    match agent {
        SkillAgent::Codex => "codex",
        SkillAgent::Claude => "claude",
        SkillAgent::Cursor => "cursor",
        SkillAgent::All => "all",
    }
}

fn home_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .ok()
        .map(|value| value.trim().to_string());
    if let Some(home) = home {
        if !home.is_empty() {
            return Some(PathBuf::from(home));
        }
    }
    let profile = std::env::var("USERPROFILE")
        .ok()
        .map(|value| value.trim().to_string());
    if let Some(profile) = profile {
        if !profile.is_empty() {
            return Some(PathBuf::from(profile));
        }
    }
    None
}

fn user_skill_path(home: &Path, agent: SkillAgent, skill_name: &str) -> PathBuf {
    let root = match agent {
        SkillAgent::Codex => home.join(".codex").join("skills"),
        SkillAgent::Claude => home.join(".claude").join("skills"),
        SkillAgent::Cursor => home.join(".cursor").join("skills"),
        SkillAgent::All => home.join(".codex").join("skills"),
    };
    root.join(skill_name).join("SKILL.md")
}

fn best_effort_other_binary_version(binary_name: &str) -> Option<String> {
    let which = which::which(binary_name).ok()?;
    let output = std::process::Command::new(which)
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() {
        None
    } else {
        Some(raw)
    }
}

fn count_lines(path: &Path) -> Option<usize> {
    let text = fs::read_to_string(path).ok()?;
    Some(text.lines().count())
}

/// Return a machine-readable diagnostics report for a WorkMesh repo.
///
/// This is meant to be human-friendly when pretty-printed, but also stable enough for agents.
pub fn doctor_report(root: &Path, running_binary: &str) -> serde_json::Value {
    let root = root.to_path_buf();
    let resolution = resolve_backlog(&root).ok();

    let (repo_root, backlog_dir, layout) = if let Some(res) = resolution.as_ref() {
        (
            res.repo_root.clone(),
            res.backlog_dir.clone(),
            layout_name(res.layout).to_string(),
        )
    } else {
        (root.clone(), root.clone(), "unresolved".to_string())
    };

    let config_root = find_config_root(&root).or_else(|| find_config_root(&repo_root));
    let config_files = config_root.as_ref().map(|dir| {
        config_filename_candidates()
            .iter()
            .map(|name| {
                let path = dir.join(name);
                json!({
                    "name": name,
                    "path": path.to_string_lossy().to_string(),
                    "exists": path.exists(),
                })
            })
            .collect::<Vec<_>>()
    });

    let focus_path = focus_path(&backlog_dir);
    let focus = load_focus(&backlog_dir).ok().flatten().map(|f| {
        json!({
            "path": focus_path.to_string_lossy().to_string(),
            "project_id": f.project_id,
            "epic_id": f.epic_id,
            "objective": f.objective,
            "working_set_count": f.working_set.len(),
            "updated_at": f.updated_at,
        })
    });

    let idx_path = index_path(&backlog_dir);
    let index = json!({
        "path": idx_path.to_string_lossy().to_string(),
        "present": idx_path.exists(),
        "entries": if idx_path.exists() { count_lines(&idx_path) } else { None },
    });

    let versions = match running_binary {
        "workmesh" => json!({
            "workmesh": env!("CARGO_PKG_VERSION"),
            "workmesh_mcp": best_effort_other_binary_version("workmesh-mcp"),
        }),
        "workmesh-mcp" => json!({
            "workmesh_mcp": env!("CARGO_PKG_VERSION"),
            "workmesh": best_effort_other_binary_version("workmesh"),
        }),
        _ => json!({
            "running": env!("CARGO_PKG_VERSION"),
        }),
    };

    let skills = {
        let embedded = embedded_skill_ids();
        let home = home_dir();
        let agents = home
            .as_ref()
            .map(|h| detect_user_agents_in_home(h))
            .unwrap_or_default();

        let mut installed = Vec::new();
        if let Some(home) = home.as_ref() {
            for skill in embedded.iter() {
                for agent in agents.iter() {
                    let path = user_skill_path(home, *agent, skill);
                    installed.push(json!({
                        "agent": agent_name(*agent),
                        "skill": skill,
                        "path": path.to_string_lossy().to_string(),
                        "present": path.exists(),
                    }));
                }
            }
        }

        json!({
            "embedded": embedded,
            "detected_user_agents": agents.iter().map(|a| agent_name(*a)).collect::<Vec<_>>(),
            "user_installed": installed,
        })
    };

    json!({
        "root": root.to_string_lossy().to_string(),
        "repo_root": repo_root.to_string_lossy().to_string(),
        "backlog_dir": backlog_dir.to_string_lossy().to_string(),
        "layout": layout,
        "tasks_dir": backlog_dir.join("tasks").to_string_lossy().to_string(),
        "archive_dir": backlog_dir.join("archive").to_string_lossy().to_string(),
        "config": {
            "root": config_root.as_ref().map(|p| p.to_string_lossy().to_string()),
            "files": config_files,
        },
        "focus": focus,
        "index": index,
        "versions": versions,
        "skills": skills,
        "notes": [
            "Index files under workmesh/.index are derived and rebuildable.",
            "If focus exists, next/next_task/next_tasks prefer active work inside focus.working_set."
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::doctor_report;
    use std::ffi::OsString;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_env_lock<T>(f: impl FnOnce() -> T) -> T {
        let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        f()
    }

    struct EnvGuard {
        home: Option<OsString>,
        userprofile: Option<OsString>,
    }

    impl EnvGuard {
        fn capture() -> Self {
            Self {
                home: std::env::var_os("HOME"),
                userprofile: std::env::var_os("USERPROFILE"),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(home) = self.home.as_ref() {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
            if let Some(profile) = self.userprofile.as_ref() {
                std::env::set_var("USERPROFILE", profile);
            } else {
                std::env::remove_var("USERPROFILE");
            }
        }
    }

    #[test]
    fn doctor_report_includes_backlog_focus_and_index() {
        with_env_lock(|| {
            let temp = TempDir::new().expect("tempdir");
            let repo = temp.path();
            let _env_guard = EnvGuard::capture();

            // Minimal backlog: workmesh/tasks with one task.
            let tasks_dir = repo.join("workmesh").join("tasks");
            std::fs::create_dir_all(&tasks_dir).expect("mkdir tasks");
            std::fs::write(
                tasks_dir.join("task-test-001 - seed task.md"),
                "---\nid: task-test-001\ntitle: Seed\nstatus: To Do\npriority: P2\nphase: Phase1\n---\n\n## Notes\n\n",
            )
            .expect("write task");

            // Focus file.
            std::fs::write(
                repo.join("workmesh").join("focus.json"),
                r#"{"project_id":"demo","epic_id":"task-test-001","objective":"Ship","working_set":["task-test-001"],"updated_at":"2026-02-09T00:00:00Z"}"#,
            )
            .expect("write focus");

            // Index file (derived).
            let index_dir = repo.join("workmesh").join(".index");
            std::fs::create_dir_all(&index_dir).expect("mkdir index");
            std::fs::write(
                index_dir.join("tasks.jsonl"),
                "{\"id\":\"task-test-001\"}\n",
            )
            .expect("write index");

            // Skills detection: point HOME at temp and create ~/.codex to be detected.
            std::env::set_var("HOME", repo);
            std::fs::create_dir_all(repo.join(".codex")).expect("mkdir .codex");
            std::fs::create_dir_all(repo.join(".codex").join("skills").join("workmesh"))
                .expect("mkdir skill");
            std::fs::write(
                repo.join(".codex")
                    .join("skills")
                    .join("workmesh")
                    .join("SKILL.md"),
                "test",
            )
            .expect("write skill");

            let report = doctor_report(repo, "workmesh");
            assert_eq!(report["layout"], "workmesh");
            assert_eq!(report["focus"]["project_id"].as_str(), Some("demo"));
            assert_eq!(report["index"]["present"], true);
            assert_eq!(report["index"]["entries"], 1);
            assert!(report["skills"]["embedded"].is_array());
        })
    }

    #[test]
    fn doctor_report_unresolved_root_uses_fallback_shape() {
        with_env_lock(|| {
            let temp = TempDir::new().expect("tempdir");
            let repo = temp.path();
            let _env_guard = EnvGuard::capture();

            std::env::remove_var("HOME");
            std::env::remove_var("USERPROFILE");

            let report = doctor_report(repo, "unknown-binary");
            assert_eq!(report["layout"], "unresolved");
            assert_eq!(report["focus"].is_null(), true);
            assert_eq!(report["index"]["present"], false);
            assert_eq!(report["versions"]["running"].as_str().is_some(), true);

            let agents = report["skills"]["detected_user_agents"]
                .as_array()
                .expect("agents");
            assert!(agents.is_empty());
        })
    }

    #[test]
    fn doctor_report_supports_mcp_binary_and_userprofile_detection() {
        with_env_lock(|| {
            let temp = TempDir::new().expect("tempdir");
            let repo = temp.path();
            let _env_guard = EnvGuard::capture();

            let tasks_dir = repo.join("workmesh").join("tasks");
            std::fs::create_dir_all(&tasks_dir).expect("mkdir tasks");
            std::fs::write(
                tasks_dir.join("task-test-001 - seed task.md"),
                "---\nid: task-test-001\ntitle: Seed\nstatus: To Do\npriority: P2\nphase: Phase1\n---\n",
            )
            .expect("write task");

            std::fs::write(repo.join(".workmesh.toml"), "root_dir = \"workmesh\"\n")
                .expect("write config");

            std::env::remove_var("HOME");
            std::env::set_var("USERPROFILE", repo);
            std::fs::create_dir_all(repo.join(".cursor")).expect("mkdir .cursor");

            let report = doctor_report(repo, "workmesh-mcp");
            assert_eq!(report["layout"], "workmesh");
            assert_eq!(report["versions"]["workmesh_mcp"].as_str().is_some(), true);

            let agents = report["skills"]["detected_user_agents"]
                .as_array()
                .expect("agents")
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>();
            assert!(agents.contains(&"cursor"));

            let config_files = report["config"]["files"].as_array().expect("config files");
            assert!(config_files.iter().any(|entry| {
                entry["name"] == ".workmesh.toml" && entry["exists"] == serde_json::json!(true)
            }));
        })
    }
}
