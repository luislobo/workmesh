use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};

use crate::config::{load_config, write_config, WorkmeshConfig};
use crate::task::Task;

pub fn best_effort_git_branch(repo_root: &Path) -> Option<String> {
    if let Ok(override_branch) = std::env::var("WORKMESH_BRANCH") {
        let value = override_branch.trim().to_string();
        if !value.is_empty() {
            return Some(value);
        }
    }
    Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|value| !value.trim().is_empty())
}

pub fn branch_to_initiative_slug(branch: &str) -> String {
    let raw = branch.trim();
    if raw.is_empty() {
        return "work".to_string();
    }
    let mut s = raw.to_string();
    for prefix in [
        "feature/",
        "feat/",
        "bugfix/",
        "fix/",
        "chore/",
        "hotfix/",
        "issue/",
        "spike/",
    ] {
        if s.starts_with(prefix) {
            s = s[prefix.len()..].to_string();
            break;
        }
    }
    // Use the last path segment as the core initiative name.
    if let Some(last) = s.split('/').last() {
        s = last.to_string();
    }
    slugify(&s).unwrap_or_else(|| "work".to_string())
}

pub fn ensure_branch_initiative(repo_root: &Path, branch: &str) -> Result<String> {
    let mut config = load_config(repo_root).unwrap_or_default();
    if let Some(map) = config.branch_initiatives.as_ref() {
        if let Some(existing) = map.get(branch) {
            if !existing.trim().is_empty() {
                return Ok(existing.trim().to_string());
            }
        }
    }

    let desired = branch_to_initiative_slug(branch);
    let key = reserve_unique_initiative(&mut config, branch, &desired);
    write_config(repo_root, &config)?;
    Ok(key)
}

fn reserve_unique_initiative(config: &mut WorkmeshConfig, branch: &str, desired: &str) -> String {
    let used = config.initiatives.get_or_insert_with(Vec::new);
    let map = config
        .branch_initiatives
        .get_or_insert_with(std::collections::HashMap::new);

    let base = desired.trim();
    let base = if base.is_empty() { "work" } else { base };
    let mut key = base.to_string();

    // Ensure we don't reuse another branch's initiative slug.
    // If two branches intentionally share an initiative, the user can set `--id` explicitly.
    if used.iter().any(|k| k == &key) {
        let mut i = 2;
        loop {
            let candidate = format!("{}-{}", base, i);
            if !used.iter().any(|k| k == &candidate) {
                key = candidate;
                break;
            }
            i += 1;
        }
    }

    if !used.iter().any(|k| k == &key) {
        used.push(key.clone());
    }
    map.insert(branch.to_string(), key.clone());
    key
}

pub fn next_namespaced_task_id(tasks: &[Task], initiative: &str) -> String {
    let init = initiative.trim().to_lowercase();
    let init = if init.is_empty() { "work".to_string() } else { init };
    let prefix = format!("task-{}-", init);
    let mut max_num = 0i32;
    for task in tasks {
        let id = task.id.trim().to_lowercase();
        if !id.starts_with(&prefix) {
            continue;
        }
        let rest = &id[prefix.len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<i32>() {
            max_num = max_num.max(n);
        }
    }
    format!("{}{:03}", prefix, max_num + 1)
}

fn slugify(raw: &str) -> Option<String> {
    let s = raw.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    let mut out = String::new();
    let mut last_dash = false;
    for ch in s.chars() {
        let ok = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        if ok {
            out.push(ch);
            last_dash = false;
            continue;
        }
        if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn resolve_initiative_or_error(repo_root: &Path) -> Result<(String, String)> {
    let branch = best_effort_git_branch(repo_root)
        .ok_or_else(|| anyhow!("Unable to infer git branch (set WORKMESH_BRANCH to override)"))?;
    let initiative = ensure_branch_initiative(repo_root, &branch)?;
    Ok((branch, initiative))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn branch_to_initiative_slug_strips_prefixes_and_slugifies() {
        assert_eq!(branch_to_initiative_slug("feature/Login UI"), "login-ui");
        assert_eq!(branch_to_initiative_slug("bugfix/api/Crash-123"), "crash-123");
        assert_eq!(branch_to_initiative_slug("main"), "main");
    }

    #[test]
    fn ensure_branch_initiative_freezes_key_and_avoids_collisions() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();
        let a = ensure_branch_initiative(repo, "feature/login").expect("a");
        assert_eq!(a, "login");
        let b = ensure_branch_initiative(repo, "feature/login").expect("b");
        assert_eq!(a, b);

        // Colliding desired slug from a different branch gets a numeric suffix.
        let x = ensure_branch_initiative(repo, "feat/login").expect("x");
        assert_eq!(x, "login-2");

        // Another distinct slug is accepted as-is.
        let y = ensure_branch_initiative(repo, "feature/billing").expect("y");
        assert_eq!(y, "billing");
    }

    #[test]
    fn reserve_unique_initiative_appends_numeric_suffix() {
        let mut config = WorkmeshConfig::default();
        let a = reserve_unique_initiative(&mut config, "feature/login", "login");
        assert_eq!(a, "login");
        let b = reserve_unique_initiative(&mut config, "feature/login-2", "login");
        assert_eq!(b, "login-2");
    }

    #[test]
    fn next_namespaced_task_id_increments_within_initiative_only() {
        let tasks = vec![
            Task {
                id: "task-login-001".to_string(),
                uid: None,
                kind: "task".to_string(),
                title: "a".to_string(),
                status: "To Do".to_string(),
                priority: "P2".to_string(),
                phase: "Phase1".to_string(),
                dependencies: Vec::new(),
                labels: Vec::new(),
                assignee: Vec::new(),
                relationships: Default::default(),
                lease: None,
                project: None,
                initiative: None,
                created_date: None,
                updated_date: None,
                extra: Default::default(),
                file_path: None,
                body: String::new(),
            },
            Task {
                id: "task-billing-002".to_string(),
                uid: None,
                kind: "task".to_string(),
                title: "b".to_string(),
                status: "To Do".to_string(),
                priority: "P2".to_string(),
                phase: "Phase1".to_string(),
                dependencies: Vec::new(),
                labels: Vec::new(),
                assignee: Vec::new(),
                relationships: Default::default(),
                lease: None,
                project: None,
                initiative: None,
                created_date: None,
                updated_date: None,
                extra: Default::default(),
                file_path: None,
                body: String::new(),
            },
        ];
        assert_eq!(next_namespaced_task_id(&tasks, "login"), "task-login-002");
        assert_eq!(next_namespaced_task_id(&tasks, "billing"), "task-billing-003");
        assert_eq!(next_namespaced_task_id(&tasks, "new"), "task-new-001");
    }
}
