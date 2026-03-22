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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SkillInstallReport {
    pub written: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SkillUninstallReport {
    pub removed: Vec<PathBuf>,
    pub missing: Vec<PathBuf>,
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
const WORKMESH_CLI_SKILL_ID: &str = "workmesh-cli";
const WORKMESH_MCP_SKILL_ID: &str = "workmesh-mcp";

#[derive(Debug, Copy, Clone)]
struct EmbeddedSkillFile {
    relative_path: &'static str,
    content: &'static str,
}

#[derive(Debug, Copy, Clone)]
struct EmbeddedSkill {
    name: &'static str,
    id: &'static str,
    markdown: &'static str,
    files: &'static [EmbeddedSkillFile],
}

const WORKMESH_SKILL_MARKDOWN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../skills/workmesh/SKILL.md"
));
const WORKMESH_SKILL_OPERATING_MODEL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../skills/workmesh/references/OPERATING_MODEL.md"
));
const WORKMESH_CLI_SKILL_MARKDOWN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../skills/workmesh-cli/SKILL.md"
));
const WORKMESH_CLI_SKILL_OPERATING_MODEL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../skills/workmesh-cli/references/OPERATING_MODEL.md"
));
const WORKMESH_MCP_SKILL_MARKDOWN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../skills/workmesh-mcp/SKILL.md"
));
const WORKMESH_MCP_SKILL_OPERATING_MODEL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../skills/workmesh-mcp/references/OPERATING_MODEL.md"
));

const WORKMESH_SKILL_FILES: &[EmbeddedSkillFile] = &[
    EmbeddedSkillFile {
        relative_path: "SKILL.md",
        content: WORKMESH_SKILL_MARKDOWN,
    },
    EmbeddedSkillFile {
        relative_path: "references/OPERATING_MODEL.md",
        content: WORKMESH_SKILL_OPERATING_MODEL,
    },
];

const WORKMESH_CLI_SKILL_FILES: &[EmbeddedSkillFile] = &[
    EmbeddedSkillFile {
        relative_path: "SKILL.md",
        content: WORKMESH_CLI_SKILL_MARKDOWN,
    },
    EmbeddedSkillFile {
        relative_path: "references/OPERATING_MODEL.md",
        content: WORKMESH_CLI_SKILL_OPERATING_MODEL,
    },
];

const WORKMESH_MCP_SKILL_FILES: &[EmbeddedSkillFile] = &[
    EmbeddedSkillFile {
        relative_path: "SKILL.md",
        content: WORKMESH_MCP_SKILL_MARKDOWN,
    },
    EmbeddedSkillFile {
        relative_path: "references/OPERATING_MODEL.md",
        content: WORKMESH_MCP_SKILL_OPERATING_MODEL,
    },
];

pub fn embedded_skill_ids() -> Vec<&'static str> {
    vec![
        WORKMESH_SKILL_ID,
        WORKMESH_CLI_SKILL_ID,
        WORKMESH_MCP_SKILL_ID,
    ]
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
    let skill = embedded_skill(name)?;
    Some(SkillContent {
        name: skill.name.to_string(),
        source: SkillSource::Embedded {
            id: skill.id.to_string(),
        },
        content: skill.markdown.to_string(),
    })
}

fn embedded_skill(name: &str) -> Option<EmbeddedSkill> {
    if name.eq_ignore_ascii_case(WORKMESH_SKILL_ID) {
        return Some(EmbeddedSkill {
            name: WORKMESH_SKILL_ID,
            id: "skills/workmesh/SKILL.md",
            markdown: WORKMESH_SKILL_MARKDOWN,
            files: WORKMESH_SKILL_FILES,
        });
    }
    if name.eq_ignore_ascii_case(WORKMESH_CLI_SKILL_ID) {
        return Some(EmbeddedSkill {
            name: WORKMESH_CLI_SKILL_ID,
            id: "skills/workmesh-cli/SKILL.md",
            markdown: WORKMESH_CLI_SKILL_MARKDOWN,
            files: WORKMESH_CLI_SKILL_FILES,
        });
    }
    if name.eq_ignore_ascii_case(WORKMESH_MCP_SKILL_ID) {
        return Some(EmbeddedSkill {
            name: WORKMESH_MCP_SKILL_ID,
            id: "skills/workmesh-mcp/SKILL.md",
            markdown: WORKMESH_MCP_SKILL_MARKDOWN,
            files: WORKMESH_MCP_SKILL_FILES,
        });
    }
    None
}

fn find_skill_content_on_disk(repo_root: &Path, name: &str) -> Option<SkillContent> {
    // Prefer agent-standard locations first (project-level), then fall back to `skills/` at repo
    // root. Legacy `.codex/skills` and `.cursor/skills` remain as read-only fallbacks for older
    // repos.
    let candidates = [
        repo_root
            .join(".agents")
            .join("skills")
            .join(name)
            .join("SKILL.md"),
        repo_root
            .join(".claude")
            .join("skills")
            .join(name)
            .join("SKILL.md"),
        repo_root
            .join(".codex")
            .join("skills")
            .join(name)
            .join("SKILL.md"),
        repo_root
            .join(".cursor")
            .join("skills")
            .join(name)
            .join("SKILL.md"),
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
    Ok(install_embedded_skill_report(repo_root, scope, agent, name, force)?.written)
}

pub fn install_embedded_skill_report(
    repo_root: Option<&Path>,
    scope: SkillScope,
    agent: SkillAgent,
    name: &str,
    force: bool,
) -> Result<SkillInstallReport> {
    let skill = embedded_skill(name)
        .ok_or_else(|| anyhow!("No embedded skill found with name: {}", name))?;

    let targets = install_targets(repo_root, scope, agent)?;
    let mut report = SkillInstallReport::default();
    for dir in targets {
        let skill_root = dir.join(skill.name);
        for file in skill.files {
            let path = skill_root.join(file.relative_path);
            if path.exists() && !force {
                report.skipped.push(path);
                continue;
            }
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, file.content)?;
            report.written.push(path);
        }
    }
    Ok(report)
}

pub fn detect_user_agents() -> Result<Vec<SkillAgent>> {
    let home =
        home_dir().ok_or_else(|| anyhow!("Unable to resolve home dir; set HOME/USERPROFILE"))?;
    Ok(detect_user_agents_in_home(&home))
}

pub fn detect_user_agents_in_home(home: &Path) -> Vec<SkillAgent> {
    let mut found = Vec::new();
    if home.join(".codex").exists() || home.join(".codex").join("skills").exists() {
        found.push(SkillAgent::Codex);
    }
    if home.join(".claude").exists() || home.join(".claude").join("skills").exists() {
        found.push(SkillAgent::Claude);
    }
    if home.join(".cursor").exists() || home.join(".cursor").join("skills").exists() {
        found.push(SkillAgent::Cursor);
    }
    found
}

pub fn install_embedded_skill_global_auto(name: &str, force: bool) -> Result<Vec<PathBuf>> {
    Ok(install_embedded_skill_global_auto_report(name, force)?.written)
}

pub fn install_embedded_skill_global_auto_report(
    name: &str,
    force: bool,
) -> Result<SkillInstallReport> {
    let home =
        home_dir().ok_or_else(|| anyhow!("Unable to resolve home dir; set HOME/USERPROFILE"))?;
    let agents = detect_user_agents_in_home(&home);
    if agents.is_empty() {
        return Err(anyhow!(
            "No agents detected under {} (expected ~/.codex, ~/.claude, and/or ~/.cursor)",
            home.display()
        ));
    }
    let mut report = SkillInstallReport::default();
    for agent in agents {
        let partial = install_embedded_skill_report(None, SkillScope::User, agent, name, force)?;
        report.written.extend(partial.written);
        report.skipped.extend(partial.skipped);
    }
    Ok(report)
}

pub fn uninstall_embedded_skill(
    repo_root: Option<&Path>,
    scope: SkillScope,
    agent: SkillAgent,
    name: &str,
) -> Result<Vec<PathBuf>> {
    Ok(uninstall_embedded_skill_report(repo_root, scope, agent, name)?.removed)
}

pub fn uninstall_embedded_skill_report(
    repo_root: Option<&Path>,
    scope: SkillScope,
    agent: SkillAgent,
    name: &str,
) -> Result<SkillUninstallReport> {
    let skill = embedded_skill(name)
        .ok_or_else(|| anyhow!("No embedded skill found with name: {}", name))?;

    let targets = install_targets(repo_root, scope, agent)?;
    let mut report = SkillUninstallReport::default();
    for dir in targets {
        let skill_root = dir.join(skill.name);
        for file in skill.files {
            let path = skill_root.join(file.relative_path);
            if path.exists() {
                fs::remove_file(&path)?;
                report.removed.push(path);
            } else {
                report.missing.push(path);
            }
        }
        cleanup_empty_skill_dirs(&skill_root);
    }
    Ok(report)
}

pub fn uninstall_embedded_skill_global_auto(name: &str) -> Result<Vec<PathBuf>> {
    Ok(uninstall_embedded_skill_global_auto_report(name)?.removed)
}

pub fn uninstall_embedded_skill_global_auto_report(name: &str) -> Result<SkillUninstallReport> {
    let home =
        home_dir().ok_or_else(|| anyhow!("Unable to resolve home dir; set HOME/USERPROFILE"))?;
    let agents = detect_user_agents_in_home(&home);
    if agents.is_empty() {
        return Err(anyhow!(
            "No agents detected under {} (expected ~/.codex, ~/.claude, and/or ~/.cursor)",
            home.display()
        ));
    }
    let mut report = SkillUninstallReport::default();
    for agent in agents {
        let partial = uninstall_embedded_skill_report(None, SkillScope::User, agent, name)?;
        report.removed.extend(partial.removed);
        report.missing.extend(partial.missing);
    }
    Ok(report)
}

fn install_targets(
    repo_root: Option<&Path>,
    scope: SkillScope,
    agent: SkillAgent,
) -> Result<Vec<PathBuf>> {
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
                let root = user_skill_root(&home, a);
                if !roots.contains(&root) {
                    roots.push(root);
                }
            }
        }
        SkillScope::Project => {
            let root = repo_root.ok_or_else(|| anyhow!("Project scope requires a repo root"))?;
            for a in agents {
                let project_root = project_skill_root(root, a);
                if !roots.contains(&project_root) {
                    roots.push(project_root);
                }
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
        SkillAgent::Codex => repo_root.join(".agents").join("skills"),
        SkillAgent::Claude => repo_root.join(".claude").join("skills"),
        SkillAgent::Cursor => repo_root.join(".agents").join("skills"),
        SkillAgent::All => repo_root.join(".agents").join("skills"),
    }
}

fn cleanup_empty_skill_dirs(skill_root: &Path) {
    let references_dir = skill_root.join("references");
    if is_empty_dir(&references_dir) {
        let _ = fs::remove_dir(&references_dir);
    }
    if is_empty_dir(skill_root) {
        let _ = fs::remove_dir(skill_root);
    }
}

fn is_empty_dir(path: &Path) -> bool {
    path.is_dir()
        && path
            .read_dir()
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
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

    fn with_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
        with_env_lock(|| {
            let _env_guard = EnvGuard::capture();
            std::env::set_var("HOME", home);
            std::env::remove_var("USERPROFILE");
            f()
        })
    }

    #[test]
    fn embedded_skill_is_available() {
        let skill = load_skill_content(None, "workmesh").expect("skill");
        assert!(skill.content.contains("# WorkMesh Router Skill"));
        assert_eq!(
            skill.source,
            SkillSource::Embedded {
                id: "skills/workmesh/SKILL.md".to_string()
            }
        );
    }

    #[test]
    fn embedded_skill_catalog_includes_cli_and_mcp_profiles() {
        let ids = embedded_skill_ids();
        assert!(ids.contains(&"workmesh"));
        assert!(ids.contains(&"workmesh-cli"));
        assert!(ids.contains(&"workmesh-mcp"));
    }

    #[test]
    fn disk_skill_is_preferred_over_embedded_when_present() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();
        let path = repo
            .join(".agents")
            .join("skills")
            .join("workmesh")
            .join("SKILL.md");
        fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
        fs::write(
            &path,
            "---\nname: workmesh\ndescription: test\n---\n# from disk\n",
        )
        .expect("write");

        let skill = load_skill_content(Some(repo), "workmesh").expect("skill");
        assert!(skill.content.contains("# from disk"));
        assert_eq!(skill.source, SkillSource::File { path });
    }

    #[test]
    fn install_embedded_writes_to_user_dirs() {
        let temp = TempDir::new().expect("tempdir");
        with_home(temp.path(), || {
            let written =
                install_embedded_skill(None, SkillScope::User, SkillAgent::Codex, "workmesh", true)
                    .expect("install");
            assert_eq!(written.len(), 2);
            let suffix = Path::new(".codex")
                .join("skills")
                .join("workmesh")
                .join("SKILL.md");
            let reference_suffix = Path::new(".codex")
                .join("skills")
                .join("workmesh")
                .join("references")
                .join("OPERATING_MODEL.md");
            assert!(written.iter().any(|path| path.ends_with(&suffix)));
            assert!(written.iter().any(|path| path.ends_with(&reference_suffix)));
            let installed_path = written
                .iter()
                .find(|path| path.ends_with(&suffix))
                .expect("skill path");
            assert!(fs::read_to_string(installed_path)
                .unwrap()
                .contains("# WorkMesh Router Skill"));
        });
    }

    #[test]
    fn install_embedded_cli_profile_writes_to_user_dirs() {
        let temp = TempDir::new().expect("tempdir");
        with_home(temp.path(), || {
            let written = install_embedded_skill(
                None,
                SkillScope::User,
                SkillAgent::Codex,
                "workmesh-cli",
                true,
            )
            .expect("install");
            assert_eq!(written.len(), 2);
            let suffix = Path::new(".codex")
                .join("skills")
                .join("workmesh-cli")
                .join("SKILL.md");
            let installed_path = written
                .iter()
                .find(|path| path.ends_with(&suffix))
                .expect("skill path");
            assert!(fs::read_to_string(installed_path)
                .unwrap()
                .contains("# WorkMesh CLI Skill"));
        });
    }

    #[test]
    fn install_embedded_mcp_profile_writes_to_user_dirs() {
        let temp = TempDir::new().expect("tempdir");
        with_home(temp.path(), || {
            let written = install_embedded_skill(
                None,
                SkillScope::User,
                SkillAgent::Codex,
                "workmesh-mcp",
                true,
            )
            .expect("install");
            assert_eq!(written.len(), 2);
            let suffix = Path::new(".codex")
                .join("skills")
                .join("workmesh-mcp")
                .join("SKILL.md");
            let installed_path = written
                .iter()
                .find(|path| path.ends_with(&suffix))
                .expect("skill path");
            assert!(fs::read_to_string(installed_path)
                .unwrap()
                .contains("# WorkMesh MCP Skill"));
        });
    }

    #[test]
    fn detect_user_agents_only_returns_existing_dirs() {
        let temp = TempDir::new().expect("tempdir");
        let home = temp.path();
        fs::create_dir_all(home.join(".codex")).expect("codex dir");
        fs::create_dir_all(home.join(".cursor").join("skills")).expect("cursor skills dir");

        let found = detect_user_agents_in_home(home);
        assert_eq!(found, vec![SkillAgent::Codex, SkillAgent::Cursor]);
    }

    #[test]
    fn install_global_auto_writes_only_to_detected_agents() {
        let temp = TempDir::new().expect("tempdir");
        with_home(temp.path(), || {
            // Detect only Codex.
            fs::create_dir_all(temp.path().join(".codex")).expect("codex dir");

            let written = install_embedded_skill_global_auto("workmesh", true).expect("install");
            assert_eq!(written.len(), 2);
            let suffix = Path::new(".codex")
                .join("skills")
                .join("workmesh")
                .join("SKILL.md");
            assert!(written.iter().any(|path| path.ends_with(&suffix)));
            assert!(!temp.path().join(".claude").exists());
            assert!(!temp.path().join(".cursor").exists());
        });
    }

    #[test]
    fn load_skill_content_returns_none_when_name_is_blank() {
        assert_eq!(load_skill_content(None, ""), None);
        assert_eq!(load_skill_content(None, "   \n\t"), None);
    }

    #[test]
    fn load_skill_content_falls_back_to_embedded_when_disk_missing() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();
        let skill = load_skill_content(Some(repo), "workmesh").expect("skill");
        assert!(matches!(skill.source, SkillSource::Embedded { .. }));
    }

    #[test]
    fn embedded_skill_lookup_is_case_insensitive() {
        let skill = load_skill_content(None, "WORKMESH").expect("skill");
        assert!(matches!(skill.source, SkillSource::Embedded { .. }));
    }

    #[test]
    fn install_embedded_skill_errors_for_unknown_skill() {
        let err = install_embedded_skill(None, SkillScope::User, SkillAgent::Codex, "nope", true)
            .unwrap_err();
        assert!(format!("{err:#}").contains("No embedded skill"));
    }

    #[test]
    fn install_embedded_skill_project_scope_requires_repo_root() {
        let err = install_embedded_skill(
            None,
            SkillScope::Project,
            SkillAgent::Codex,
            "workmesh",
            true,
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains("Project scope requires a repo root"));
    }

    #[test]
    fn install_embedded_skill_project_scope_uses_agent_standard_path() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();

        let written = install_embedded_skill(
            Some(repo),
            SkillScope::Project,
            SkillAgent::Codex,
            "workmesh",
            true,
        )
        .expect("install");

        let suffix = Path::new(".agents")
            .join("skills")
            .join("workmesh")
            .join("SKILL.md");
        assert!(written.iter().any(|path| path.ends_with(&suffix)));
    }

    #[test]
    fn legacy_project_skill_path_remains_loadable() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();
        let path = repo
            .join(".codex")
            .join("skills")
            .join("workmesh")
            .join("SKILL.md");
        fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
        fs::write(
            &path,
            "---\nname: workmesh\ndescription: test\n---\n# legacy disk skill\n",
        )
        .expect("write");

        let skill = load_skill_content(Some(repo), "workmesh").expect("skill");
        assert!(skill.content.contains("# legacy disk skill"));
    }

    #[test]
    fn install_embedded_skill_force_false_skips_existing_file() {
        let temp = TempDir::new().expect("tempdir");
        with_home(temp.path(), || {
            // First write (force=true), then ensure the second pass (force=false) doesn't overwrite.
            let written1 =
                install_embedded_skill(None, SkillScope::User, SkillAgent::Codex, "workmesh", true)
                    .expect("install 1");
            assert_eq!(written1.len(), 2);
            let path = written1
                .iter()
                .find(|path| path.ends_with(Path::new("SKILL.md")))
                .expect("skill path")
                .clone();
            fs::write(&path, "do not overwrite").expect("overwrite with sentinel");

            let report = install_embedded_skill_report(
                None,
                SkillScope::User,
                SkillAgent::Codex,
                "workmesh",
                false,
            )
            .expect("install 2");
            assert!(report.written.is_empty());
            assert!(report.skipped.contains(&path));
            assert_eq!(fs::read_to_string(&path).expect("read"), "do not overwrite");
        });
    }

    #[test]
    fn install_global_auto_errors_when_no_agents_detected() {
        let temp = TempDir::new().expect("tempdir");
        with_home(temp.path(), || {
            let err = install_embedded_skill_global_auto("workmesh", true).unwrap_err();
            assert!(format!("{err:#}").contains("No agents detected"));
        });
    }

    #[test]
    fn uninstall_embedded_skill_removes_installed_file() {
        let temp = TempDir::new().expect("tempdir");
        with_home(temp.path(), || {
            let written =
                install_embedded_skill(None, SkillScope::User, SkillAgent::Codex, "workmesh", true)
                    .expect("install");
            assert_eq!(written.len(), 2);
            for path in &written {
                assert!(path.exists());
            }

            let report = uninstall_embedded_skill_report(
                None,
                SkillScope::User,
                SkillAgent::Codex,
                "workmesh",
            )
            .expect("uninstall");
            assert_eq!(report.removed.len(), 2);
            assert!(report.missing.is_empty());
            for path in &written {
                assert!(!path.exists());
            }
        });
    }

    #[test]
    fn uninstall_embedded_skill_reports_missing_file() {
        let temp = TempDir::new().expect("tempdir");
        with_home(temp.path(), || {
            let report = uninstall_embedded_skill_report(
                None,
                SkillScope::User,
                SkillAgent::Codex,
                "workmesh",
            )
            .expect("uninstall");
            assert!(report.removed.is_empty());
            assert_eq!(report.missing.len(), 2);
            let suffix = Path::new(".codex")
                .join("skills")
                .join("workmesh")
                .join("SKILL.md");
            assert!(report.missing.iter().any(|path| path.ends_with(&suffix)));
        });
    }

    #[test]
    fn uninstall_global_auto_reports_removed_and_missing() {
        let temp = TempDir::new().expect("tempdir");
        with_home(temp.path(), || {
            fs::create_dir_all(temp.path().join(".codex")).expect("codex dir");
            let written =
                install_embedded_skill(None, SkillScope::User, SkillAgent::Codex, "workmesh", true)
                    .expect("install");
            assert_eq!(written.len(), 2);

            let report =
                uninstall_embedded_skill_global_auto_report("workmesh").expect("uninstall");
            assert_eq!(report.removed.len(), 2);
            assert!(report.missing.is_empty());
        });
    }

    #[test]
    fn detect_user_agents_errors_when_home_is_unset() {
        with_env_lock(|| {
            let old_home = std::env::var("HOME").ok();
            let old_profile = std::env::var("USERPROFILE").ok();
            std::env::remove_var("HOME");
            std::env::remove_var("USERPROFILE");

            let err = detect_user_agents().unwrap_err();
            assert!(format!("{err:#}").contains("Unable to resolve home dir"));

            match old_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match old_profile {
                Some(v) => std::env::set_var("USERPROFILE", v),
                None => std::env::remove_var("USERPROFILE"),
            }
        });
    }
}
