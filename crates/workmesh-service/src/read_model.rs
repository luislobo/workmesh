use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use chrono::Local;
use workmesh_core::global_sessions::{load_sessions_latest, read_current_session_id, AgentSession};
use workmesh_core::workstreams::{load_workstream_registry, WorkstreamRecord};
use workmesh_core::worktrees::{list_worktree_views, load_worktree_registry, WorktreeRecord};

use crate::model::{
    RepoView, ServiceSnapshot, SessionView, SummaryResponse, WorkstreamContextView, WorkstreamView,
    WorktreeView,
};

pub fn collect_snapshot(home: &Path, scan_roots: &[PathBuf]) -> ServiceSnapshot {
    let generated_at = Local::now().to_rfc3339();
    let mut warnings = Vec::new();

    let sessions = match load_sessions_latest(home) {
        Ok(value) => value,
        Err(err) => {
            warnings.push(format!("sessions load failed: {}", err));
            Vec::new()
        }
    };

    let workstream_registry = match load_workstream_registry(home) {
        Ok(value) => value,
        Err(err) => {
            warnings.push(format!("workstream registry load failed: {}", err));
            Default::default()
        }
    };

    let worktree_registry = match load_worktree_registry(home) {
        Ok(value) => value,
        Err(err) => {
            warnings.push(format!("worktree registry load failed: {}", err));
            Default::default()
        }
    };

    let mut repo_roots = collect_repo_roots(
        &sessions,
        &workstream_registry.workstreams,
        &worktree_registry.worktrees,
        scan_roots,
    );

    // Keep deterministic ordering.
    let mut repo_roots_vec: Vec<String> = repo_roots.iter().cloned().collect();
    repo_roots_vec.sort_by_key(|value| value.to_ascii_lowercase());

    let mut registry_by_path: BTreeMap<String, WorktreeRecord> = BTreeMap::new();
    for record in worktree_registry.worktrees {
        registry_by_path.insert(record.path.to_ascii_lowercase(), record);
    }

    let mut worktree_views: Vec<WorktreeView> = Vec::new();
    let mut seen_worktree_paths: BTreeSet<String> = BTreeSet::new();

    for repo_root in &repo_roots_vec {
        let repo_path = PathBuf::from(repo_root);
        match list_worktree_views(&repo_path, home) {
            Ok(entries) => {
                for entry in entries {
                    let key = entry.path.to_ascii_lowercase();
                    seen_worktree_paths.insert(key.clone());
                    let registry = registry_by_path.get(&key);
                    worktree_views.push(WorktreeView {
                        id: entry
                            .id
                            .clone()
                            .or_else(|| registry.map(|record| record.id.clone())),
                        path: entry.path,
                        repo_root: entry
                            .repo_root
                            .clone()
                            .or_else(|| registry.map(|record| record.repo_root.clone())),
                        branch: entry
                            .branch
                            .clone()
                            .or_else(|| registry.and_then(|record| record.branch.clone())),
                        attached_session_id: registry.and_then(|record| {
                            record
                                .attached_session_id
                                .as_ref()
                                .map(|value| value.to_string())
                        }),
                        in_git: entry.in_git,
                        exists: entry.exists,
                        issues: entry.issues,
                    });
                }
            }
            Err(err) => {
                warnings.push(format!(
                    "worktree view load failed for {}: {}",
                    repo_root, err
                ));
            }
        }
    }

    // Include registry entries even when git worktree listing failed.
    for record in registry_by_path.values() {
        let key = record.path.to_ascii_lowercase();
        if seen_worktree_paths.contains(&key) {
            continue;
        }
        let exists = Path::new(&record.path).exists();
        let mut issues = Vec::new();
        if !exists {
            issues.push("path_missing".to_string());
        }
        issues.push("not_in_git_worktree_list".to_string());
        worktree_views.push(WorktreeView {
            id: Some(record.id.clone()),
            path: record.path.clone(),
            repo_root: Some(record.repo_root.clone()),
            branch: record.branch.clone(),
            attached_session_id: record.attached_session_id.clone(),
            in_git: false,
            exists,
            issues,
        });
        repo_roots.insert(record.repo_root.clone());
    }

    worktree_views.sort_by_key(|value| value.path.to_ascii_lowercase());

    let mut workstream_by_session_id: BTreeMap<String, String> = BTreeMap::new();
    let mut workstream_by_worktree_path: BTreeMap<String, String> = BTreeMap::new();
    let mut workstream_views = Vec::new();

    for record in workstream_registry.workstreams {
        if let Some(session_id) = record
            .session_id
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            workstream_by_session_id.insert(session_id.to_string(), record.id.clone());
        }
        if let Some(path) = record
            .worktree
            .as_ref()
            .map(|binding| normalize_path_string(Path::new(&binding.path)))
        {
            workstream_by_worktree_path.insert(path.to_ascii_lowercase(), record.id.clone());
        }

        workstream_views.push(workstream_to_view(&record));
        repo_roots.insert(record.repo_root.clone());
    }

    workstream_views.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut session_views: Vec<SessionView> = sessions
        .into_iter()
        .map(|session| {
            let worktree_path = session
                .worktree
                .as_ref()
                .map(|binding| binding.path.clone());
            let linked_workstream =
                workstream_by_session_id
                    .get(&session.id)
                    .cloned()
                    .or_else(|| {
                        worktree_path.as_ref().and_then(|path| {
                            let key = normalize_path_string(Path::new(path));
                            workstream_by_worktree_path
                                .get(&key.to_ascii_lowercase())
                                .cloned()
                        })
                    });

            SessionView {
                id: session.id,
                updated_at: session.updated_at,
                objective: optional_string(Some(&session.objective)),
                cwd: session.cwd,
                repo_root: session.repo_root.clone(),
                worktree_path,
                workstream_id: linked_workstream,
                working_set: session.working_set,
                truth_refs: session.truth_refs,
            }
        })
        .collect();

    session_views.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut repo_views: Vec<RepoView> = repo_roots
        .into_iter()
        .map(|repo_root| {
            repo_to_view(
                &repo_root,
                &session_views,
                &workstream_views,
                &worktree_views,
            )
        })
        .collect();

    repo_views.sort_by_key(|repo| repo.repo_root.to_ascii_lowercase());

    let summary = SummaryResponse {
        generated_at: generated_at.clone(),
        active_sessions: session_views.len(),
        active_workstreams: workstream_views
            .iter()
            .filter(|stream| stream.status.eq_ignore_ascii_case("active"))
            .count(),
        open_worktrees: worktree_views
            .iter()
            .filter(|worktree| worktree.exists)
            .count(),
        repos_tracked: repo_views.len(),
        warnings: warnings.clone(),
    };

    ServiceSnapshot {
        generated_at,
        summary,
        current_session_id: read_current_session_id(home),
        sessions: session_views,
        workstreams: workstream_views,
        worktrees: worktree_views,
        repos: repo_views,
        warnings,
    }
}

fn repo_to_view(
    repo_root: &str,
    sessions: &[SessionView],
    workstreams: &[WorkstreamView],
    worktrees: &[WorktreeView],
) -> RepoView {
    let repo_root_lc = repo_root.to_ascii_lowercase();

    let matching_workstreams: Vec<&WorkstreamView> = workstreams
        .iter()
        .filter(|stream| stream.repo_root.to_ascii_lowercase() == repo_root_lc)
        .collect();

    let matching_sessions: Vec<&SessionView> = sessions
        .iter()
        .filter(|session| {
            session
                .repo_root
                .as_deref()
                .map(|value| value.to_ascii_lowercase() == repo_root_lc)
                .unwrap_or(false)
        })
        .collect();

    let matching_worktrees: Vec<&WorktreeView> = worktrees
        .iter()
        .filter(|worktree| {
            worktree
                .repo_root
                .as_deref()
                .map(|value| value.to_ascii_lowercase() == repo_root_lc)
                .unwrap_or(false)
        })
        .collect();

    let mut latest = None::<String>;
    for stream in &matching_workstreams {
        if latest
            .as_ref()
            .map(|value| value < &stream.updated_at)
            .unwrap_or(true)
        {
            latest = Some(stream.updated_at.clone());
        }
    }
    for session in &matching_sessions {
        if latest
            .as_ref()
            .map(|value| value < &session.updated_at)
            .unwrap_or(true)
        {
            latest = Some(session.updated_at.clone());
        }
    }

    RepoView {
        repo_root: repo_root.to_string(),
        workstream_count: matching_workstreams.len(),
        active_workstream_count: matching_workstreams
            .iter()
            .filter(|stream| stream.status.eq_ignore_ascii_case("active"))
            .count(),
        worktree_count: matching_worktrees.len(),
        session_count: matching_sessions.len(),
        last_activity_at: latest,
    }
}

fn workstream_to_view(record: &WorkstreamRecord) -> WorkstreamView {
    let context = record.context.as_ref().map(|value| WorkstreamContextView {
        project_id: value.project_id.clone(),
        objective: value.objective.clone(),
        scope_mode: Some(
            match value.scope.mode {
                workmesh_core::context::ContextScopeMode::None => "none",
                workmesh_core::context::ContextScopeMode::Epic => "epic",
                workmesh_core::context::ContextScopeMode::Tasks => "tasks",
            }
            .to_string(),
        ),
        epic_id: value.scope.epic_id.clone(),
        task_ids: value.scope.task_ids.clone(),
    });

    WorkstreamView {
        id: record.id.clone(),
        key: optional_string(record.key.as_deref()),
        name: record.name.clone(),
        status: record.status.as_str().to_string(),
        repo_root: record.repo_root.clone(),
        worktree_path: record.worktree.as_ref().map(|binding| binding.path.clone()),
        branch: record
            .worktree
            .as_ref()
            .and_then(|binding| binding.branch.clone()),
        session_id: optional_string(record.session_id.as_deref()),
        context,
        truth_refs: record.truth_refs.clone(),
        updated_at: record.updated_at.clone(),
    }
}

fn collect_repo_roots(
    sessions: &[AgentSession],
    workstreams: &[WorkstreamRecord],
    worktrees: &[WorktreeRecord],
    scan_roots: &[PathBuf],
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();

    for session in sessions {
        if let Some(value) = optional_string(session.repo_root.as_deref()) {
            out.insert(normalize_path_string(Path::new(&value)));
        }
    }

    for stream in workstreams {
        if let Some(value) = optional_string(Some(&stream.repo_root)) {
            out.insert(normalize_path_string(Path::new(&value)));
        }
    }

    for worktree in worktrees {
        if let Some(value) = optional_string(Some(&worktree.repo_root)) {
            out.insert(normalize_path_string(Path::new(&value)));
        }
    }

    for root in scan_roots {
        if root.exists() {
            out.insert(normalize_path_string(root));
        }
    }

    out
}

fn optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn normalize_path_string(path: &Path) -> String {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    absolute
        .canonicalize()
        .unwrap_or(absolute)
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use workmesh_core::global_sessions::{append_session_saved, AgentSession, WorktreeBinding};
    use workmesh_core::workstreams::{
        upsert_workstream_record, WorkstreamRecord, WorkstreamStatus,
    };
    use workmesh_core::worktrees::{upsert_worktree_record, WorktreeRecord};

    #[test]
    fn collect_snapshot_links_session_workstream_and_worktree() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let home = temp.path();

        let repo_root = home.join("repo");
        let wt_path = repo_root.join("wt-a");
        std::fs::create_dir_all(&wt_path).expect("worktree path");

        let worktree = upsert_worktree_record(
            home,
            WorktreeRecord {
                id: String::new(),
                repo_root: repo_root.to_string_lossy().to_string(),
                path: wt_path.to_string_lossy().to_string(),
                branch: Some("ws/a".to_string()),
                created_at: String::new(),
                updated_at: String::new(),
                attached_session_id: Some("session-1".to_string()),
            },
        )
        .expect("insert worktree");

        let stream = upsert_workstream_record(
            home,
            WorkstreamRecord {
                id: String::new(),
                repo_root: repo_root.to_string_lossy().to_string(),
                key: Some("a".to_string()),
                name: "A".to_string(),
                status: WorkstreamStatus::Active,
                created_at: String::new(),
                updated_at: String::new(),
                worktree: Some(WorktreeBinding {
                    id: Some(worktree.id.clone()),
                    path: wt_path.to_string_lossy().to_string(),
                    branch: Some("ws/a".to_string()),
                    repo_root: Some(repo_root.to_string_lossy().to_string()),
                }),
                session_id: Some("session-1".to_string()),
                context: None,
                truth_refs: vec![],
                notes: None,
            },
        )
        .expect("insert stream");

        append_session_saved(
            home,
            AgentSession {
                id: "session-1".to_string(),
                created_at: "2026-02-18T00:00:00Z".to_string(),
                updated_at: "2026-02-18T00:00:01Z".to_string(),
                cwd: wt_path.to_string_lossy().to_string(),
                repo_root: Some(repo_root.to_string_lossy().to_string()),
                project_id: Some("demo".to_string()),
                epic_id: None,
                objective: "ship".to_string(),
                working_set: vec!["task-001".to_string()],
                notes: None,
                git: None,
                checkpoint: None,
                recent_changes: None,
                handoff: None,
                worktree: Some(WorktreeBinding {
                    id: Some(worktree.id.clone()),
                    path: wt_path.to_string_lossy().to_string(),
                    branch: Some("ws/a".to_string()),
                    repo_root: Some(repo_root.to_string_lossy().to_string()),
                }),
                truth_refs: vec![],
            },
        )
        .expect("append session");

        let snapshot = collect_snapshot(home, &[repo_root.clone()]);
        assert_eq!(snapshot.summary.active_workstreams, 1);
        assert_eq!(snapshot.summary.active_sessions, 1);
        assert_eq!(snapshot.workstreams.len(), 1);
        assert_eq!(snapshot.sessions.len(), 1);
        assert_eq!(snapshot.workstreams[0].id, stream.id);
        assert_eq!(
            snapshot.sessions[0].workstream_id.as_deref(),
            Some(stream.id.as_str())
        );
    }
}
