use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::context::{ContextScopeMode, ContextState};
use crate::focus::FocusState;
use crate::task::Task;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoardBy {
    Status,
    Phase,
    Priority,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoardLane {
    pub key: String,
    pub tasks: Vec<String>,
}

fn canonical_status_name(status: &str) -> Option<&'static str> {
    let lc = status.trim().to_lowercase();
    match lc.as_str() {
        "to do" => Some("To Do"),
        "in progress" => Some("In Progress"),
        "done" => Some("Done"),
        _ => None,
    }
}

fn stable_task_sort_key(task: &Task) -> (i32, String) {
    (task.id_num(), task.id.to_lowercase())
}

/// Group tasks into lanes for a simple "board" view.
///
/// Returns a stable, deterministic lane order and a stable task order within each lane.
pub fn board_lanes<'a>(
    tasks: &'a [Task],
    by: BoardBy,
    scope_ids: Option<&HashSet<String>>,
) -> Vec<(String, Vec<&'a Task>)> {
    let mut lanes: HashMap<String, (String, Vec<&Task>)> = HashMap::new();
    let mut first_seen: Vec<String> = Vec::new();

    for task in tasks {
        if let Some(scope) = scope_ids {
            if !scope.contains(&task.id.to_lowercase()) {
                continue;
            }
        }

        let raw_key = match by {
            BoardBy::Status => task.status.trim(),
            BoardBy::Phase => task.phase.trim(),
            BoardBy::Priority => task.priority.trim(),
        };
        let display = if by == BoardBy::Status {
            canonical_status_name(raw_key)
                .unwrap_or_else(|| {
                    let trimmed = raw_key.trim();
                    if trimmed.is_empty() {
                        "(none)"
                    } else {
                        trimmed
                    }
                })
                .to_string()
        } else {
            let trimmed = raw_key.trim();
            if trimmed.is_empty() {
                "(none)".to_string()
            } else {
                trimmed.to_string()
            }
        };

        let k = display.to_lowercase();
        if !lanes.contains_key(&k) {
            first_seen.push(k.clone());
            lanes.insert(k.clone(), (display, Vec::new()));
        }
        lanes.get_mut(&k).expect("lane").1.push(task);
    }

    let mut out: Vec<(String, Vec<&Task>)> = Vec::new();

    if by == BoardBy::Status {
        let mut used = HashSet::new();
        for name in ["to do", "in progress", "done"] {
            if let Some((display, mut lane_tasks)) = lanes.remove(name) {
                lane_tasks.sort_by_key(|t| stable_task_sort_key(t));
                out.push((display, lane_tasks));
                used.insert(name.to_string());
            }
        }
        // Remaining lanes in deterministic order.
        let mut remaining: Vec<(String, (String, Vec<&Task>))> = lanes.into_iter().collect();
        remaining.sort_by_key(|(k, _)| k.to_string());
        for (_, (display, mut lane_tasks)) in remaining {
            lane_tasks.sort_by_key(|t| stable_task_sort_key(t));
            out.push((display, lane_tasks));
        }
        return out;
    }

    // For non-status boards: deterministic lane order by key.
    let mut ordered: BTreeMap<String, (String, Vec<&Task>)> = BTreeMap::new();
    for (k, v) in lanes {
        ordered.insert(k, v);
    }
    for (_, (display, mut lane_tasks)) in ordered {
        lane_tasks.sort_by_key(|t| stable_task_sort_key(t));
        out.push((display, lane_tasks));
    }
    out
}

fn is_done(task: &Task) -> bool {
    task.status.trim().eq_ignore_ascii_case("done")
}

fn all_blocker_refs(task: &Task) -> Vec<String> {
    let mut refs = Vec::new();
    refs.extend(task.dependencies.iter().cloned());
    refs.extend(task.relationships.blocked_by.iter().cloned());
    refs
}

fn scope_ids_for_epic(tasks: &[Task], epic_id: &str) -> HashSet<String> {
    let epic_lc = epic_id.trim().to_lowercase();
    let mut included: HashSet<String> = HashSet::new();
    included.insert(epic_lc.clone());

    // Expand by parent relationships (transitively).
    let mut changed = true;
    while changed {
        changed = false;
        for task in tasks {
            let task_lc = task.id.to_lowercase();
            if included.contains(&task_lc) {
                continue;
            }
            let has_parent = task
                .relationships
                .parent
                .iter()
                .any(|p| included.contains(&p.to_lowercase()));
            if has_parent {
                included.insert(task_lc);
                changed = true;
            }
        }
    }
    included
}

/// Best-effort scope IDs from focus state.
///
/// Prefer epic subtree when `focus.epic_id` is set. Otherwise, if `focus.working_set` has ids,
/// scope to those ids.
pub fn scope_ids_from_focus(tasks: &[Task], focus: &FocusState) -> Option<HashSet<String>> {
    if let Some(epic) = focus
        .epic_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return Some(scope_ids_for_epic(tasks, epic));
    }
    if !focus.working_set.is_empty() {
        let mut ids = HashSet::new();
        for id in focus.working_set.iter() {
            let trimmed = id.trim();
            if trimmed.is_empty() {
                continue;
            }
            ids.insert(trimmed.to_lowercase());
        }
        if !ids.is_empty() {
            return Some(ids);
        }
    }
    None
}

pub fn scope_ids_from_context(tasks: &[Task], context: &ContextState) -> Option<HashSet<String>> {
    match context.scope.mode {
        ContextScopeMode::Epic => context
            .scope
            .epic_id
            .as_deref()
            .map(|id| id.trim())
            .filter(|id| !id.is_empty())
            .map(|id| scope_ids_for_epic(tasks, id)),
        ContextScopeMode::Tasks => {
            if context.scope.task_ids.is_empty() {
                return None;
            }
            let mut ids = HashSet::new();
            for id in context.scope.task_ids.iter() {
                let trimmed = id.trim();
                if trimmed.is_empty() {
                    continue;
                }
                ids.insert(trimmed.to_lowercase());
            }
            if ids.is_empty() {
                None
            } else {
                Some(ids)
            }
        }
        ContextScopeMode::None => None,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BlockedTaskEntry {
    pub id: String,
    pub title: String,
    pub status: String,
    pub blockers: Vec<String>,
    pub missing_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopBlockerEntry {
    pub id: String,
    pub blocked_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlockersReport {
    pub scope: serde_json::Value,
    pub blocked_tasks: Vec<BlockedTaskEntry>,
    pub top_blockers: Vec<TopBlockerEntry>,
    pub warnings: Vec<String>,
}

/// Compute a "blockers" report.
///
/// Scope rules:
/// - If `epic_id` is provided, scope to that epic + descendants via relationships.parent.
/// - Else if focus has `epic_id`, scope to that epic subtree.
/// - Otherwise scope to all tasks.
pub fn blockers_report(
    tasks: &[Task],
    focus: Option<&FocusState>,
    epic_id: Option<&str>,
) -> BlockersReport {
    let context = focus.map(|f| ContextState {
        version: 1,
        project_id: f.project_id.clone(),
        objective: f.objective.clone(),
        scope: crate::context::ContextScope {
            mode: if f
                .epic_id
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                ContextScopeMode::Epic
            } else if !f.working_set.is_empty() {
                ContextScopeMode::Tasks
            } else {
                ContextScopeMode::None
            },
            epic_id: f.epic_id.clone(),
            task_ids: f.working_set.clone(),
        },
        updated_at: f.updated_at.clone(),
    });
    blockers_report_with_context(tasks, context.as_ref(), epic_id)
}

pub fn blockers_report_with_context(
    tasks: &[Task],
    context: Option<&ContextState>,
    epic_id: Option<&str>,
) -> BlockersReport {
    let mut warnings = Vec::new();
    let chosen_epic = epic_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            context
                .filter(|c| c.scope.mode == ContextScopeMode::Epic)
                .and_then(|c| c.scope.epic_id.clone())
        });

    let scope_ids = if let Some(epic) = chosen_epic.as_deref() {
        Some(scope_ids_for_epic(tasks, epic))
    } else {
        context.and_then(|c| scope_ids_from_context(tasks, c))
    };

    if let Some(epic) = chosen_epic.as_deref() {
        let exists = tasks.iter().any(|t| t.id.eq_ignore_ascii_case(epic));
        if !exists {
            warnings.push(format!("epic not found: {}", epic));
        }
    }

    let done_ids: HashSet<String> = tasks
        .iter()
        .filter(|t| is_done(t))
        .map(|t| t.id.to_lowercase())
        .collect();
    let by_id: HashMap<String, &Task> = tasks.iter().map(|t| (t.id.to_lowercase(), t)).collect();

    let mut blocked_tasks = Vec::new();
    let mut blocker_counts: HashMap<String, usize> = HashMap::new();

    for task in tasks {
        if let Some(scope) = scope_ids.as_ref() {
            if !scope.contains(&task.id.to_lowercase()) {
                continue;
            }
        }
        if is_done(task) {
            continue;
        }
        let mut blockers = Vec::new();
        let mut missing = Vec::new();
        let mut seen_refs: HashSet<String> = HashSet::new();
        for raw in all_blocker_refs(task) {
            let id = raw.trim();
            if id.is_empty() {
                continue;
            }
            let lc = id.to_lowercase();
            if seen_refs.contains(&lc) {
                continue;
            }
            seen_refs.insert(lc.clone());
            let Some(dep) = by_id.get(&lc) else {
                missing.push(id.to_string());
                continue;
            };
            if !done_ids.contains(&lc) {
                blockers.push(dep.id.clone());
                *blocker_counts.entry(dep.id.clone()).or_insert(0) += 1;
            }
        }
        blockers.sort_by_key(|id| {
            tasks
                .iter()
                .find(|t| t.id.eq_ignore_ascii_case(id))
                .map(|t| stable_task_sort_key(t))
                .unwrap_or((999_999, id.to_lowercase()))
        });
        missing.sort();
        if blockers.is_empty() && missing.is_empty() {
            continue;
        }
        blocked_tasks.push(BlockedTaskEntry {
            id: task.id.clone(),
            title: task.title.clone(),
            status: task.status.clone(),
            blockers,
            missing_refs: missing,
        });
    }

    blocked_tasks.sort_by_key(|entry| {
        tasks
            .iter()
            .find(|t| t.id.eq_ignore_ascii_case(&entry.id))
            .map(|t| stable_task_sort_key(t))
            .unwrap_or((999_999, entry.id.to_lowercase()))
    });

    let mut top_blockers: Vec<TopBlockerEntry> = blocker_counts
        .into_iter()
        .map(|(id, count)| TopBlockerEntry {
            id,
            blocked_count: count,
        })
        .collect();
    top_blockers.sort_by_key(|b| (-(b.blocked_count as i64), b.id.to_lowercase()));

    let scope = if let Some(epic) = chosen_epic.as_deref() {
        serde_json::json!({"type": "epic", "epic_id": epic})
    } else if let Some(ctx) = context {
        match ctx.scope.mode {
            ContextScopeMode::Tasks => {
                serde_json::json!({"type": "tasks", "task_ids": ctx.scope.task_ids})
            }
            ContextScopeMode::Epic => {
                serde_json::json!({"type": "epic", "epic_id": ctx.scope.epic_id})
            }
            ContextScopeMode::None => serde_json::json!({"type": "repo"}),
        }
    } else {
        serde_json::json!({"type": "repo"})
    };

    BlockersReport {
        scope,
        blocked_tasks,
        top_blockers,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Relationships;

    fn t(id: &str, title: &str, status: &str, deps: &[&str], parents: &[&str]) -> Task {
        Task {
            id: id.to_string(),
            uid: None,
            kind: "task".to_string(),
            title: title.to_string(),
            status: status.to_string(),
            priority: "P2".to_string(),
            phase: "Phase1".to_string(),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
            labels: vec![],
            assignee: vec![],
            relationships: Relationships {
                blocked_by: vec![],
                parent: parents.iter().map(|s| s.to_string()).collect(),
                child: vec![],
                discovered_from: vec![],
            },
            lease: None,
            project: None,
            initiative: None,
            created_date: None,
            updated_date: None,
            extra: Default::default(),
            file_path: None,
            body: String::new(),
        }
    }

    #[test]
    fn board_groups_by_status_in_canonical_lane_order() {
        let tasks = vec![
            t("task-001", "A", "To Do", &[], &[]),
            t("task-002", "B", "In Progress", &[], &[]),
            t("task-003", "C", "Done", &[], &[]),
            t("task-004", "D", "Blocked", &[], &[]),
        ];
        let lanes = board_lanes(&tasks, BoardBy::Status, None);
        let keys: Vec<String> = lanes.iter().map(|(k, _lane)| k.clone()).collect();
        assert_eq!(keys[0], "To Do");
        assert_eq!(keys[1], "In Progress");
        assert_eq!(keys[2], "Done");
        assert_eq!(keys[3], "Blocked");
    }

    #[test]
    fn blockers_report_scopes_to_epic_subtree() {
        let mut tasks = vec![
            t("task-100", "Epic", "In Progress", &[], &[]),
            t("task-101", "Child", "To Do", &["task-102"], &["task-100"]),
            t("task-102", "Blocker", "To Do", &[], &["task-100"]),
            t("task-200", "Other", "To Do", &["task-102"], &[]),
        ];
        tasks[1].relationships.blocked_by = vec!["task-102".to_string()];
        let report = blockers_report(&tasks, None, Some("task-100"));
        assert_eq!(report.scope["type"].as_str(), Some("epic"));
        assert_eq!(report.blocked_tasks.len(), 1);
        assert_eq!(report.blocked_tasks[0].id, "task-101");
        assert_eq!(report.top_blockers[0].id, "task-102");
        assert_eq!(report.top_blockers[0].blocked_count, 1);
    }

    #[test]
    fn board_lanes_phase_scope_and_blank_bucket() {
        let tasks = vec![
            t("task-001", "A", "To Do", &[], &[]),
            t("task-002", "B", "To Do", &[], &[]),
            t("task-003", "C", "To Do", &[], &[]),
        ];
        let mut tasks = tasks;
        tasks[0].phase = "Phase2".to_string();
        tasks[1].phase = "".to_string();
        tasks[2].phase = "Phase1".to_string();

        let scope = HashSet::from(["task-001".to_string(), "task-002".to_string()]);
        let lanes = board_lanes(&tasks, BoardBy::Phase, Some(&scope));
        let keys: Vec<String> = lanes.iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(keys, vec!["(none)".to_string(), "Phase2".to_string()]);
    }

    #[test]
    fn scope_ids_from_focus_prefers_epic_then_working_set() {
        let tasks = vec![
            t("task-100", "Epic", "To Do", &[], &[]),
            t("task-101", "Child", "To Do", &[], &["task-100"]),
            t("task-200", "Other", "To Do", &[], &[]),
        ];

        let focus_with_epic = FocusState {
            project_id: None,
            epic_id: Some("task-100".to_string()),
            objective: None,
            working_set: vec!["task-200".to_string()],
            updated_at: None,
        };
        let scoped = scope_ids_from_focus(&tasks, &focus_with_epic).expect("scope");
        assert!(scoped.contains("task-100"));
        assert!(scoped.contains("task-101"));
        assert!(!scoped.contains("task-200"));

        let focus_with_working_set = FocusState {
            project_id: None,
            epic_id: None,
            objective: None,
            working_set: vec!["task-200".to_string(), " ".to_string()],
            updated_at: None,
        };
        let scoped = scope_ids_from_focus(&tasks, &focus_with_working_set).expect("scope");
        assert_eq!(scoped, HashSet::from(["task-200".to_string()]));
    }

    #[test]
    fn blockers_report_warns_when_epic_missing_and_tracks_missing_refs() {
        let tasks = vec![
            t("task-001", "A", "To Do", &["task-missing-999"], &[]),
            t("task-002", "B", "Done", &[], &[]),
        ];
        let report = blockers_report(&tasks, None, Some("task-epic-missing"));
        assert_eq!(report.scope["type"].as_str(), Some("epic"));
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("epic not found: task-epic-missing")));
        assert_eq!(report.blocked_tasks.len(), 0);
    }

    #[test]
    fn blockers_report_tracks_missing_refs_in_repo_scope() {
        let tasks = vec![
            t("task-001", "A", "To Do", &["task-missing-999"], &[]),
            t("task-002", "B", "Done", &[], &[]),
        ];
        let report = blockers_report(&tasks, None, None);
        assert_eq!(report.scope["type"].as_str(), Some("repo"));
        assert_eq!(report.blocked_tasks.len(), 1);
        assert_eq!(report.blocked_tasks[0].id, "task-001");
        assert_eq!(
            report.blocked_tasks[0].missing_refs,
            vec!["task-missing-999".to_string()]
        );
    }
}
