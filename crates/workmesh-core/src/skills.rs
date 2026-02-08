use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum SkillSource {
    #[serde(rename = "file")]
    File { path: PathBuf },
    #[serde(rename = "embedded")]
    Embedded { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillContent {
    pub name: String,
    pub source: SkillSource,
    pub content: String,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillScope {
    User,
    Project,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillAgent {
    Codex,
    Claude,
    Cursor,
    All,
}

const WORKMESH_SKILL_ID: &str = "workmesh";
const WORKMESH_SKILL_MARKDOWN: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../skills/workmesh/SKILL.md"));

pub fn embedded_skill_ids() -> Vec<&'static str> {
    vec![WORKMESH_SKILL_ID]
}

pub fn load_skill_content(repo_root: Option<&Path>, name: &str) -> Option<SkillContent> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(root) = repo_root {
        if let Some(found) = find_skill_content_on_disk(root, trimmed) {
            return Some(found);
        }
    }
    embedded_skill_content(trimmed)
}

fn embedded_skill_content(name: &str) -> Option<SkillContent> {
    if name.eq_ignore_ascii_case(WORKMESH_SKILL_ID) {
        return Some(SkillContent {
            name: WORKMESH_SKILL_ID.to_string(),
            source: SkillSource::Embedded {
                id: "skills/workmesh/SKILL.md".to_string(),
            },
            content: WORKMESH_SKILL_MARKDOWN.to_string(),
        });
    }
    None
}

fn find_skill_content_on_disk(repo_root: &Path, name: &str) -> Option<SkillContent> {
    // Prefer agent-standard locations first (project-level), then fall back to `skills/` at repo root.
    let candidates = [
        repo_root.join(".codex").join("skills").join(name).join("SKILL.md"),
        repo_root.join(".claude").join("skills").join(name).join("SKILL.md"),
        repo_root.join(".cursor").join("skills").join(name).join("SKILL.md"),
        repo_root.join("skills").join(name).join("SKILL.md"),
    ];
    for path in candidates {
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path).ok()?;
        return Some(SkillContent {
            name: name.to_string(),
            source: SkillSource::File { path },
            content,
        });
    }
    None
}

pub fn install_embedded_skill(
    repo_root: Option<&Path>,
    scope: SkillScope,
    agent: SkillAgent,
    name: &str,
    force: bool,
) -> Result<Vec<PathBuf>> {
    let skill = embedded_skill_content(name)
        .ok_or_else(|| anyhow!("No embedded skill found with name: {}", name))?;

    let targets = install_targets(repo_root, scope, agent)?;
    let mut written = Vec::new();
    for dir in targets {
        let path = dir.join(&skill.name).join("SKILL.md");
        if path.exists() && !force {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &skill.content)?;
        written.push(path);
    }
    Ok(written)
}

fn install_targets(repo_root: Option<&Path>, scope: SkillScope, agent: SkillAgent) -> Result<Vec<PathBuf>> {
    let agents = match agent {
        SkillAgent::All => vec![SkillAgent::Codex, SkillAgent::Claude, SkillAgent::Cursor],
        other => vec![other],
    };

    let mut roots = Vec::new();
    match scope {
        SkillScope::User => {
            let home = home_dir()
                .ok_or_else(|| anyhow!("Unable to resolve home dir; set HOME/USERPROFILE"))?;
            for a in agents {
                roots.push(user_skill_root(&home, a));
            }
        }
        SkillScope::Project => {
            let root = repo_root.ok_or_else(|| anyhow!("Project scope requires a repo root"))?;
            for a in agents {
                roots.push(project_skill_root(root, a));
            }
        }
    }
    Ok(roots)
}

fn user_skill_root(home: &Path, agent: SkillAgent) -> PathBuf {
    match agent {
        SkillAgent::Codex => home.join(".codex").join("skills"),
        SkillAgent::Claude => home.join(".claude").join("skills"),
        SkillAgent::Cursor => home.join(".cursor").join("skills"),
        SkillAgent::All => home.join(".codex").join("skills"),
    }
}

fn project_skill_root(repo_root: &Path, agent: SkillAgent) -> PathBuf {
    match agent {
        SkillAgent::Codex => repo_root.join(".codex").join("skills"),
        SkillAgent::Claude => repo_root.join(".claude").join("skills"),
        SkillAgent::Cursor => repo_root.join(".cursor").join("skills"),
        SkillAgent::All => repo_root.join(".codex").join("skills"),
    }
}

fn home_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok().map(|value| value.trim().to_string());
    if let Some(home) = home {
        if !home.is_empty() {
            return Some(PathBuf::from(home));
        }
    }
    let profile = std::env::var("USERPROFILE").ok().map(|value| value.trim().to_string());
    if let Some(profile) = profile {
        if !profile.is_empty() {
            return Some(PathBuf::from(profile));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn embedded_skill_is_available() {
        let skill = load_skill_content(None, "workmesh").expect("skill");
        assert!(skill.content.contains("# WorkMesh skill"));
        assert_eq!(
            skill.source,
            SkillSource::Embedded {
                id: "skills/workmesh/SKILL.md".to_string()
            }
        );
    }

    #[test]
    fn disk_skill_is_preferred_over_embedded_when_present() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();
        let path = repo
            .join(".codex")
            .join("skills")
            .join("workmesh")
            .join("SKILL.md");
        fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
        fs::write(&path, "---\nname: workmesh\ndescription: test\n---\n# from disk\n").expect("write");

        let skill = load_skill_content(Some(repo), "workmesh").expect("skill");
        assert!(skill.content.contains("# from disk"));
        assert_eq!(skill.source, SkillSource::File { path });
    }

    #[test]
    fn install_embedded_writes_to_user_dirs() {
        let temp = TempDir::new().expect("tempdir");
        std::env::set_var("HOME", temp.path());
        std::env::remove_var("USERPROFILE");

        let written =
            install_embedded_skill(None, SkillScope::User, SkillAgent::Codex, "workmesh", true)
                .expect("install");
        assert_eq!(written.len(), 1);
        assert!(written[0]
            .to_string_lossy()
            .ends_with(".codex/skills/workmesh/SKILL.md"));
        assert!(fs::read_to_string(&written[0]).unwrap().contains("# WorkMesh skill"));
    }
}

