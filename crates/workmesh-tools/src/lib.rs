use std::path::{Path, PathBuf};

use serde_json::Value;
use workmesh_core::backlog::{
    locate_backlog_dir, resolve_backlog, resolve_backlog_dir, BacklogError,
};
use workmesh_core::project::repo_root_from_backlog;

pub const ROOT_REQUIRED_ERROR: &str =
    "root is required for MCP calls unless the server is started within a repo containing tasks/ or backlog/tasks";

pub fn default_verbose() -> bool {
    false
}

pub fn maybe_verbose_value(verbose: bool, minimal: Value, detailed: Value) -> Value {
    if verbose {
        detailed
    } else {
        minimal
    }
}

pub fn bulk_summary(updated: &[String], failed: &[String]) -> Value {
    serde_json::json!({
        "ok": failed.is_empty(),
        "updated_count": updated.len(),
        "failed_count": failed.len(),
        "failed_ids": failed,
    })
}

pub fn recommended_kinds() -> &'static [&'static str] {
    &[
        "epic", "story", "task", "bug", "subtask", "incident", "spike",
    ]
}

pub fn best_practice_hints() -> &'static [&'static str] {
    &[
        "Check config_show first if you are not sure which task-quality fields this repo requires.",
        "By default, actionable tasks use Description, Acceptance Criteria, and Definition of Done.",
        "By default, Definition of Done should include outcome-based criteria, not only hygiene checks.",
        "Actionable and Done transitions are gated by the repo's configured task quality requirements.",
        "Always record dependencies for tasks that are blocked by other work.",
        "Use dependencies to power next-task selection and blocked/ready views.",
        "If unsure, start with an empty list and add dependencies as soon as blockers appear.",
        "Prefer specific task ids (e.g., task-042) over vague references.",
        "Update dependencies when status changes to avoid stale blocked tasks.",
        "Do not commit derived artifacts like `.workmesh/.index/` or `.workmesh/.audit.log` (they are rebuildable).",
    ]
}

pub fn tool_catalog() -> Vec<Value> {
    vec![
        serde_json::json!({"name": "version", "summary": "Return WorkMesh version information."}),
        serde_json::json!({"name": "readme", "summary": "Return README.json (agent-friendly repo docs)."}),
        serde_json::json!({"name": "doctor", "summary": "Diagnostics report for repo layout, context, index, skills, and versions."}),
        serde_json::json!({"name": "bootstrap", "summary": "Bootstrap WorkMesh by detecting repo state and applying setup/migration."}),
        serde_json::json!({"name": "config_show", "summary": "Show project/global config and effective defaults."}),
        serde_json::json!({"name": "config_set", "summary": "Set a WorkMesh config key in project or global scope."}),
        serde_json::json!({"name": "config_unset", "summary": "Unset a WorkMesh config key (remove it from the selected config file)."}),
        serde_json::json!({"name": "context_show", "summary": "Show repo-local context (project/objective/scope)."}),
        serde_json::json!({"name": "context_set", "summary": "Set repo-local context (project/objective/scope)."}),
        serde_json::json!({"name": "context_clear", "summary": "Clear repo-local context."}),
        serde_json::json!({"name": "workstream_list", "summary": "List workstreams for the current repo."}),
        serde_json::json!({"name": "workstream_create", "summary": "Create a new workstream (optionally create a worktree)."}),
        serde_json::json!({"name": "workstream_show", "summary": "Show one workstream (defaults to active stream in this worktree)."}),
        serde_json::json!({"name": "workstream_switch", "summary": "Switch active workstream for this worktree."}),
        serde_json::json!({"name": "workstream_pause", "summary": "Pause a workstream (intentionally inactive)."}),
        serde_json::json!({"name": "workstream_close", "summary": "Close a workstream (completed or abandoned)."}),
        serde_json::json!({"name": "workstream_reopen", "summary": "Reopen a paused/closed workstream (marks it active)."}),
        serde_json::json!({"name": "workstream_rename", "summary": "Rename a workstream."}),
        serde_json::json!({"name": "workstream_set", "summary": "Update workstream fields (key, notes, context snapshot)."}),
        serde_json::json!({"name": "workstream_doctor", "summary": "Diagnose workstream registry health for this repo."}),
        serde_json::json!({"name": "workstream_restore", "summary": "Build a deterministic restore plan for active workstreams (after reboot / lost terminals)."}),
        serde_json::json!({"name": "worktree_list", "summary": "List worktrees (git + registry)."}),
        serde_json::json!({"name": "worktree_create", "summary": "Create a git worktree and register it."}),
        serde_json::json!({"name": "worktree_adopt_clone", "summary": "Convert a standalone clone into a git worktree under this repo."}),
        serde_json::json!({"name": "worktree_attach", "summary": "Attach current/specified session to a worktree."}),
        serde_json::json!({"name": "worktree_detach", "summary": "Detach worktree from current/specified session."}),
        serde_json::json!({"name": "worktree_doctor", "summary": "Diagnose worktree registry drift and missing paths."}),
        serde_json::json!({"name": "truth_propose", "summary": "Propose a new truth record for a feature/session/worktree context."}),
        serde_json::json!({"name": "truth_accept", "summary": "Accept a proposed truth record."}),
        serde_json::json!({"name": "truth_reject", "summary": "Reject a proposed truth record."}),
        serde_json::json!({"name": "truth_supersede", "summary": "Mark an accepted truth as superseded by another accepted truth."}),
        serde_json::json!({"name": "truth_show", "summary": "Show a truth record by id."}),
        serde_json::json!({"name": "truth_list", "summary": "List truth records with filters by state/project/feature/session/worktree."}),
        serde_json::json!({"name": "truth_validate", "summary": "Validate truth events/projection consistency."}),
        serde_json::json!({"name": "truth_migrate_audit", "summary": "Detect legacy decision candidates for truth migration."}),
        serde_json::json!({"name": "truth_migrate_plan", "summary": "Build a truth migration plan from audit findings."}),
        serde_json::json!({"name": "truth_migrate_apply", "summary": "Apply a truth migration plan."}),
        serde_json::json!({"name": "list_tasks", "summary": "List tasks with optional filters."}),
        serde_json::json!({"name": "show_task", "summary": "Show a single task by id."}),
        serde_json::json!({"name": "ready_tasks", "summary": "List ready tasks (deps satisfied, status To Do)."}),
        serde_json::json!({"name": "next_task", "summary": "Return the next context-relevant task."}),
        serde_json::json!({"name": "next_tasks", "summary": "Recommend next work items ordered by context and readiness."}),
        serde_json::json!({"name": "stats", "summary": "Return counts by status."}),
        serde_json::json!({"name": "board", "summary": "Board (swimlanes) grouped by status/phase/priority."}),
        serde_json::json!({"name": "blockers", "summary": "Show blocked work and top blockers."}),
        serde_json::json!({"name": "validate", "summary": "Validate task metadata and dependencies."}),
        serde_json::json!({"name": "export_tasks", "summary": "Export all tasks as JSON."}),
        serde_json::json!({"name": "set_status", "summary": "Set task status."}),
        serde_json::json!({"name": "set_field", "summary": "Set a front matter field value."}),
        serde_json::json!({"name": "add_label", "summary": "Add a label to a task."}),
        serde_json::json!({"name": "remove_label", "summary": "Remove a label from a task."}),
        serde_json::json!({"name": "add_dependency", "summary": "Add a dependency to a task."}),
        serde_json::json!({"name": "remove_dependency", "summary": "Remove a dependency from a task."}),
        serde_json::json!({"name": "bulk_set_status", "summary": "Bulk update task statuses."}),
        serde_json::json!({"name": "bulk_set_field", "summary": "Bulk update a front matter field."}),
        serde_json::json!({"name": "bulk_add_label", "summary": "Bulk add a label to tasks."}),
        serde_json::json!({"name": "bulk_remove_label", "summary": "Bulk remove a label from tasks."}),
        serde_json::json!({"name": "bulk_add_dependency", "summary": "Bulk add a dependency to tasks."}),
        serde_json::json!({"name": "bulk_remove_dependency", "summary": "Bulk remove a dependency from tasks."}),
        serde_json::json!({"name": "bulk_add_note", "summary": "Bulk append a note to tasks."}),
        serde_json::json!({"name": "claim_task", "summary": "Claim a task lease."}),
        serde_json::json!({"name": "release_task", "summary": "Release a task lease."}),
        serde_json::json!({"name": "add_note", "summary": "Append a note to Notes or Implementation Notes."}),
        serde_json::json!({"name": "set_body", "summary": "Replace full task body (all content after front matter)."}),
        serde_json::json!({"name": "set_section", "summary": "Replace a named section in the task body."}),
        serde_json::json!({"name": "add_task", "summary": "Create a new task file with actionable content or explicit draft status."}),
        serde_json::json!({"name": "add_discovered", "summary": "Create a discovered task with actionable content or explicit draft status."}),
        serde_json::json!({"name": "archive_tasks", "summary": "Archive terminal tasks into date-based folders."}),
        serde_json::json!({"name": "migrate_backlog", "summary": "Migrate legacy backlog to workmesh/."}),
        serde_json::json!({"name": "migrate_audit", "summary": "Detect deprecated structures and report migration findings."}),
        serde_json::json!({"name": "migrate_plan", "summary": "Build migration plan from audit findings."}),
        serde_json::json!({"name": "migrate_apply", "summary": "Apply migration plan."}),
        serde_json::json!({"name": "checkpoint", "summary": "Write a session checkpoint."}),
        serde_json::json!({"name": "resume", "summary": "Resume from the latest checkpoint."}),
        serde_json::json!({"name": "checkpoint_diff", "summary": "Show changes since a checkpoint."}),
        serde_json::json!({"name": "session_save", "summary": "Save a global agent session."}),
        serde_json::json!({"name": "session_list", "summary": "List global agent sessions."}),
        serde_json::json!({"name": "session_show", "summary": "Show a global agent session."}),
        serde_json::json!({"name": "session_resume", "summary": "Resume from a global agent session."}),
        serde_json::json!({"name": "session_journal", "summary": "Append a session journal entry."}),
        serde_json::json!({"name": "working_set", "summary": "Write the working set file."}),
        serde_json::json!({"name": "project_init", "summary": "Create project docs scaffold."}),
        serde_json::json!({"name": "quickstart", "summary": "Scaffold docs + task/state roots + seed task."}),
        serde_json::json!({"name": "best_practices", "summary": "Return best practices guidance."}),
        serde_json::json!({"name": "help", "summary": "Show available tools and best practices."}),
        serde_json::json!({"name": "tool_info", "summary": "Show detailed usage for a specific tool."}),
        serde_json::json!({"name": "skill_content", "summary": "Return SKILL.md content for a repo skill."}),
        serde_json::json!({"name": "project_management_skill", "summary": "Return a project management guide for WorkMesh."}),
        serde_json::json!({"name": "graph_export", "summary": "Export task graph as JSON."}),
        serde_json::json!({"name": "issues_export", "summary": "Export tasks as JSONL."}),
        serde_json::json!({"name": "index_rebuild", "summary": "Rebuild JSONL task index."}),
        serde_json::json!({"name": "index_refresh", "summary": "Refresh JSONL task index."}),
        serde_json::json!({"name": "index_verify", "summary": "Verify JSONL task index."}),
        serde_json::json!({"name": "doctor", "summary": "Diagnostics report for repo layout, context, index, skills, and versions."}),
        serde_json::json!({"name": "gantt_text", "summary": "Return PlantUML gantt text for current tasks."}),
        serde_json::json!({"name": "gantt_file", "summary": "Write PlantUML gantt text to a file and return the path."}),
        serde_json::json!({"name": "gantt_svg", "summary": "Render gantt SVG via PlantUML; return SVG or a file path."}),
        serde_json::json!({"name": "render_table", "summary": "Render a table from array/object data."}),
        serde_json::json!({"name": "render_kv", "summary": "Render a key/value list."}),
        serde_json::json!({"name": "render_stats", "summary": "Render a compact stats block."}),
        serde_json::json!({"name": "render_list", "summary": "Render a list view."}),
        serde_json::json!({"name": "render_progress", "summary": "Render a progress bar or summary."}),
        serde_json::json!({"name": "render_tree", "summary": "Render a tree view from nested nodes."}),
        serde_json::json!({"name": "render_diff", "summary": "Render a unified diff from before/after values."}),
        serde_json::json!({"name": "render_logs", "summary": "Render log entries as a structured table."}),
        serde_json::json!({"name": "render_alerts", "summary": "Render alert summaries."}),
        serde_json::json!({"name": "render_chart_bar", "summary": "Render a simple bar chart."}),
        serde_json::json!({"name": "render_sparkline", "summary": "Render a sparkline chart."}),
        serde_json::json!({"name": "render_timeline", "summary": "Render a timeline view."}),
    ]
}

pub fn tool_examples(name: &str) -> Vec<Value> {
    let name = name.trim();
    match name {
        "version" => {
            vec![serde_json::json!({"tool": "version", "arguments": { "format": "json" }})]
        }
        "list_tasks" => vec![
            serde_json::json!({"tool": "list_tasks", "arguments": { "status": ["To Do"], "kind": ["bug"], "sort": "id", "format": "json" }}),
        ],
        "show_task" => vec![
            serde_json::json!({"tool": "show_task", "arguments": { "task_id": "task-001", "format": "json", "include_body": true }}),
        ],
        "next_task" => {
            vec![serde_json::json!({"tool": "next_task", "arguments": { "format": "json" }})]
        }
        "ready_tasks" => vec![
            serde_json::json!({"tool": "ready_tasks", "arguments": { "format": "json", "limit": 10 }}),
        ],
        "workstream_list" => {
            vec![serde_json::json!({"tool": "workstream_list", "arguments": { "format": "json" }})]
        }
        "workstream_create" => vec![
            serde_json::json!({"tool": "workstream_create", "arguments": { "name": "Feature A workstream", "key": "fa", "project_id": "demo", "objective": "Ship Feature A", "format": "json" }}),
        ],
        "workstream_show" => vec![
            serde_json::json!({"tool": "workstream_show", "arguments": { "id": "ws1-001", "restore": true, "truths": true, "format": "json" }}),
        ],
        "config_show" => {
            vec![serde_json::json!({"tool": "config_show", "arguments": { "format": "json" }})]
        }
        "config_set" => vec![
            serde_json::json!({"tool": "config_set", "arguments": { "scope": "global", "key": "auto_session_default", "value": "true", "format": "json" }}),
            serde_json::json!({"tool": "config_set", "arguments": { "scope": "global", "key": "auto_session_default", "value": "true", "format": "json", "verbose": true }}),
        ],
        "config_unset" => vec![
            serde_json::json!({"tool": "config_unset", "arguments": { "scope": "global", "key": "auto_session_default", "format": "json" }}),
        ],
        "workstream_switch" => vec![
            serde_json::json!({"tool": "workstream_switch", "arguments": { "id": "ws1-001", "format": "json" }}),
        ],
        "workstream_doctor" => vec![
            serde_json::json!({"tool": "workstream_doctor", "arguments": { "format": "json" }}),
        ],
        "workstream_restore" => vec![
            serde_json::json!({"tool": "workstream_restore", "arguments": { "all": false, "format": "json" }}),
        ],
        "worktree_list" => {
            vec![serde_json::json!({"tool": "worktree_list", "arguments": { "format": "json" }})]
        }
        "worktree_create" => vec![
            serde_json::json!({"tool": "worktree_create", "arguments": { "path": "../repo-feature-a", "branch": "feature/a", "project_id": "demo", "objective": "Implement feature A", "format": "json" }}),
        ],
        "worktree_attach" => vec![
            serde_json::json!({"tool": "worktree_attach", "arguments": { "path": "../repo-feature-a", "format": "json" }}),
        ],
        "worktree_detach" => {
            vec![serde_json::json!({"tool": "worktree_detach", "arguments": { "format": "json" }})]
        }
        "worktree_doctor" => {
            vec![serde_json::json!({"tool": "worktree_doctor", "arguments": { "format": "json" }})]
        }
        "truth_propose" => vec![
            serde_json::json!({"tool": "truth_propose", "arguments": { "title": "Use append-only truth events", "statement": "Truth records are append-only and immutable.", "project_id": "workmesh", "epic_id": "task-main-001", "format": "json" }}),
        ],
        "truth_accept" => vec![
            serde_json::json!({"tool": "truth_accept", "arguments": { "truth_id": "truth-01...", "note": "approved", "format": "json" }}),
        ],
        "truth_supersede" => vec![
            serde_json::json!({"tool": "truth_supersede", "arguments": { "truth_id": "truth-01old", "by_truth_id": "truth-01new", "reason": "replacement accepted", "format": "json" }}),
        ],
        "truth_list" => vec![
            serde_json::json!({"tool": "truth_list", "arguments": { "states": ["accepted"], "project_id": "workmesh", "epic_id": "task-main-001", "limit": 10, "format": "json" }}),
        ],
        "truth_validate" => {
            vec![serde_json::json!({"tool": "truth_validate", "arguments": { "format": "json" }})]
        }
        "set_status" => vec![
            serde_json::json!({"tool": "set_status", "arguments": { "task_id": "task-001", "status": "In Progress", "touch": true }}),
            serde_json::json!({"tool": "set_status", "arguments": { "task_id": "task-001", "status": "In Progress", "touch": true, "verbose": true }}),
        ],
        "set_field" => vec![
            serde_json::json!({"tool": "set_field", "arguments": { "task_id": "task-001", "field": "kind", "value": "bug", "touch": true }}),
        ],
        "bulk_set_status" => vec![
            serde_json::json!({"tool": "bulk_set_status", "arguments": { "tasks": ["task-001", "task-002"], "status": "In Progress", "touch": true }}),
            serde_json::json!({"tool": "bulk_set_status", "arguments": { "tasks": ["task-001", "task-002"], "status": "In Progress", "touch": true, "verbose": true }}),
        ],
        "bulk_set_field" => vec![
            serde_json::json!({"tool": "bulk_set_field", "arguments": { "tasks": ["task-001", "task-002"], "field": "priority", "value": "P1", "touch": true }}),
            serde_json::json!({"tool": "bulk_set_field", "arguments": { "tasks": ["task-001", "task-002"], "field": "priority", "value": "P1", "touch": true, "verbose": true }}),
        ],
        "bulk_add_label" => vec![
            serde_json::json!({"tool": "bulk_add_label", "arguments": { "tasks": ["task-001", "task-002"], "label": "docs", "touch": true }}),
            serde_json::json!({"tool": "bulk_add_label", "arguments": { "tasks": ["task-001", "task-002"], "label": "docs", "touch": true, "verbose": true }}),
        ],
        "bulk_remove_label" => vec![
            serde_json::json!({"tool": "bulk_remove_label", "arguments": { "tasks": ["task-001", "task-002"], "label": "docs", "touch": true }}),
            serde_json::json!({"tool": "bulk_remove_label", "arguments": { "tasks": ["task-001", "task-002"], "label": "docs", "touch": true, "verbose": true }}),
        ],
        "bulk_add_dependency" => vec![
            serde_json::json!({"tool": "bulk_add_dependency", "arguments": { "tasks": ["task-001", "task-002"], "dependency": "task-010", "touch": true }}),
            serde_json::json!({"tool": "bulk_add_dependency", "arguments": { "tasks": ["task-001", "task-002"], "dependency": "task-010", "touch": true, "verbose": true }}),
        ],
        "bulk_remove_dependency" => vec![
            serde_json::json!({"tool": "bulk_remove_dependency", "arguments": { "tasks": ["task-001", "task-002"], "dependency": "task-010", "touch": true }}),
            serde_json::json!({"tool": "bulk_remove_dependency", "arguments": { "tasks": ["task-001", "task-002"], "dependency": "task-010", "touch": true, "verbose": true }}),
        ],
        "bulk_add_note" => vec![
            serde_json::json!({"tool": "bulk_add_note", "arguments": { "tasks": ["task-001", "task-002"], "section": "Notes", "note": "Follow up with vendor", "touch": true }}),
            serde_json::json!({"tool": "bulk_add_note", "arguments": { "tasks": ["task-001", "task-002"], "section": "Notes", "note": "Follow up with vendor", "touch": true, "verbose": true }}),
        ],
        "add_task" => vec![
            serde_json::json!({"tool": "add_task", "arguments": { "title": "Investigate flaky test", "description": "- Investigate the flaky failure and identify the triggering condition.", "acceptance_criteria": "- The flaky scenario is reproducible or ruled out with evidence.", "definition_of_done": "- The root cause or next action is documented.\n- Code/config committed if changed.", "priority": "P2", "phase": "Phase1" }}),
            serde_json::json!({"tool": "add_task", "arguments": { "title": "Explore a rough idea", "draft": true, "status": "Draft", "priority": "P3", "phase": "Phase1", "verbose": true }}),
        ],
        "add_discovered" => vec![
            serde_json::json!({"tool": "add_discovered", "arguments": { "from": "task-001", "title": "New edge case discovered", "description": "- Capture the newly discovered edge case and required follow-up.", "acceptance_criteria": "- The edge case and expected handling are documented.", "definition_of_done": "- Follow-up work is clearly defined.\n- Docs updated if needed.", "priority": "P2", "phase": "Phase1" }}),
        ],
        "graph_export" => {
            vec![serde_json::json!({"tool": "graph_export", "arguments": { "pretty": true }})]
        }
        "export_tasks" => {
            vec![
                serde_json::json!({"tool": "export_tasks", "arguments": { "include_body": false }}),
            ]
        }
        "issues_export" => vec![
            serde_json::json!({"tool": "issues_export", "arguments": { "include_body": false }}),
        ],
        "index_rebuild" => vec![serde_json::json!({"tool": "index_rebuild", "arguments": {}})],
        "checkpoint" => vec![
            serde_json::json!({"tool": "checkpoint", "arguments": { "project": "workmesh", "json": true }}),
        ],
        "session_save" => vec![
            serde_json::json!({"tool": "session_save", "arguments": { "objective": "Continue migration work", "project": "workmesh", "format": "json" }}),
            serde_json::json!({"tool": "session_save", "arguments": { "objective": "Continue migration work", "project": "workmesh", "format": "json", "verbose": true }}),
        ],
        "resume" => vec![
            serde_json::json!({"tool": "resume", "arguments": { "project": "workmesh", "json": true }}),
        ],
        "help" => vec![serde_json::json!({"tool": "help", "arguments": { "format": "json" }})],
        "tool_info" => vec![
            serde_json::json!({"tool": "tool_info", "arguments": { "name": "list_tasks", "format": "text" }}),
        ],
        _ => vec![serde_json::json!({"tool": name, "arguments": {}})],
    }
}

pub fn supports_verbose_response(name: &str) -> bool {
    matches!(
        name,
        "config_set"
            | "config_unset"
            | "context_set"
            | "context_clear"
            | "workstream_create"
            | "workstream_switch"
            | "workstream_pause"
            | "workstream_close"
            | "workstream_reopen"
            | "workstream_rename"
            | "workstream_set"
            | "worktree_create"
            | "worktree_adopt_clone"
            | "worktree_attach"
            | "worktree_detach"
            | "truth_propose"
            | "truth_accept"
            | "truth_reject"
            | "truth_supersede"
            | "truth_migrate_apply"
            | "set_status"
            | "set_field"
            | "add_label"
            | "remove_label"
            | "add_dependency"
            | "remove_dependency"
            | "bulk_set_status"
            | "bulk_set_field"
            | "bulk_add_label"
            | "bulk_remove_label"
            | "bulk_add_dependency"
            | "bulk_remove_dependency"
            | "bulk_add_note"
            | "archive_tasks"
            | "migrate_backlog"
            | "migrate_apply"
            | "claim_task"
            | "release_task"
            | "add_note"
            | "set_body"
            | "set_section"
            | "add_task"
            | "add_discovered"
            | "session_save"
    )
}

pub fn placeholder_tool_definition(name: &str) -> Value {
    serde_json::json!({
        "name": name,
        "note": "Canonical full input schema is exposed through the MCP adapter tool registry."
    })
}

pub fn build_tool_info_payload(name: &str, tool_def: Value) -> Option<Value> {
    let name = name.trim();
    let summary = tool_catalog()
        .into_iter()
        .find(|tool| tool.get("name").and_then(|v| v.as_str()) == Some(name))
        .and_then(|tool| {
            tool.get("summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })?;

    let mut notes = vec![
        "root is optional if the server is started inside a repo with tasks/ + .workmesh/, a legacy single-root layout, or legacy backlog/tasks".to_string(),
        "List-style arguments accept CSV strings (\"a,b,c\") or JSON arrays (\"[\\\"a\\\",\\\"b\\\"]\").".to_string(),
    ];
    if name == "list_tasks" {
        notes.push(format!(
            "Task kind is free-form (not enforced). Suggested kinds: {}.",
            recommended_kinds().join(", ")
        ));
    }
    if name == "add_task" || name == "add_discovered" {
        notes.push(
            "New tasks default to kind=task in front matter. You can set kind later with set_field."
                .to_string(),
        );
        notes.push(
            "By default, actionable tasks must include description, acceptance criteria, and definition of done. Repos can override those requirements in config. Use draft=true for incomplete draft tasks."
                .to_string(),
        );
    }
    if name.starts_with("truth_") {
        notes.push(
            "Truth lifecycle is strict: proposed -> accepted|rejected, and accepted -> superseded."
                .to_string(),
        );
    }
    if supports_verbose_response(name) {
        notes.push(
            "Mutation tools return a minimal acknowledgement by default to save tokens. Pass verbose=true to include richer post-write state."
                .to_string(),
        );
    }

    Some(serde_json::json!({
        "ok": true,
        "name": name,
        "summary": summary,
        "tool": tool_def,
        "notes": notes,
        "examples": tool_examples(name),
    }))
}

pub fn render_tool_info_text(name: &str, info: &Value) -> String {
    let examples = info
        .get("examples")
        .and_then(|value| value.as_array())
        .map(|arr| {
            arr.iter()
                .map(|ex| serde_json::to_string_pretty(ex).unwrap_or_default())
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .unwrap_or_default();
    let notes = info
        .get("notes")
        .and_then(|value| value.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|line| format!("- {}", line))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    let summary = info.get("summary").and_then(|v| v.as_str()).unwrap_or("");
    let tool_def = info.get("tool").cloned().unwrap_or_default();
    format!(
        "Tool: {name}\n\nSummary:\n  {summary}\n\nTool definition:\n{tool_def}\n\nExamples:\n{examples}\n\nNotes:\n{notes}\n",
        name = name,
        summary = summary,
        tool_def = serde_json::to_string_pretty(&tool_def).unwrap_or_default(),
        examples = examples,
        notes = notes
    )
}

pub fn resolve_mcp_backlog_root(
    default_root: Option<&Path>,
    root: Option<&str>,
) -> Result<PathBuf, Value> {
    let root_value = root.and_then(trimmed_non_empty).map(PathBuf::from);
    let used_root = root_value.or_else(|| default_root.map(Path::to_path_buf));

    let resolved = if let Some(root_path) = &used_root {
        resolve_backlog_dir(root_path)
    } else {
        locate_backlog_dir(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    };

    match resolved {
        Ok(path) => Ok(path),
        Err(BacklogError::NotFound(_)) => {
            if let Some(root_path) = used_root {
                Err(
                    serde_json::json!({"error": format!("No tasks found under {}", root_path.display())}),
                )
            } else {
                Err(serde_json::json!({"error": ROOT_REQUIRED_ERROR}))
            }
        }
    }
}

pub fn resolve_repo_root_input(default_root: Option<&Path>, root: Option<&str>) -> PathBuf {
    if let Some(root_value) = root.and_then(trimmed_non_empty) {
        return PathBuf::from(root_value);
    }
    if let Some(default_root) = default_root {
        return default_root.to_path_buf();
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn resolve_cli_repo_root(root: &Path) -> PathBuf {
    if root.join("README.json").exists() || root.join(".git").exists() {
        return root.to_path_buf();
    }
    resolve_backlog(root)
        .map(|resolution| resolution.repo_root)
        .unwrap_or_else(|_| repo_root_from_backlog(root))
}

fn trimmed_non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn bulk_summary_is_compact_and_stable() {
        let value = bulk_summary(&["task-1".into(), "task-2".into()], &["task-3".into()]);
        assert_eq!(value["updated_count"], 2);
        assert_eq!(value["failed_count"], 1);
        assert_eq!(value["failed_ids"][0], "task-3");
        assert_eq!(value["ok"], false);
    }

    #[test]
    fn tool_info_payload_includes_verbose_note_for_mutations() {
        let info = build_tool_info_payload("set_status", placeholder_tool_definition("set_status"))
            .expect("payload");
        let notes = info["notes"].as_array().expect("notes");
        assert!(notes
            .iter()
            .any(|note| { note.as_str().unwrap_or_default().contains("verbose=true") }));
    }

    #[test]
    fn cli_repo_root_prefers_repo_root() {
        let temp = std::env::temp_dir().join(format!("workmesh-tools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join(".git")).expect("git dir");
        std::fs::write(temp.join("README.json"), "{}").expect("readme");
        assert_eq!(resolve_cli_repo_root(&temp), temp);
        let _ = std::fs::remove_dir_all(&temp);
    }
}
