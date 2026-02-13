use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::audit::{read_recent_audit_events, AuditEvent};
use crate::project::{ensure_project_docs, project_docs_dir, repo_root_from_backlog};
use crate::task::Task;
use crate::task_ops::{is_lease_active, ready_tasks};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Failed to write checkpoint: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to ensure project docs: {0}")]
    Project(#[from] crate::project::ProjectError),
    #[error("Failed to parse checkpoint: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseSummary {
    pub owner: String,
    pub acquired_at: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub uid: Option<String>,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub phase: String,
    pub project: Option<String>,
    pub initiative: Option<String>,
    pub lease: Option<LeaseSummary>,
}

impl TaskSummary {
    pub fn line(&self) -> String {
        let title = if self.title.trim().is_empty() {
            "(untitled)"
        } else {
            self.title.as_str()
        };
        format!(
            "{} | {} | {} | {} | {}",
            self.id, self.status, self.priority, self.phase, title
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSummary {
    pub available: bool,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: Option<i64>,
    pub behind: Option<i64>,
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSnapshot {
    pub checkpoint_id: String,
    pub generated_at: String,
    pub project_id: String,
    pub repo_root: String,
    pub backlog_dir: String,
    pub current_task: Option<TaskSummary>,
    pub ready: Vec<TaskSummary>,
    pub leases: Vec<TaskSummary>,
    pub git: GitSummary,
    pub changed_files: Vec<String>,
    pub top_level_dirs: Vec<String>,
    pub audit_events: Vec<AuditEvent>,
}

#[derive(Debug, Clone)]
pub struct CheckpointResult {
    pub snapshot: CheckpointSnapshot,
    pub json_path: PathBuf,
    pub markdown_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CheckpointOptions {
    pub project_id: Option<String>,
    pub checkpoint_id: Option<String>,
    pub audit_limit: usize,
}

#[derive(Debug, Clone)]
pub struct ResumeSummary {
    pub snapshot: CheckpointSnapshot,
    pub working_set: Option<String>,
    pub checkpoint_path: PathBuf,
}

pub fn write_checkpoint(
    backlog_dir: &Path,
    tasks: &[Task],
    options: &CheckpointOptions,
) -> Result<CheckpointResult, SessionError> {
    let repo_root = repo_root_from_backlog(backlog_dir);
    let project_id = resolve_project_id(&repo_root, tasks, options.project_id.as_deref());
    ensure_project_docs(&repo_root, &project_id, None)?;

    let updates_dir = project_docs_dir(&repo_root, &project_id).join("updates");
    fs::create_dir_all(&updates_dir)?;

    let checkpoint_id = options
        .checkpoint_id
        .clone()
        .unwrap_or_else(default_checkpoint_id);
    let generated_at = Local::now().format("%Y-%m-%d %H:%M").to_string();

    let (git_summary, changed_files) = git_status(&repo_root);
    let top_level_dirs = top_level_dirs(&changed_files);
    let audit_events = read_recent_audit_events(backlog_dir, options.audit_limit);

    let current_task = pick_current_task(tasks).map(task_to_summary);
    let ready = ready_tasks(tasks)
        .iter()
        .map(|task| task_to_summary(task))
        .collect::<Vec<_>>();
    let leases = active_lease_tasks(tasks)
        .into_iter()
        .map(task_to_summary)
        .collect::<Vec<_>>();

    let snapshot = CheckpointSnapshot {
        checkpoint_id: checkpoint_id.clone(),
        generated_at,
        project_id: project_id.clone(),
        repo_root: repo_root.display().to_string(),
        backlog_dir: backlog_dir.display().to_string(),
        current_task,
        ready,
        leases,
        git: git_summary,
        changed_files,
        top_level_dirs,
        audit_events,
    };

    let json_path = updates_dir.join(format!("checkpoint-{}.json", checkpoint_id));
    let markdown_path = updates_dir.join(format!("checkpoint-{}.md", checkpoint_id));

    fs::write(
        &json_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_default(),
    )?;
    fs::write(&markdown_path, render_checkpoint_markdown(&snapshot))?;

    Ok(CheckpointResult {
        snapshot,
        json_path,
        markdown_path,
    })
}

pub fn load_checkpoint(
    repo_root: &Path,
    project_id: &str,
    checkpoint_id: Option<&str>,
) -> Result<Option<(CheckpointSnapshot, PathBuf)>, SessionError> {
    let path = match resolve_checkpoint_path(repo_root, project_id, checkpoint_id) {
        Some(path) => path,
        None => return Ok(None),
    };
    let content = fs::read_to_string(&path)?;
    let snapshot = serde_json::from_str::<CheckpointSnapshot>(&content)?;
    Ok(Some((snapshot, path)))
}

pub fn resume_summary(
    repo_root: &Path,
    project_id: &str,
    checkpoint_id: Option<&str>,
) -> Result<Option<ResumeSummary>, SessionError> {
    let Some((snapshot, path)) = load_checkpoint(repo_root, project_id, checkpoint_id)? else {
        return Ok(None);
    };
    let working_set_path = project_docs_dir(repo_root, project_id)
        .join("updates")
        .join("working-set.md");
    let working_set = fs::read_to_string(&working_set_path).ok();
    Ok(Some(ResumeSummary {
        snapshot,
        working_set,
        checkpoint_path: path,
    }))
}

pub fn diff_since_checkpoint(
    repo_root: &Path,
    backlog_dir: &Path,
    tasks: &[Task],
    checkpoint: &CheckpointSnapshot,
) -> DiffReport {
    let mut updated_tasks = Vec::new();
    let checkpoint_time = parse_timestamp(&checkpoint.generated_at);
    for task in tasks {
        let updated = task
            .updated_date
            .as_deref()
            .or(task.created_date.as_deref());
        if let (Some(updated), Some(checkpoint_time)) = (updated, checkpoint_time) {
            if let Ok(updated_time) = NaiveDateTime::parse_from_str(updated, "%Y-%m-%d %H:%M") {
                if updated_time >= checkpoint_time {
                    updated_tasks.push(task_to_summary(task));
                }
            }
        }
    }
    updated_tasks.sort_by(|a, b| a.id.cmp(&b.id));

    let (_, current_files) = git_status(repo_root);
    let mut new_files: Vec<String> = current_files
        .iter()
        .filter(|path| !checkpoint.changed_files.contains(path))
        .cloned()
        .collect();
    new_files.sort();

    let audit_events = read_recent_audit_events(backlog_dir, 10);

    DiffReport {
        checkpoint_id: checkpoint.checkpoint_id.clone(),
        checkpoint_time: checkpoint.generated_at.clone(),
        updated_tasks,
        new_files,
        audit_events,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffReport {
    pub checkpoint_id: String,
    pub checkpoint_time: String,
    pub updated_tasks: Vec<TaskSummary>,
    pub new_files: Vec<String>,
    pub audit_events: Vec<AuditEvent>,
}

pub fn render_resume(summary: &ResumeSummary) -> String {
    let snapshot = &summary.snapshot;
    let mut lines = Vec::new();
    lines.push(format!("Resume from checkpoint {}", snapshot.checkpoint_id));
    lines.push(format!("Generated: {}", snapshot.generated_at));
    lines.push(format!("Project: {}", snapshot.project_id));
    lines.push(String::new());

    lines.push("Current task:".to_string());
    if let Some(task) = snapshot.current_task.as_ref() {
        lines.push(format!("- {}", task.line()));
    } else {
        lines.push("- None".to_string());
    }

    lines.push(String::new());
    lines.push("Ready tasks:".to_string());
    for task in snapshot.ready.iter().take(5) {
        lines.push(format!("- {}", task.line()));
    }
    if snapshot.ready.is_empty() {
        lines.push("- None".to_string());
    }

    lines.push(String::new());
    lines.push("Next actions:".to_string());
    if let Some(task) = snapshot.current_task.as_ref() {
        lines.push(format!("- Continue {}", task.id));
    } else if let Some(task) = snapshot.ready.first() {
        lines.push(format!("- Start {}", task.id));
    } else {
        lines.push("- Review tasks for next work item".to_string());
    }

    if let Some(working_set) = summary.working_set.as_ref() {
        lines.push(String::new());
        lines.push("Working set:".to_string());
        for line in working_set.lines() {
            lines.push(line.to_string());
        }
    }

    lines.join("\n")
}

pub fn render_diff(report: &DiffReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("Diff since checkpoint {}", report.checkpoint_id));
    lines.push(format!("Checkpoint time: {}", report.checkpoint_time));
    lines.push(String::new());

    lines.push("Updated tasks:".to_string());
    if report.updated_tasks.is_empty() {
        lines.push("- None".to_string());
    } else {
        for task in &report.updated_tasks {
            lines.push(format!("- {}", task.line()));
        }
    }

    lines.push(String::new());
    lines.push("New changed files:".to_string());
    if report.new_files.is_empty() {
        lines.push("- None".to_string());
    } else {
        for path in &report.new_files {
            lines.push(format!("- {}", path));
        }
    }

    if !report.audit_events.is_empty() {
        lines.push(String::new());
        lines.push("Recent audit events:".to_string());
        for event in &report.audit_events {
            let task = event.task_id.as_deref().unwrap_or("-");
            lines.push(format!(
                "- {} | {} | {}",
                event.timestamp, event.action, task
            ));
        }
    }

    lines.join("\n")
}

pub fn write_working_set(
    repo_root: &Path,
    project_id: &str,
    tasks: &[TaskSummary],
    note: Option<&str>,
) -> Result<PathBuf, SessionError> {
    let updates_dir = project_docs_dir(repo_root, project_id).join("updates");
    fs::create_dir_all(&updates_dir)?;
    let path = updates_dir.join("working-set.md");
    let mut lines = Vec::new();
    lines.push("# Working Set".to_string());
    lines.push(String::new());
    if tasks.is_empty() {
        lines.push("- No active tasks".to_string());
    } else {
        for task in tasks {
            lines.push(format!("- {}", task.line()));
        }
    }
    if let Some(note) = note {
        if !note.trim().is_empty() {
            lines.push(String::new());
            lines.push("## Notes".to_string());
            lines.push(note.trim().to_string());
        }
    }
    fs::write(&path, lines.join("\n"))?;
    Ok(path)
}

pub fn append_session_journal(
    repo_root: &Path,
    project_id: &str,
    task_id: Option<&str>,
    next_action: Option<&str>,
    note: Option<&str>,
) -> Result<PathBuf, SessionError> {
    let updates_dir = project_docs_dir(repo_root, project_id).join("updates");
    fs::create_dir_all(&updates_dir)?;
    let path = updates_dir.join("session-journal.md");
    let timestamp = Local::now().format("%Y-%m-%d %H:%M");
    let mut entry = Vec::new();
    entry.push(format!("## {}", timestamp));
    if let Some(task_id) = task_id {
        if !task_id.trim().is_empty() {
            entry.push(format!("- Task: {}", task_id.trim()));
        }
    }
    if let Some(next) = next_action {
        if !next.trim().is_empty() {
            entry.push(format!("- Next: {}", next.trim()));
        }
    }
    if let Some(note) = note {
        if !note.trim().is_empty() {
            entry.push(format!("- Note: {}", note.trim()));
        }
    }
    entry.push(String::new());

    let mut content = String::new();
    if path.exists() {
        content = fs::read_to_string(&path)?;
    } else {
        content.push_str("# Session Journal\n\n");
    }
    content.push_str(&entry.join("\n"));
    fs::write(&path, content)?;
    Ok(path)
}

pub fn resolve_project_id(repo_root: &Path, tasks: &[Task], explicit: Option<&str>) -> String {
    if let Some(value) = explicit {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let mut task_projects: Vec<String> = tasks
        .iter()
        .filter_map(|task| task.project.as_ref())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    task_projects.sort();
    task_projects.dedup();
    if task_projects.len() == 1 {
        return task_projects[0].clone();
    }

    let docs_projects = repo_root.join("docs").join("projects");
    if let Ok(entries) = fs::read_dir(&docs_projects) {
        let mut dirs = Vec::new();
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        dirs.push(name.to_string());
                    }
                }
            }
        }
        dirs.sort();
        if dirs.len() == 1 {
            return dirs[0].clone();
        }
    }

    repo_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("project")
        .to_lowercase()
}

fn resolve_checkpoint_path(
    repo_root: &Path,
    project_id: &str,
    checkpoint_id: Option<&str>,
) -> Option<PathBuf> {
    let updates_dir = project_docs_dir(repo_root, project_id).join("updates");
    if let Some(id) = checkpoint_id {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return None;
        }
        let candidate = PathBuf::from(trimmed);
        if candidate.is_absolute() && candidate.exists() {
            return Some(candidate);
        }
        let relative = repo_root.join(trimmed);
        if relative.exists() {
            return Some(relative);
        }
        let normalized = if trimmed.starts_with("checkpoint-") {
            trimmed.to_string()
        } else {
            format!("checkpoint-{}", trimmed)
        };
        let name = if normalized.ends_with(".json") {
            normalized
        } else {
            format!("{}.json", normalized)
        };
        let candidate = updates_dir.join(name);
        if candidate.exists() {
            return Some(candidate);
        }
        return None;
    }

    let entries = fs::read_dir(&updates_dir).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("checkpoint-") && name.ends_with(".json"))
                .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates.pop()
}

fn default_checkpoint_id() -> String {
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn pick_current_task(tasks: &[Task]) -> Option<&Task> {
    let mut active: Vec<&Task> = tasks
        .iter()
        .filter(|task| task.status.eq_ignore_ascii_case("in progress"))
        .collect();
    active.sort_by(|a, b| a.id_num().cmp(&b.id_num()));
    active.first().copied()
}

fn active_lease_tasks(tasks: &[Task]) -> Vec<&Task> {
    let mut leased: Vec<&Task> = tasks.iter().filter(|task| is_lease_active(task)).collect();
    leased.sort_by(|a, b| a.id_num().cmp(&b.id_num()));
    leased
}

fn task_to_summary(task: &Task) -> TaskSummary {
    TaskSummary {
        id: task.id.clone(),
        uid: task.uid.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority.clone(),
        phase: task.phase.clone(),
        project: task.project.clone(),
        initiative: task.initiative.clone(),
        lease: task.lease.as_ref().map(|lease| LeaseSummary {
            owner: lease.owner.clone(),
            acquired_at: lease.acquired_at.clone(),
            expires_at: lease.expires_at.clone(),
        }),
    }
}

pub fn task_summary(task: &Task) -> TaskSummary {
    task_to_summary(task)
}

fn git_status(repo_root: &Path) -> (GitSummary, Vec<String>) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("status")
        .arg("--porcelain=v1")
        .arg("-b")
        .output();
    let output = match output {
        Ok(output) if output.status.success() => output,
        _ => {
            return (
                GitSummary {
                    available: false,
                    branch: None,
                    upstream: None,
                    ahead: None,
                    behind: None,
                    staged: 0,
                    unstaged: 0,
                    untracked: 0,
                },
                Vec::new(),
            )
        }
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines();
    let mut summary = GitSummary {
        available: true,
        branch: None,
        upstream: None,
        ahead: None,
        behind: None,
        staged: 0,
        unstaged: 0,
        untracked: 0,
    };
    let mut files = Vec::new();

    if let Some(first) = lines.next() {
        if first.starts_with("## ") {
            let mut header = first.trim_start_matches("## ").trim();
            if let Some((branch, rest)) = header.split_once("...") {
                summary.branch = Some(branch.trim().to_string());
                header = rest.trim();
                if let Some((upstream, meta)) = header.split_once(' ') {
                    summary.upstream = Some(upstream.trim().to_string());
                    parse_ahead_behind(meta, &mut summary);
                } else {
                    summary.upstream = Some(header.trim().to_string());
                }
            } else {
                if let Some((branch, meta)) = header.split_once(' ') {
                    summary.branch = Some(branch.trim().to_string());
                    parse_ahead_behind(meta, &mut summary);
                } else {
                    summary.branch = Some(header.trim().to_string());
                }
            }
        }
    }

    for line in lines {
        if line.starts_with("?? ") {
            summary.untracked += 1;
            if let Some(path) = line.get(3..) {
                files.push(path.trim().to_string());
            }
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        if chars.len() < 3 {
            continue;
        }
        let index_status = chars[0];
        let work_status = chars[1];
        if index_status != ' ' && index_status != '?' {
            summary.staged += 1;
        }
        if work_status != ' ' && work_status != '?' {
            summary.unstaged += 1;
        }
        if let Some(path) = line.get(3..) {
            files.push(parse_path(path.trim()));
        }
    }

    files.sort();
    files.dedup();

    (summary, files)
}

fn parse_ahead_behind(meta: &str, summary: &mut GitSummary) {
    let meta = meta.trim().trim_start_matches('[').trim_end_matches(']');
    let parts = meta.split(',');
    for part in parts {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix("ahead ") {
            summary.ahead = value.parse::<i64>().ok();
        }
        if let Some(value) = trimmed.strip_prefix("behind ") {
            summary.behind = value.parse::<i64>().ok();
        }
    }
}

fn parse_path(path: &str) -> String {
    if let Some((_, new_path)) = path.split_once(" -> ") {
        return new_path.trim().to_string();
    }
    path.to_string()
}

fn top_level_dirs(paths: &[String]) -> Vec<String> {
    let mut dirs: Vec<String> = paths
        .iter()
        .map(|path| path.split('/').next().unwrap_or("").to_string())
        .filter(|segment| !segment.is_empty())
        .collect();
    dirs.sort_by(|a, b| match (a.as_str(), b.as_str()) {
        (".", ".") => Ordering::Equal,
        (".", _) => Ordering::Less,
        (_, ".") => Ordering::Greater,
        _ => a.cmp(b),
    });
    dirs.dedup();
    dirs
}

fn render_checkpoint_markdown(snapshot: &CheckpointSnapshot) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# Checkpoint {}", snapshot.checkpoint_id));
    lines.push(String::new());
    lines.push(format!("Generated: {}", snapshot.generated_at));
    lines.push(format!("Project: {}", snapshot.project_id));
    lines.push(String::new());

    lines.push("## Current Task".to_string());
    if let Some(task) = snapshot.current_task.as_ref() {
        lines.push(format!("- {}", task.line()));
    } else {
        lines.push("- None".to_string());
    }
    lines.push(String::new());

    lines.push("## Ready Tasks".to_string());
    if snapshot.ready.is_empty() {
        lines.push("- None".to_string());
    } else {
        for task in &snapshot.ready {
            lines.push(format!("- {}", task.line()));
        }
    }
    lines.push(String::new());

    lines.push("## Active Leases".to_string());
    if snapshot.leases.is_empty() {
        lines.push("- None".to_string());
    } else {
        for task in &snapshot.leases {
            let owner = task
                .lease
                .as_ref()
                .map(|lease| lease.owner.as_str())
                .unwrap_or("unknown");
            let expires = task
                .lease
                .as_ref()
                .and_then(|lease| lease.expires_at.as_deref())
                .unwrap_or("n/a");
            lines.push(format!("- {} | {} | {}", task.id, owner, expires));
        }
    }
    lines.push(String::new());

    lines.push("## Git Status".to_string());
    if snapshot.git.available {
        if let Some(branch) = snapshot.git.branch.as_deref() {
            lines.push(format!("- Branch: {}", branch));
        }
        if let Some(upstream) = snapshot.git.upstream.as_deref() {
            lines.push(format!("- Upstream: {}", upstream));
        }
        if let (Some(ahead), Some(behind)) = (snapshot.git.ahead, snapshot.git.behind) {
            lines.push(format!("- Ahead/Behind: {}/{}", ahead, behind));
        } else if let Some(ahead) = snapshot.git.ahead {
            lines.push(format!("- Ahead: {}", ahead));
        } else if let Some(behind) = snapshot.git.behind {
            lines.push(format!("- Behind: {}", behind));
        }
        lines.push(format!(
            "- Staged: {}, Unstaged: {}, Untracked: {}",
            snapshot.git.staged, snapshot.git.unstaged, snapshot.git.untracked
        ));
    } else {
        lines.push("- Git status unavailable".to_string());
    }
    lines.push(String::new());

    lines.push("## Changed Files".to_string());
    if snapshot.changed_files.is_empty() {
        lines.push("- None".to_string());
    } else {
        for path in &snapshot.changed_files {
            lines.push(format!("- {}", path));
        }
    }
    lines.push(String::new());

    lines.push("## Top-level Directories".to_string());
    if snapshot.top_level_dirs.is_empty() {
        lines.push("- None".to_string());
    } else {
        for dir in &snapshot.top_level_dirs {
            lines.push(format!("- {}", dir));
        }
    }
    lines.push(String::new());

    lines.push("## Recent Audit Events".to_string());
    if snapshot.audit_events.is_empty() {
        lines.push("- None".to_string());
    } else {
        for event in &snapshot.audit_events {
            let task = event.task_id.as_deref().unwrap_or("-");
            let actor = event.actor.as_deref().unwrap_or("-");
            lines.push(format!(
                "- {} | {} | {} | {}",
                event.timestamp, event.action, task, actor
            ));
        }
    }
    lines.push(String::new());

    lines.join("\n")
}

fn parse_timestamp(value: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Lease, Relationships, Task};
    use std::collections::HashMap;
    use std::process::Command;
    use tempfile::TempDir;

    fn task(
        id: &str,
        title: &str,
        status: &str,
        updated: Option<&str>,
        project: Option<&str>,
        lease: Option<Lease>,
    ) -> Task {
        Task {
            id: id.to_string(),
            uid: None,
            kind: "task".to_string(),
            title: title.to_string(),
            status: status.to_string(),
            priority: "P2".to_string(),
            phase: "Phase1".to_string(),
            dependencies: vec![],
            labels: vec![],
            assignee: vec![],
            relationships: Relationships::default(),
            lease: lease.map(|l| l),
            project: project.map(|p| p.to_string()),
            initiative: None,
            created_date: Some("2026-02-01 10:00".to_string()),
            updated_date: updated.map(|v| v.to_string()),
            extra: HashMap::new(),
            file_path: None,
            body: String::new(),
        }
    }

    fn init_git_repo(dir: &Path) {
        let ok = Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "git init");
        Command::new("git")
            .args(["config", "user.email", "workmesh@example.com"])
            .current_dir(dir)
            .status()
            .expect("git config");
        Command::new("git")
            .args(["config", "user.name", "WorkMesh"])
            .current_dir(dir)
            .status()
            .expect("git config");
        fs::write(dir.join("README.md"), "hi\n").expect("write");
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(dir)
            .status()
            .expect("git add");
        Command::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(dir)
            .status()
            .expect("git commit");
    }

    #[test]
    fn resolve_project_id_prefers_explicit_then_single_task_project_then_docs_then_repo_name() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();
        fs::create_dir_all(repo.join("docs/projects/myproj")).expect("docs");

        let tasks = vec![
            task("task-001", "A", "To Do", None, Some("myproj"), None),
            task("task-002", "B", "To Do", None, Some("myproj"), None),
        ];
        assert_eq!(
            resolve_project_id(repo, &tasks, Some(" explicit ")),
            "explicit"
        );
        assert_eq!(resolve_project_id(repo, &tasks, None), "myproj");

        let tasks_multi = vec![
            task("task-001", "A", "To Do", None, Some("p1"), None),
            task("task-002", "B", "To Do", None, Some("p2"), None),
        ];
        assert_eq!(resolve_project_id(repo, &tasks_multi, None), "myproj");

        // If docs has a single project dir, it's used even without task project fields.
        let tasks_none = vec![task("task-001", "A", "To Do", None, None, None)];
        assert_eq!(resolve_project_id(repo, &tasks_none, None), "myproj");
    }

    #[test]
    fn resolve_checkpoint_path_supports_absolute_relative_and_id_variants() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();
        fs::create_dir_all(repo.join("docs/projects/p/updates")).expect("updates");

        let abs = repo.join("docs/projects/p/updates/checkpoint-aaa.json");
        fs::write(&abs, "{}").expect("write");
        assert_eq!(
            resolve_checkpoint_path(repo, "p", Some(abs.to_string_lossy().as_ref()))
                .unwrap()
                .canonicalize()
                .unwrap(),
            abs.canonicalize().unwrap()
        );

        // Relative path from repo root.
        let rel = "docs/projects/p/updates/checkpoint-bbb.json";
        fs::write(repo.join(rel), "{}").expect("write");
        assert_eq!(
            resolve_checkpoint_path(repo, "p", Some(rel))
                .unwrap()
                .canonicalize()
                .unwrap(),
            repo.join(rel).canonicalize().unwrap()
        );

        // ID expands to checkpoint-<id>.json in updates dir.
        fs::write(
            repo.join("docs/projects/p/updates/checkpoint-ccc.json"),
            "{}",
        )
        .expect("write");
        assert!(resolve_checkpoint_path(repo, "p", Some("ccc"))
            .unwrap()
            .to_string_lossy()
            .ends_with("checkpoint-ccc.json"));

        // Empty is treated as none.
        assert!(resolve_checkpoint_path(repo, "p", Some("  ")).is_none());

        // No id: picks latest by sorted filename.
        fs::write(
            repo.join("docs/projects/p/updates/checkpoint-zzz.json"),
            "{}",
        )
        .expect("write");
        let latest = resolve_checkpoint_path(repo, "p", None).expect("latest");
        assert!(latest.to_string_lossy().ends_with("checkpoint-zzz.json"));
    }

    #[test]
    fn pick_current_task_picks_lowest_id_in_progress() {
        let tasks = vec![
            task("task-010", "A", "In Progress", None, None, None),
            task("task-002", "B", "In Progress", None, None, None),
            task("task-001", "C", "To Do", None, None, None),
        ];
        let current = pick_current_task(&tasks).expect("current");
        assert_eq!(current.id, "task-002");
    }

    #[test]
    fn active_lease_tasks_returns_active_sorted() {
        let lease_active = Lease {
            owner: "agent".to_string(),
            acquired_at: Some("2026-02-01 10:00".to_string()),
            expires_at: Some("2999-01-01 00:00".to_string()),
        };
        let lease_inactive = Lease {
            owner: "".to_string(),
            acquired_at: None,
            expires_at: None,
        };
        let tasks = vec![
            task(
                "task-010",
                "A",
                "To Do",
                None,
                None,
                Some(lease_active.clone()),
            ),
            task("task-002", "B", "To Do", None, None, Some(lease_active)),
            task("task-001", "C", "To Do", None, None, Some(lease_inactive)),
        ];
        let leased = active_lease_tasks(&tasks);
        let ids: Vec<&str> = leased.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["task-002", "task-010"]);
    }

    #[test]
    fn git_status_parses_branch_and_file_counts() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();
        init_git_repo(repo);

        // Create one untracked file.
        fs::write(repo.join("u.txt"), "u\n").expect("write");

        // Create one staged file.
        fs::write(repo.join("s.txt"), "s\n").expect("write");
        Command::new("git")
            .args(["add", "s.txt"])
            .current_dir(repo)
            .status()
            .expect("git add");

        // Create one unstaged change.
        fs::write(repo.join("README.md"), "changed\n").expect("write");

        let (summary, files) = git_status(repo);
        assert!(summary.available);
        assert!(summary.branch.is_some());
        assert_eq!(summary.untracked, 1);
        assert_eq!(summary.staged, 1);
        assert_eq!(summary.unstaged, 1);
        assert!(files.iter().any(|p| p == "u.txt"));
    }

    #[test]
    fn diff_since_checkpoint_tracks_updated_tasks_and_new_changed_files() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();
        init_git_repo(repo);

        // Backlog dir only needed for audit; keep it minimal.
        let backlog = repo.join("workmesh");
        fs::create_dir_all(backlog.join("tasks")).expect("backlog");

        // Current changes: add one new untracked file.
        fs::write(repo.join("new.txt"), "x\n").expect("write");

        let tasks = vec![
            task(
                "task-001",
                "Old",
                "To Do",
                Some("2026-02-01 09:59"),
                None,
                None,
            ),
            task(
                "task-002",
                "Newer",
                "To Do",
                Some("2026-02-01 10:00"),
                None,
                None,
            ),
            task(
                "task-003",
                "Newest",
                "To Do",
                Some("2026-02-01 10:01"),
                None,
                None,
            ),
        ];

        let checkpoint = CheckpointSnapshot {
            checkpoint_id: "x".to_string(),
            generated_at: "2026-02-01 10:00".to_string(),
            project_id: "p".to_string(),
            repo_root: repo.display().to_string(),
            backlog_dir: backlog.display().to_string(),
            current_task: None,
            ready: vec![],
            leases: vec![],
            git: GitSummary {
                available: false,
                branch: None,
                upstream: None,
                ahead: None,
                behind: None,
                staged: 0,
                unstaged: 0,
                untracked: 0,
            },
            changed_files: vec!["README.md".to_string()],
            top_level_dirs: vec![],
            audit_events: vec![],
        };

        let diff = diff_since_checkpoint(repo, &backlog, &tasks, &checkpoint);
        let updated_ids: Vec<&str> = diff.updated_tasks.iter().map(|t| t.id.as_str()).collect();
        // >= checkpoint timestamp includes task-002 and task-003
        assert_eq!(updated_ids, vec!["task-002", "task-003"]);
        assert!(diff.new_files.iter().any(|p| p == "new.txt"));
    }

    #[test]
    fn write_working_set_and_journal_create_files() {
        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path();
        fs::create_dir_all(repo.join("docs/projects/p/updates")).expect("updates");

        let tasks = vec![TaskSummary {
            id: "task-001".to_string(),
            uid: None,
            title: "Do".to_string(),
            status: "To Do".to_string(),
            priority: "P2".to_string(),
            phase: "Phase1".to_string(),
            project: None,
            initiative: None,
            lease: None,
        }];

        let path = write_working_set(repo, "p", &tasks, Some("note")).expect("working set");
        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("# Working Set"));
        assert!(content.contains("## Notes"));

        let j = append_session_journal(repo, "p", Some("task-001"), Some("next"), Some("note"))
            .expect("journal");
        let content = fs::read_to_string(&j).expect("read");
        assert!(content.contains("# Session Journal"));
        assert!(content.contains("Task: task-001"));
    }

    #[test]
    fn render_resume_and_diff_have_stable_defaults() {
        let snapshot = CheckpointSnapshot {
            checkpoint_id: "x".to_string(),
            generated_at: "2026-02-01 10:00".to_string(),
            project_id: "p".to_string(),
            repo_root: "/repo".to_string(),
            backlog_dir: "/repo/workmesh".to_string(),
            current_task: None,
            ready: vec![],
            leases: vec![],
            git: GitSummary {
                available: false,
                branch: None,
                upstream: None,
                ahead: None,
                behind: None,
                staged: 0,
                unstaged: 0,
                untracked: 0,
            },
            changed_files: vec![],
            top_level_dirs: vec![],
            audit_events: vec![],
        };
        let summary = ResumeSummary {
            snapshot: snapshot.clone(),
            working_set: Some("- x\n".to_string()),
            checkpoint_path: PathBuf::from("checkpoint.json"),
        };
        let rendered = render_resume(&summary);
        assert!(rendered.contains("Resume from checkpoint x"));
        assert!(rendered.contains("Current task:"));
        assert!(rendered.contains("Working set:"));

        let diff = DiffReport {
            checkpoint_id: "x".to_string(),
            checkpoint_time: "t".to_string(),
            updated_tasks: vec![],
            new_files: vec![],
            audit_events: vec![],
        };
        let rendered_diff = render_diff(&diff);
        assert!(rendered_diff.contains("Updated tasks:"));
        assert!(rendered_diff.contains("- None"));
    }

    #[test]
    fn parse_timestamp_parses_expected_format() {
        assert!(parse_timestamp("2026-02-01 10:00").is_some());
        assert!(parse_timestamp("not-a-time").is_none());
    }
}
