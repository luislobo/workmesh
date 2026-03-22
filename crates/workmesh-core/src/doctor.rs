use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::backlog::{resolve_backlog, BacklogLayout};
use crate::config::{
    config_filename_candidates, find_config_root, global_config_path, load_global_config,
    resolve_workmesh_home_dir, resolve_worktrees_default_with_source,
};
use crate::context::{context_path, load_context};
use crate::focus::focus_path;
use crate::global_sessions::{
    rebuild_sessions_index, recover_sessions_events, sessions_current_path, sessions_events_path,
};
use crate::index::index_path;
use crate::skills::{detect_user_agents_in_home, embedded_skill_ids, SkillAgent};
use crate::storage::read_versioned_or_legacy_json;
use crate::truth::{
    rebuild_truth_projection, recover_truth_events, truth_events_path, truth_store_status,
    validate_truth_store,
};
use crate::worktrees::worktrees_registry_path;

fn layout_name(layout: BacklogLayout) -> &'static str {
    match layout {
        BacklogLayout::Split => "split",
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

#[derive(Debug, Default, Clone)]
struct StorageFixResult {
    attempted: bool,
    sessions_trimmed: usize,
    truth_trimmed: usize,
    sessions_index_rebuilt: bool,
    truth_projection_rebuilt: bool,
    errors: Vec<String>,
}

fn lock_path_accessibility(lock_dir: &Path) -> serde_json::Value {
    let mut accessible = true;
    let mut error: Option<String> = None;

    if let Err(err) = fs::create_dir_all(lock_dir) {
        accessible = false;
        error = Some(err.to_string());
    } else {
        let probe = lock_dir.join(".doctor.lockcheck");
        match fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&probe)
        {
            Ok(mut file) => {
                if let Err(err) = file.write_all(b"ok").and_then(|_| file.sync_all()) {
                    accessible = false;
                    error = Some(err.to_string());
                }
                let _ = fs::remove_file(probe);
            }
            Err(err) => {
                accessible = false;
                error = Some(err.to_string());
            }
        }
    }

    json!({
        "path": lock_dir.to_string_lossy().to_string(),
        "accessible": accessible,
        "error": error,
    })
}

fn jsonl_malformed_stats(path: &Path) -> serde_json::Value {
    if !path.exists() {
        return json!({
            "path": path.to_string_lossy().to_string(),
            "present": false,
            "malformed_lines": 0,
            "trailing_malformed_lines": 0,
            "non_trailing_malformed_lines": 0,
            "malformed_line_numbers": [],
            "error": null
        });
    }

    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => {
            return json!({
                "path": path.to_string_lossy().to_string(),
                "present": true,
                "malformed_lines": 0,
                "trailing_malformed_lines": 0,
                "non_trailing_malformed_lines": 0,
                "malformed_line_numbers": [],
                "error": err.to_string()
            });
        }
    };

    let lines: Vec<&str> = raw.lines().collect();
    let mut malformed_line_numbers = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if serde_json::from_str::<serde_json::Value>(trimmed).is_err() {
            malformed_line_numbers.push(idx + 1);
        }
    }

    let mut trailing_malformed = 0usize;
    for line in lines.iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if serde_json::from_str::<serde_json::Value>(trimmed).is_err() {
            trailing_malformed += 1;
        } else {
            break;
        }
    }

    let malformed_total = malformed_line_numbers.len();
    json!({
        "path": path.to_string_lossy().to_string(),
        "present": true,
        "malformed_lines": malformed_total,
        "trailing_malformed_lines": trailing_malformed,
        "non_trailing_malformed_lines": malformed_total.saturating_sub(trailing_malformed),
        "malformed_line_numbers": malformed_line_numbers,
        "error": null
    })
}

fn version_snapshot_report(name: &str, path: &Path) -> serde_json::Value {
    if !path.exists() {
        return json!({
            "name": name,
            "path": path.to_string_lossy().to_string(),
            "present": false,
            "ok": true,
            "version": null,
            "legacy": false,
            "error": null
        });
    }

    match read_versioned_or_legacy_json::<serde_json::Value>(path) {
        Ok(Some(snapshot)) => {
            let legacy = snapshot.version == 0;
            json!({
                "name": name,
                "path": path.to_string_lossy().to_string(),
                "present": true,
                "ok": true,
                "version": snapshot.version,
                "legacy": legacy,
                "error": null
            })
        }
        Ok(None) => json!({
            "name": name,
            "path": path.to_string_lossy().to_string(),
            "present": false,
            "ok": true,
            "version": null,
            "legacy": false,
            "error": null
        }),
        Err(err) => json!({
            "name": name,
            "path": path.to_string_lossy().to_string(),
            "present": true,
            "ok": false,
            "version": null,
            "legacy": false,
            "error": err.to_string()
        }),
    }
}

fn apply_storage_fixes(backlog_dir: &Path, home: Option<&PathBuf>) -> StorageFixResult {
    let mut result = StorageFixResult {
        attempted: true,
        ..StorageFixResult::default()
    };

    if let Some(home) = home {
        match recover_sessions_events(home) {
            Ok(trimmed) => result.sessions_trimmed = trimmed,
            Err(err) => result
                .errors
                .push(format!("sessions recovery failed: {}", err)),
        }
        let events_path = sessions_events_path(home);
        if events_path.exists() || result.sessions_trimmed > 0 {
            match rebuild_sessions_index(home) {
                Ok(_) => result.sessions_index_rebuilt = true,
                Err(err) => result
                    .errors
                    .push(format!("sessions index rebuild failed: {}", err)),
            }
        }
    } else {
        result
            .errors
            .push("global workmesh home could not be resolved".to_string());
    }

    match recover_truth_events(backlog_dir) {
        Ok(trimmed) => result.truth_trimmed = trimmed,
        Err(err) => result
            .errors
            .push(format!("truth recovery failed: {}", err)),
    }
    let truth_events = truth_events_path(backlog_dir);
    if truth_events.exists() || result.truth_trimmed > 0 {
        match rebuild_truth_projection(backlog_dir) {
            Ok(_) => result.truth_projection_rebuilt = true,
            Err(err) => result
                .errors
                .push(format!("truth projection rebuild failed: {}", err)),
        }
    }

    result
}

fn storage_fix_to_json(fix: Option<&StorageFixResult>) -> serde_json::Value {
    if let Some(fix) = fix {
        json!({
            "attempted": fix.attempted,
            "sessions_trimmed": fix.sessions_trimmed,
            "truth_trimmed": fix.truth_trimmed,
            "sessions_index_rebuilt": fix.sessions_index_rebuilt,
            "truth_projection_rebuilt": fix.truth_projection_rebuilt,
            "errors": fix.errors,
            "ok": fix.errors.is_empty(),
        })
    } else {
        json!({
            "attempted": false,
            "ok": true
        })
    }
}

fn storage_integrity_report(
    backlog_dir: &Path,
    global_home: Option<&PathBuf>,
    fix: Option<&StorageFixResult>,
) -> serde_json::Value {
    let repo_lock = lock_path_accessibility(&backlog_dir.join(".locks"));
    let global_lock = global_home
        .map(|home| lock_path_accessibility(&home.join(".locks")))
        .unwrap_or_else(|| {
            json!({
                "path": null,
                "accessible": false,
                "error": "global workmesh home could not be resolved"
            })
        });

    let truth_events = jsonl_malformed_stats(&truth_events_path(backlog_dir));
    let sessions_events = global_home
        .map(|home| jsonl_malformed_stats(&sessions_events_path(home)))
        .unwrap_or_else(|| {
            json!({
                "path": null,
                "present": false,
                "malformed_lines": 0,
                "trailing_malformed_lines": 0,
                "non_trailing_malformed_lines": 0,
                "malformed_line_numbers": [],
                "error": "global workmesh home could not be resolved"
            })
        });

    let truth_validation = validate_truth_store(backlog_dir).ok();
    let projection_mismatches = truth_validation
        .as_ref()
        .map(|report| report.projection_mismatches.clone())
        .unwrap_or_default();
    let transition_errors = truth_validation
        .as_ref()
        .map(|report| report.transition_errors.clone())
        .unwrap_or_default();

    let mut snapshots = vec![version_snapshot_report(
        "context",
        &context_path(backlog_dir),
    )];
    if let Some(home) = global_home {
        snapshots.push(version_snapshot_report(
            "sessions_current",
            &sessions_current_path(home),
        ));
        snapshots.push(version_snapshot_report(
            "worktree_registry",
            &worktrees_registry_path(home),
        ));
    }

    let legacy_snapshots = snapshots
        .iter()
        .filter(|entry| entry["legacy"].as_bool().unwrap_or(false))
        .count();
    let malformed_lines = truth_events["malformed_lines"].as_u64().unwrap_or(0)
        + sessions_events["malformed_lines"].as_u64().unwrap_or(0);
    let non_trailing_malformed = truth_events["non_trailing_malformed_lines"]
        .as_u64()
        .unwrap_or(0)
        + sessions_events["non_trailing_malformed_lines"]
            .as_u64()
            .unwrap_or(0);
    let locks_ok = repo_lock["accessible"].as_bool().unwrap_or(false)
        && global_lock["accessible"].as_bool().unwrap_or(false);
    let snapshots_ok = snapshots
        .iter()
        .all(|entry| entry["ok"].as_bool().unwrap_or(false));
    let truth_ok = truth_validation
        .as_ref()
        .map(|report| report.ok)
        .unwrap_or(true);
    let overall_ok = locks_ok
        && snapshots_ok
        && malformed_lines == 0
        && non_trailing_malformed == 0
        && projection_mismatches.is_empty()
        && transition_errors.is_empty()
        && truth_ok;

    json!({
        "ok": overall_ok,
        "locks": {
            "repo": repo_lock,
            "global": global_lock,
        },
        "jsonl": {
            "truth_events": truth_events,
            "sessions_events": sessions_events,
            "malformed_total": malformed_lines,
            "non_trailing_malformed_total": non_trailing_malformed,
        },
        "truth_projection": {
            "ok": truth_ok,
            "projection_mismatches": projection_mismatches,
            "transition_errors": transition_errors,
        },
        "versioned_snapshots": {
            "entries": snapshots,
            "legacy_count": legacy_snapshots,
        },
        "fix": storage_fix_to_json(fix),
    })
}

/// Return a machine-readable diagnostics report for a WorkMesh repo.
///
/// This is meant to be human-friendly when pretty-printed, but also stable enough for agents.
pub fn doctor_report(root: &Path, running_binary: &str) -> serde_json::Value {
    doctor_report_with_options(root, running_binary, false)
}

pub fn doctor_report_with_options(
    root: &Path,
    running_binary: &str,
    fix_storage: bool,
) -> serde_json::Value {
    let root = root.to_path_buf();
    let resolution = resolve_backlog(&root).ok();

    let (repo_root, backlog_dir, layout) = if let Some(res) = resolution.as_ref() {
        (
            res.repo_root.clone(),
            res.state_root.clone(),
            layout_name(res.layout).to_string(),
        )
    } else {
        (root.clone(), root.clone(), "unresolved".to_string())
    };

    let global_home = resolve_workmesh_home_dir();
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
    let global_config = {
        let path = global_config_path();
        let loaded = load_global_config();
        json!({
            "home": global_home.as_ref().map(|p| p.to_string_lossy().to_string()),
            "path": path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "present": path.as_ref().map(|p| p.exists()).unwrap_or(false),
            "loaded": loaded,
        })
    };
    let (worktrees_default, worktrees_default_source) =
        resolve_worktrees_default_with_source(&repo_root);

    let context_file = context_path(&backlog_dir);
    let context = load_context(&backlog_dir).ok().flatten().map(|c| {
        json!({
            "path": context_file.to_string_lossy().to_string(),
            "project_id": c.project_id,
            "objective": c.objective,
            "scope": c.scope,
            "updated_at": c.updated_at,
        })
    });
    let legacy_focus = {
        let path = focus_path(&backlog_dir);
        json!({
            "path": path.to_string_lossy().to_string(),
            "present": path.exists(),
        })
    };

    let idx_path = index_path(&backlog_dir);
    let index = json!({
        "path": idx_path.to_string_lossy().to_string(),
        "present": idx_path.exists(),
        "entries": if idx_path.exists() { count_lines(&idx_path) } else { None },
    });
    let truth = truth_store_status(&backlog_dir).ok().map(|status| {
        json!({
            "events_path": status.events_path,
            "current_path": status.current_path,
            "has_events": status.has_events,
            "has_current": status.has_current,
            "event_count": status.event_count,
            "record_count": status.record_count,
            "validation_ok": status.validation_ok,
        })
    });
    let storage_fix = if fix_storage {
        if resolution.is_some() {
            Some(apply_storage_fixes(&backlog_dir, global_home.as_ref()))
        } else {
            Some(StorageFixResult {
                attempted: true,
                errors: vec!["backlog unresolved; cannot apply storage fixes".to_string()],
                ..StorageFixResult::default()
            })
        }
    } else {
        None
    };
    let storage =
        storage_integrity_report(&backlog_dir, global_home.as_ref(), storage_fix.as_ref());

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
            "global": global_config,
            "effective": {
                "worktrees_default": worktrees_default,
                "worktrees_default_source": worktrees_default_source,
                "precedence": "project > global > default(true)"
            }
        },
        "context": context,
        "legacy_focus": legacy_focus,
        "index": index,
        "truth": truth,
        "storage": storage,
        "versions": versions,
        "skills": skills,
        "notes": [
            "Index files under workmesh/.index are derived and rebuildable.",
            "Context is primary orchestration state (workmesh/context.json).",
            "Legacy focus.json is deprecated and should be migrated.",
            "Truth records are append-only events under workmesh/truth/ with a current projection."
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::{doctor_report, doctor_report_with_options};
    use std::ffi::OsString;
    use tempfile::TempDir;

    fn with_env_lock<T>(f: impl FnOnce() -> T) -> T {
        let _guard = crate::test_env::lock();
        f()
    }

    struct EnvGuard {
        home: Option<OsString>,
        userprofile: Option<OsString>,
        workmesh_home: Option<OsString>,
    }

    impl EnvGuard {
        fn capture() -> Self {
            Self {
                home: std::env::var_os("HOME"),
                userprofile: std::env::var_os("USERPROFILE"),
                workmesh_home: std::env::var_os("WORKMESH_HOME"),
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
            if let Some(home) = self.workmesh_home.as_ref() {
                std::env::set_var("WORKMESH_HOME", home);
            } else {
                std::env::remove_var("WORKMESH_HOME");
            }
        }
    }

    #[test]
    fn doctor_report_includes_backlog_context_and_index() {
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

            // Context file.
            std::fs::write(
                repo.join("workmesh").join("context.json"),
                r#"{"version":1,"project_id":"demo","objective":"Ship","scope":{"mode":"epic","epic_id":"task-test-001","task_ids":[]},"updated_at":"2026-02-09T00:00:00Z"}"#,
            )
            .expect("write context");

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
            assert_eq!(report["context"]["project_id"].as_str(), Some("demo"));
            assert_eq!(report["legacy_focus"]["present"], false);
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
            assert_eq!(report["context"].is_null(), true);
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

    #[test]
    fn doctor_fix_storage_trims_trailing_jsonl_and_reports_fixes() {
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

            let workmesh_home = temp.path().join("global-home");
            std::env::set_var("WORKMESH_HOME", &workmesh_home);
            let sessions_dir = workmesh_home.join("sessions");
            std::fs::create_dir_all(&sessions_dir).expect("mkdir sessions");
            std::fs::write(
                sessions_dir.join("events.jsonl"),
                "{\"type\":\"session_saved\",\"session\":{\"id\":\"s1\",\"created_at\":\"2026-02-01T00:00:00Z\",\"updated_at\":\"2026-02-01T00:00:00Z\",\"cwd\":\"/tmp\",\"repo_root\":null,\"project_id\":null,\"epic_id\":null,\"objective\":\"ship\",\"working_set\":[],\"notes\":null,\"git\":null,\"checkpoint\":null,\"recent_changes\":null,\"handoff\":null,\"worktree\":null,\"truth_refs\":[]}}\n{\n",
            )
            .expect("write sessions events");

            let truth_dir = repo.join("workmesh").join("truth");
            std::fs::create_dir_all(&truth_dir).expect("mkdir truth");
            std::fs::write(truth_dir.join("events.jsonl"), "{\n").expect("write truth events");

            let report = doctor_report_with_options(repo, "workmesh", true);
            assert_eq!(report["storage"]["fix"]["attempted"], true);
            assert_eq!(report["storage"]["fix"]["ok"], true);
            assert_eq!(report["storage"]["fix"]["sessions_trimmed"], 1);
            assert_eq!(report["storage"]["fix"]["truth_trimmed"], 1);
            assert_eq!(report["storage"]["fix"]["sessions_index_rebuilt"], true);
            assert_eq!(report["storage"]["fix"]["truth_projection_rebuilt"], true);
            assert_eq!(
                report["storage"]["jsonl"]["sessions_events"]["trailing_malformed_lines"],
                0
            );
            assert_eq!(
                report["storage"]["jsonl"]["truth_events"]["trailing_malformed_lines"],
                0
            );
        })
    }
}
