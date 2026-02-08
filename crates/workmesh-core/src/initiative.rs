use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};

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

fn four_letter_key_from_slug(slug: &str) -> String {
    // "4 letters" means ASCII a-z only. Non-letters are ignored.
    // If the result is shorter than 4, pad with 'x' (stable + readable).
    let mut out = String::new();
    for ch in slug.to_lowercase().chars() {
        if ch.is_ascii_lowercase() {
            out.push(ch);
            if out.len() == 4 {
                break;
            }
        }
    }
    while out.len() < 4 {
        out.push('x');
    }
    out
}

fn four_letter_key_candidates<'a>(
    branch: &'a str,
    desired: &'a str,
) -> impl Iterator<Item = String> + 'a {
    // Candidate order:
    // 1. desired (usually derived from the branch name segment)
    // 2. deterministic 4-letter keys derived from SHA256(branch), then SHA256(branch + ":<n>") if needed.
    let mut emitted_desired = false;
    std::iter::from_fn(move || {
        if !emitted_desired {
            emitted_desired = true;
            return Some(desired.to_string());
        }
        None
    })
    .chain(FourLetterHashCandidates::new(branch))
}

struct FourLetterHashCandidates<'a> {
    branch: &'a str,
    salt: u32,
    offset: usize,
    bytes: [u8; 32],
}

impl<'a> FourLetterHashCandidates<'a> {
    fn new(branch: &'a str) -> Self {
        let bytes = sha256_bytes(branch);
        Self {
            branch,
            salt: 0,
            offset: 0,
            bytes,
        }
    }
}

impl<'a> Iterator for FourLetterHashCandidates<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        // Each 32-byte digest yields 8 candidates (4 bytes each).
        if self.offset + 4 > self.bytes.len() {
            self.salt = self.salt.saturating_add(1);
            self.offset = 0;
            self.bytes = sha256_bytes(&format!("{}:{}", self.branch, self.salt));
        }
        let chunk = &self.bytes[self.offset..self.offset + 4];
        self.offset += 4;
        let mut key = String::new();
        for b in chunk {
            let letter = (b % 26) as u8;
            key.push((b'a' + letter) as char);
        }
        Some(key)
    }
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

    let desired_slug = branch_to_initiative_slug(branch);
    let desired = four_letter_key_from_slug(&desired_slug);
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

    // Ensure we don't reuse another branch's 4-letter initiative key.
    // If two branches intentionally share an initiative, the user can set `--id` explicitly.
    let mut key = None;
    for candidate in four_letter_key_candidates(branch, base) {
        let candidate = candidate.trim().to_string();
        if candidate.len() != 4 || !candidate.chars().all(|c| c.is_ascii_lowercase()) {
            continue;
        }
        if !used.iter().any(|k| k == &candidate) {
            key = Some(candidate);
            break;
        }
    }
    let key = key.unwrap_or_else(|| "work".to_string());

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

fn sha256_bytes(input: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..]);
    out
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
        assert_eq!(a, "logi");
        let b = ensure_branch_initiative(repo, "feature/login").expect("b");
        assert_eq!(a, b);

        // Colliding desired key from a different branch gets a different 4-letter key.
        let x = ensure_branch_initiative(repo, "feat/login").expect("x");
        assert_ne!(x, "logi");
        assert_eq!(x.len(), 4);
        assert!(x.chars().all(|c| c.is_ascii_lowercase()));

        // Another distinct slug is accepted as-is.
        let y = ensure_branch_initiative(repo, "feature/billing").expect("y");
        assert_eq!(y, "bill");
    }

    #[test]
    fn reserve_unique_initiative_dedup_keeps_length_4() {
        let mut config = WorkmeshConfig::default();
        let a = reserve_unique_initiative(&mut config, "feature/login", "logi");
        assert_eq!(a, "logi");
        let b = reserve_unique_initiative(&mut config, "feature/login-2", "logi");
        assert_ne!(b, "logi");
        assert_eq!(b.len(), 4);
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
