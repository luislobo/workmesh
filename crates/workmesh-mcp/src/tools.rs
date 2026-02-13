use std::collections::HashSet;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::{Duration, Local, NaiveDate};
use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{
    schema_utils::CallToolError, CallToolRequestParams, CallToolResult, ListToolsResult,
    PaginatedRequestParams, RpcError, TextContent,
};
use rust_mcp_sdk::tool_box;
use rust_mcp_sdk::{mcp_server::ServerHandler, McpServer};
use serde::{Deserialize, Serialize};

use crate::version;

use workmesh_core::archive::{archive_tasks, ArchiveOptions};
use workmesh_core::audit::{append_audit_event, AuditEvent};
use workmesh_core::backlog::{
    locate_backlog_dir, resolve_backlog, resolve_backlog_dir, BacklogError,
};
use workmesh_core::context::{
    clear_context, context_path, extract_task_id_from_branch, infer_project_id, load_context,
    save_context, ContextScope, ContextScopeMode, ContextState,
};
use workmesh_core::doctor::doctor_report;
use workmesh_core::focus::load_focus;
use workmesh_core::gantt::{plantuml_gantt, render_plantuml_svg, write_text_file};
use workmesh_core::global_sessions::{
    append_session_saved, load_sessions_latest, new_session_id, now_rfc3339,
    read_current_session_id, resolve_workmesh_home, set_current_session, AgentSession,
    CheckpointRef, GitSnapshot, RecentChanges, WorktreeBinding,
};
use workmesh_core::id_fix::{fix_duplicate_task_ids, FixIdsOptions};
use workmesh_core::index::{rebuild_index, refresh_index, verify_index};
use workmesh_core::initiative::{
    best_effort_git_branch as core_git_branch, ensure_branch_initiative, next_namespaced_task_id,
};
use workmesh_core::migration::migrate_backlog;
use workmesh_core::migration_audit::{
    apply_migration_plan, audit_deprecations, plan_migrations, MigrationApplyOptions,
    MigrationPlanOptions,
};
use workmesh_core::project::{ensure_project_docs, repo_root_from_backlog};
use workmesh_core::quickstart::quickstart;
use workmesh_core::rekey::{
    parse_rekey_request, rekey_apply, render_rekey_prompt, RekeyApplyOptions, RekeyPromptOptions,
};
use workmesh_core::session::{
    append_session_journal, diff_since_checkpoint, render_diff, render_resume, resolve_project_id,
    resume_summary, task_summary, write_checkpoint, write_working_set, CheckpointOptions,
};
use workmesh_core::task::{load_tasks, load_tasks_with_archive, Lease, Task};
use workmesh_core::task_ops::{
    append_note, create_task_file, ensure_can_mark_done, filter_tasks, graph_export,
    is_lease_active, now_timestamp, ready_tasks, recommend_next_tasks_with_context,
    render_task_line, replace_section, set_list_field, sort_tasks, status_counts,
    task_to_json_value, tasks_to_jsonl, timestamp_plus_minutes, update_body, update_lease_fields,
    update_task_field, update_task_field_or_section, validate_tasks, FieldValue,
};
use workmesh_core::views::{
    blockers_report_with_context, board_lanes, scope_ids_from_context, BoardBy,
};
use workmesh_core::worktrees::{
    create_git_worktree, current_branch as current_worktree_branch, doctor_worktrees,
    find_worktree_record_by_path, list_worktree_views, upsert_worktree_record, WorktreeRecord,
};

const ROOT_REQUIRED_ERROR: &str = "root is required for MCP calls unless the server is started within a repo containing tasks/ or backlog/tasks";

#[derive(Clone)]
pub struct McpContext {
    pub default_root: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ListInput {
    String(String),
    List(Vec<String>),
}

fn parse_list_input(value: Option<ListInput>) -> Vec<String> {
    match value {
        None => Vec::new(),
        Some(ListInput::List(values)) => values
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect(),
        Some(ListInput::String(value)) => parse_list_string(&value),
    }
}

fn parse_list_string(value: &str) -> Vec<String> {
    let raw = value.trim();
    if raw.is_empty() || raw == "[]" {
        return Vec::new();
    }
    let inner = if raw.starts_with('[') && raw.ends_with(']') {
        raw[1..raw.len() - 1].trim()
    } else {
        raw
    };
    if inner.is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn parse_before_date(value: &str) -> Result<NaiveDate, CallToolError> {
    let trimmed = value.trim();
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(date);
    }
    if let Some(days) = trimmed.strip_suffix('d') {
        if let Ok(days) = days.parse::<i64>() {
            return Ok(Local::now().date_naive() - Duration::days(days));
        }
    }
    Err(CallToolError::from_message(format!(
        "Invalid date format: {}",
        value
    )))
}

fn resolve_root(context: &McpContext, root: Option<&str>) -> Result<PathBuf, serde_json::Value> {
    let root_value = root.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let used_root = if let Some(root_value) = root_value {
        Some(PathBuf::from(root_value))
    } else {
        context.default_root.clone()
    };

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

fn resolve_repo_root(context: &McpContext, root: Option<&str>) -> PathBuf {
    let root_value = root.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    if let Some(root_value) = root_value {
        return PathBuf::from(root_value);
    }
    if let Some(default_root) = &context.default_root {
        return default_root.clone();
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn load_context_state(backlog_dir: &Path) -> Option<ContextState> {
    if let Ok(Some(context)) = load_context(backlog_dir) {
        return Some(context);
    }
    let legacy = load_focus(backlog_dir).ok().flatten()?;
    Some(workmesh_core::context::context_from_legacy_focus(
        legacy.project_id,
        legacy.epic_id,
        legacy.objective,
        legacy.working_set,
    ))
}

fn read_skill_content(
    repo_root: &Path,
    name: &str,
) -> Result<workmesh_core::skills::SkillContent, serde_json::Value> {
    match workmesh_core::skills::load_skill_content(Some(repo_root), name) {
        Some(skill) => Ok(skill),
        None => Err(serde_json::json!({
            "error": format!("Skill not found: {}", name),
            "available_embedded": workmesh_core::skills::embedded_skill_ids(),
        })),
    }
}

fn ok_text(content: String) -> Result<CallToolResult, CallToolError> {
    Ok(CallToolResult::text_content(vec![TextContent::from(
        content,
    )]))
}

fn ok_json(value: serde_json::Value) -> Result<CallToolResult, CallToolError> {
    let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string());
    ok_text(text)
}

fn audit_event(
    backlog_dir: &Path,
    action: &str,
    task_id: Option<&str>,
    details: serde_json::Value,
) -> Result<(), CallToolError> {
    let event = AuditEvent {
        timestamp: now_timestamp(),
        actor: Some("mcp".to_string()),
        action: action.to_string(),
        task_id: task_id.map(|value| value.to_string()),
        details,
    };
    append_audit_event(backlog_dir, &event).map_err(CallToolError::new)
}

fn refresh_index_best_effort(backlog_dir: &Path) {
    let _ = refresh_index(backlog_dir);
}

fn best_practice_hints() -> Vec<&'static str> {
    vec![
        "Always record dependencies for tasks that are blocked by other work.",
        "Use dependencies to power next-task selection and blocked/ready views.",
        "If unsure, start with an empty list and add dependencies as soon as blockers appear.",
        "Prefer specific task ids (e.g., task-042) over vague references.",
        "Update dependencies when status changes to avoid stale blocked tasks.",
        "Do not commit derived artifacts like `workmesh/.index/` or `workmesh/.audit.log` (they are rebuildable).",
    ]
}

fn recommended_kinds() -> Vec<&'static str> {
    vec![
        "epic", "story", "task", "bug", "subtask", "incident", "spike",
    ]
}

fn tool_catalog() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({"name": "version", "summary": "Return WorkMesh version information."}),
        serde_json::json!({"name": "readme", "summary": "Return README.json (agent-friendly repo docs)."}),
        serde_json::json!({"name": "doctor", "summary": "Diagnostics report for repo layout, context, index, skills, and versions."}),
        serde_json::json!({"name": "context_show", "summary": "Show repo-local context (project/objective/scope)."}),
        serde_json::json!({"name": "context_set", "summary": "Set repo-local context (project/objective/scope)."}),
        serde_json::json!({"name": "context_clear", "summary": "Clear repo-local context."}),
        serde_json::json!({"name": "focus_show", "summary": "Deprecated alias for context_show."}),
        serde_json::json!({"name": "focus_set", "summary": "Deprecated alias for context_set."}),
        serde_json::json!({"name": "focus_clear", "summary": "Deprecated alias for context_clear."}),
        serde_json::json!({"name": "worktree_list", "summary": "List worktrees (git + registry)."}),
        serde_json::json!({"name": "worktree_create", "summary": "Create a git worktree and register it."}),
        serde_json::json!({"name": "worktree_attach", "summary": "Attach current/specified session to a worktree."}),
        serde_json::json!({"name": "worktree_detach", "summary": "Detach worktree from current/specified session."}),
        serde_json::json!({"name": "worktree_doctor", "summary": "Diagnose worktree registry drift and missing paths."}),
        serde_json::json!({"name": "list_tasks", "summary": "List tasks with filters and sorting."}),
        serde_json::json!({"name": "show_task", "summary": "Show a single task by id."}),
        serde_json::json!({"name": "next_task", "summary": "Get the next context-relevant task (active/leased first, else next ready To Do)."}),
        serde_json::json!({"name": "next_tasks", "summary": "Get a deterministic list of next-task candidates (includes active work; context-aware)."}),
        serde_json::json!({"name": "ready_tasks", "summary": "List tasks with deps satisfied (ready work)."}),
        serde_json::json!({"name": "board", "summary": "Board (swimlanes) grouped by status/phase/priority (optionally context-scoped)."}),
        serde_json::json!({"name": "blockers", "summary": "Blocked work and top blockers (scoped to context epic by default)."}),
        serde_json::json!({"name": "export_tasks", "summary": "Export all tasks as JSON."}),
        serde_json::json!({"name": "set_status", "summary": "Update task status."}),
        serde_json::json!({"name": "set_field", "summary": "Update a front matter field."}),
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
        serde_json::json!({"name": "archive_tasks", "summary": "Archive done tasks into date-based folders."}),
        serde_json::json!({"name": "migrate_backlog", "summary": "Migrate legacy backlog to workmesh/."}),
        serde_json::json!({"name": "migrate_audit", "summary": "Detect deprecated structures and produce migration findings."}),
        serde_json::json!({"name": "migrate_plan", "summary": "Build migration plan from findings."}),
        serde_json::json!({"name": "migrate_apply", "summary": "Apply migration plan (dry-run by default)."}),
        serde_json::json!({"name": "claim_task", "summary": "Claim a task lease."}),
        serde_json::json!({"name": "release_task", "summary": "Release a task lease."}),
        serde_json::json!({"name": "add_note", "summary": "Append a note to Notes or Implementation Notes."}),
        serde_json::json!({"name": "set_body", "summary": "Replace full task body (after front matter)."}),
        serde_json::json!({"name": "set_section", "summary": "Replace a named section in the task body."}),
        serde_json::json!({"name": "add_task", "summary": "Create a new task file."}),
        serde_json::json!({"name": "add_discovered", "summary": "Create a task discovered from another task."}),
        serde_json::json!({"name": "project_init", "summary": "Create project docs scaffold."}),
        serde_json::json!({"name": "quickstart", "summary": "Scaffold docs + backlog + seed task."}),
        serde_json::json!({"name": "validate", "summary": "Validate task metadata and dependencies."}),
        serde_json::json!({"name": "graph_export", "summary": "Export task graph as JSON."}),
        serde_json::json!({"name": "issues_export", "summary": "Export tasks as JSONL."}),
        serde_json::json!({"name": "index_rebuild", "summary": "Rebuild JSONL task index."}),
        serde_json::json!({"name": "index_refresh", "summary": "Refresh JSONL task index."}),
        serde_json::json!({"name": "index_verify", "summary": "Verify JSONL task index."}),
        serde_json::json!({"name": "checkpoint", "summary": "Write a session checkpoint (JSON + Markdown)."}),
        serde_json::json!({"name": "resume", "summary": "Resume from the latest checkpoint."}),
        serde_json::json!({"name": "working_set", "summary": "Write the working set file."}),
        serde_json::json!({"name": "session_journal", "summary": "Append a session journal entry."}),
        serde_json::json!({"name": "checkpoint_diff", "summary": "Show changes since a checkpoint."}),
        serde_json::json!({"name": "gantt_text", "summary": "Return PlantUML gantt text."}),
        serde_json::json!({"name": "gantt_file", "summary": "Write PlantUML gantt to a file."}),
        serde_json::json!({"name": "gantt_svg", "summary": "Render gantt SVG via PlantUML."}),
        serde_json::json!({"name": "best_practices", "summary": "Return best practices guidance."}),
        serde_json::json!({"name": "help", "summary": "Show available tools and best practices."}),
        serde_json::json!({"name": "tool_info", "summary": "Show detailed usage for a specific tool."}),
        serde_json::json!({"name": "skill_content", "summary": "Return SKILL.md content for a repo skill."}),
        serde_json::json!({"name": "project_management_skill", "summary": "Return project management skill content (default: workmesh-mcp)."}),
    ]
}

#[mcp_tool(name = "version", description = "Return WorkMesh version information.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct VersionTool {
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "readme",
    description = "Return the repo README in JSON form (README.json) for fast agent consumption."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadmeTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "doctor",
    description = "Return a diagnostics report for repo layout, context, index, skills, and versions."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DoctorTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(name = "context_show", description = "Show repo-local context state.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ContextShowTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(name = "context_set", description = "Set repo-local context state.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ContextSetTool {
    pub root: Option<String>,
    pub project_id: Option<String>,
    pub epic_id: Option<String>,
    pub objective: Option<String>,
    pub tasks: Option<ListInput>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "context_clear",
    description = "Clear repo-local context state."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ContextClearTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "focus_show",
    description = "Deprecated alias for context_show. Show repo-local context state."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FocusShowTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "focus_set",
    description = "Deprecated alias for context_set. Set repo-local context state."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FocusSetTool {
    pub root: Option<String>,
    pub project_id: Option<String>,
    pub epic_id: Option<String>,
    pub objective: Option<String>,
    pub tasks: Option<ListInput>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "focus_clear",
    description = "Deprecated alias for context_clear. Clear repo-local context state."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FocusClearTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "worktree_list",
    description = "List worktrees (git + registry)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WorktreeListTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "worktree_create",
    description = "Create a git worktree and register it (optional context seed)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WorktreeCreateTool {
    pub root: Option<String>,
    pub path: String,
    pub branch: String,
    pub from: Option<String>,
    pub project_id: Option<String>,
    pub epic_id: Option<String>,
    pub objective: Option<String>,
    pub tasks: Option<ListInput>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "worktree_attach",
    description = "Attach current/specified session to a worktree path."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WorktreeAttachTool {
    pub root: Option<String>,
    pub session_id: Option<String>,
    pub path: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "worktree_detach",
    description = "Detach worktree metadata from current/specified session."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WorktreeDetachTool {
    pub session_id: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "worktree_doctor",
    description = "Diagnose worktree registry drift and missing paths."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WorktreeDoctorTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(name = "list_tasks", description = "List tasks with optional filters.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListTasksTool {
    pub root: Option<String>,
    /// Include archived tasks under `workmesh/archive/` (recursively).
    #[serde(default)]
    pub all: bool,
    pub status: Option<ListInput>,
    pub kind: Option<ListInput>,
    pub phase: Option<ListInput>,
    pub priority: Option<ListInput>,
    pub labels: Option<ListInput>,
    pub depends_on: Option<String>,
    pub deps_satisfied: Option<bool>,
    pub blocked: Option<bool>,
    pub search: Option<String>,
    #[serde(default = "default_sort")]
    pub sort: String,
    pub limit: Option<u32>,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub include_hints: bool,
}

#[mcp_tool(name = "show_task", description = "Show a single task by id.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ShowTaskTool {
    pub task_id: String,
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_include_body")]
    pub include_body: bool,
}

#[mcp_tool(
    name = "next_task",
    description = "Return the next context-relevant task (active/leased first, else next ready To Do)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NextTaskTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "next_tasks",
    description = "Recommend next work items (active/leased first, then ready To Do), ordered deterministically and biased by context. Use this when an agent should choose among candidates."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NextTasksTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
    pub limit: Option<u32>,
}

#[mcp_tool(
    name = "ready_tasks",
    description = "List ready tasks (deps satisfied, status To Do)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadyTasksTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
    pub limit: Option<u32>,
}

#[mcp_tool(
    name = "board",
    description = "Board (swimlanes) grouped by status/phase/priority. Use --focus to scope to current context."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BoardTool {
    pub root: Option<String>,
    /// Include archived tasks under `workmesh/archive/` (recursively).
    #[serde(default)]
    pub all: bool,
    /// Group lanes by: status|phase|priority
    #[serde(default = "default_board_by")]
    pub by: String,
    /// Scope to context epic subtree or explicit context task scope.
    #[serde(default)]
    pub focus: bool,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "blockers",
    description = "Show blocked work and top blockers (scoped to context epic by default)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BlockersTool {
    pub root: Option<String>,
    /// Include archived tasks under `workmesh/archive/` (recursively).
    #[serde(default)]
    pub all: bool,
    /// Override context epic id for scoping.
    pub epic_id: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(name = "export_tasks", description = "Export all tasks as JSON.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ExportTasksTool {
    pub root: Option<String>,
    #[serde(default = "default_include_body")]
    pub include_body: bool,
}

#[mcp_tool(name = "stats", description = "Return counts by status.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct StatsTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(name = "set_status", description = "Set task status.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetStatusTool {
    pub task_id: String,
    pub status: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(name = "set_field", description = "Set a front matter field value.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetFieldTool {
    pub task_id: String,
    pub field: String,
    pub value: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(name = "add_label", description = "Add a label to a task.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddLabelTool {
    pub task_id: String,
    pub label: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(name = "remove_label", description = "Remove a label from a task.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RemoveLabelTool {
    pub task_id: String,
    pub label: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(name = "add_dependency", description = "Add a dependency to a task.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddDependencyTool {
    pub task_id: String,
    pub dependency: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(
    name = "remove_dependency",
    description = "Remove a dependency from a task."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RemoveDependencyTool {
    pub task_id: String,
    pub dependency: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(name = "bulk_set_status", description = "Bulk update task statuses.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BulkSetStatusTool {
    pub tasks: Option<ListInput>,
    pub status: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(
    name = "bulk_set_field",
    description = "Bulk update a front matter field."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BulkSetFieldTool {
    pub tasks: Option<ListInput>,
    pub field: String,
    pub value: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(name = "bulk_add_label", description = "Bulk add a label to tasks.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BulkAddLabelTool {
    pub tasks: Option<ListInput>,
    pub label: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(
    name = "bulk_remove_label",
    description = "Bulk remove a label from tasks."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BulkRemoveLabelTool {
    pub tasks: Option<ListInput>,
    pub label: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(
    name = "bulk_add_dependency",
    description = "Bulk add a dependency to tasks."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BulkAddDependencyTool {
    pub tasks: Option<ListInput>,
    pub dependency: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(
    name = "bulk_remove_dependency",
    description = "Bulk remove a dependency from tasks."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BulkRemoveDependencyTool {
    pub tasks: Option<ListInput>,
    pub dependency: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(name = "bulk_add_note", description = "Bulk append a note to tasks.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BulkAddNoteTool {
    pub tasks: Option<ListInput>,
    pub note: String,
    pub root: Option<String>,
    #[serde(default = "default_notes_section")]
    pub section: String,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(
    name = "archive_tasks",
    description = "Archive done tasks into date-based folders."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ArchiveTool {
    pub root: Option<String>,
    #[serde(default = "default_archive_before")]
    pub before: String,
    #[serde(default = "default_status")]
    pub status: String,
}

#[mcp_tool(
    name = "migrate_backlog",
    description = "Migrate legacy backlog to workmesh/ (compat tool)"
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct MigrateTool {
    pub root: Option<String>,
    pub to: Option<String>,
}

#[mcp_tool(
    name = "migrate_audit",
    description = "Detect deprecated structures and report migration findings."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct MigrateAuditTool {
    pub root: Option<String>,
}

#[mcp_tool(
    name = "migrate_plan",
    description = "Build migration plan from audit findings."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct MigratePlanTool {
    pub root: Option<String>,
    pub include: Option<ListInput>,
    pub exclude: Option<ListInput>,
}

#[mcp_tool(
    name = "migrate_apply",
    description = "Apply migration plan (dry-run by default)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct MigrateApplyTool {
    pub root: Option<String>,
    #[serde(default)]
    pub include: Option<ListInput>,
    #[serde(default)]
    pub exclude: Option<ListInput>,
    #[serde(default)]
    pub apply: bool,
    #[serde(default)]
    pub backup: bool,
}

#[mcp_tool(name = "claim_task", description = "Claim a task lease.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ClaimTaskTool {
    pub task_id: String,
    pub owner: String,
    pub root: Option<String>,
    pub minutes: Option<i64>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(name = "release_task", description = "Release a task lease.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReleaseTaskTool {
    pub task_id: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(
    name = "add_note",
    description = "Append a note to Notes or Implementation Notes."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddNoteTool {
    pub task_id: String,
    pub note: String,
    pub root: Option<String>,
    #[serde(default = "default_notes_section")]
    pub section: String,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(
    name = "set_body",
    description = "Replace full task body (all content after front matter)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetBodyTool {
    pub task_id: String,
    pub body: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(
    name = "set_section",
    description = "Replace a named section in the task body."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetSectionTool {
    pub task_id: String,
    pub section: String,
    pub content: String,
    pub root: Option<String>,
    #[serde(default = "default_touch")]
    pub touch: bool,
}

#[mcp_tool(name = "add_task", description = "Create a new task file.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddTaskTool {
    pub title: String,
    pub root: Option<String>,
    pub task_id: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_phase")]
    pub phase: String,
    pub labels: Option<ListInput>,
    pub dependencies: Option<ListInput>,
    pub assignee: Option<ListInput>,
}

#[mcp_tool(
    name = "add_discovered",
    description = "Create a task discovered from another task."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddDiscoveredTool {
    pub from: String,
    pub title: String,
    pub root: Option<String>,
    pub task_id: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_phase")]
    pub phase: String,
    pub labels: Option<ListInput>,
    pub dependencies: Option<ListInput>,
    pub assignee: Option<ListInput>,
}

#[mcp_tool(name = "project_init", description = "Create project docs scaffold.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ProjectInitTool {
    pub project_id: String,
    pub root: Option<String>,
    pub name: Option<String>,
}

#[mcp_tool(
    name = "quickstart",
    description = "Scaffold docs + backlog + seed task."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct QuickstartTool {
    pub project_id: String,
    pub root: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub agents_snippet: bool,
}

#[mcp_tool(
    name = "validate",
    description = "Validate task metadata and dependencies."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ValidateTool {
    pub root: Option<String>,
}

#[mcp_tool(
    name = "fix_ids",
    description = "Fix duplicate task ids after merges (dry-run unless apply=true)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FixIdsTool {
    pub root: Option<String>,
    #[serde(default)]
    pub apply: bool,
}

#[mcp_tool(
    name = "rekey_prompt",
    description = "Generate an agent prompt to propose a task-id rekey mapping (and reference rewrites)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RekeyPromptTool {
    pub root: Option<String>,
    #[serde(default)]
    pub all: bool,
    #[serde(default)]
    pub include_body: bool,
    pub limit: Option<u32>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "rekey_apply",
    description = "Apply a task-id rekey mapping and rewrite structured references (dependencies + relationships)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RekeyApplyTool {
    pub root: Option<String>,
    #[serde(default)]
    pub apply: bool,
    #[serde(default)]
    pub all: bool,
    /// JSON request. Either `{ \"mapping\": { ... }, \"strict\": true }` or the mapping object directly.
    pub mapping_json: String,
}

#[mcp_tool(name = "graph_export", description = "Export task graph as JSON.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GraphExportTool {
    pub root: Option<String>,
    #[serde(default)]
    pub pretty: bool,
}

#[mcp_tool(name = "issues_export", description = "Export tasks as JSONL.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct IssuesExportTool {
    pub root: Option<String>,
    #[serde(default)]
    pub include_body: bool,
}

#[mcp_tool(name = "index_rebuild", description = "Rebuild JSONL task index.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct IndexRebuildTool {
    pub root: Option<String>,
}

#[mcp_tool(name = "index_refresh", description = "Refresh JSONL task index.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct IndexRefreshTool {
    pub root: Option<String>,
}

#[mcp_tool(name = "index_verify", description = "Verify JSONL task index.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct IndexVerifyTool {
    pub root: Option<String>,
}

#[mcp_tool(
    name = "checkpoint",
    description = "Write a session checkpoint (JSON + Markdown)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CheckpointTool {
    pub root: Option<String>,
    pub project: Option<String>,
    pub id: Option<String>,
    pub audit_limit: Option<u32>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(name = "resume", description = "Resume from the latest checkpoint.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ResumeTool {
    pub root: Option<String>,
    pub project: Option<String>,
    pub id: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(name = "working_set", description = "Write the working set file.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WorkingSetTool {
    pub root: Option<String>,
    pub project: Option<String>,
    pub tasks: Option<ListInput>,
    pub note: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "session_journal",
    description = "Append a session journal entry."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SessionJournalTool {
    pub root: Option<String>,
    pub project: Option<String>,
    pub task: Option<String>,
    pub next: Option<String>,
    pub note: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "checkpoint_diff",
    description = "Show changes since a checkpoint."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CheckpointDiffTool {
    pub root: Option<String>,
    pub project: Option<String>,
    pub id: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "session_save",
    description = "Save a global agent session (cross-repo continuity)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SessionSaveTool {
    pub objective: String,
    pub cwd: Option<String>,
    pub project: Option<String>,
    pub tasks: Option<ListInput>,
    pub notes: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "session_list",
    description = "List global agent sessions (cross-repo continuity)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SessionListTool {
    pub limit: Option<u32>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(name = "session_show", description = "Show a global agent session.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SessionShowTool {
    pub session_id: String,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "session_resume",
    description = "Resume from a global agent session (summary + suggested commands)."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SessionResumeTool {
    pub session_id: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "best_practices",
    description = "Return best practices guidance."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BestPracticesTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(
    name = "gantt_text",
    description = "Return PlantUML gantt text for current tasks."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GanttTextTool {
    pub root: Option<String>,
    pub start: Option<String>,
    #[serde(default = "default_zoom")]
    pub zoom: i32,
}

#[mcp_tool(
    name = "gantt_file",
    description = "Write PlantUML gantt text to a file and return the path."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GanttFileTool {
    pub output: String,
    pub root: Option<String>,
    pub start: Option<String>,
    #[serde(default = "default_zoom")]
    pub zoom: i32,
}

#[mcp_tool(
    name = "gantt_svg",
    description = "Render gantt SVG via PlantUML; return SVG or a file path."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GanttSvgTool {
    pub root: Option<String>,
    pub start: Option<String>,
    #[serde(default = "default_zoom")]
    pub zoom: i32,
    pub output: Option<String>,
    pub plantuml_cmd: Option<String>,
    pub plantuml_jar: Option<String>,
}

#[mcp_tool(
    name = "skill_content",
    description = "Return SKILL.md content for a repo skill."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SkillContentTool {
    pub root: Option<String>,
    pub name: Option<String>,
    #[serde(default = "default_text_format")]
    pub format: String,
}

#[mcp_tool(
    name = "help",
    description = "Show available tools and best practices."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct HelpTool {
    pub root: Option<String>,
    #[serde(default = "default_text_format")]
    pub format: String,
}

#[mcp_tool(
    name = "tool_info",
    description = "Show detailed usage for a specific tool."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ToolInfoTool {
    pub name: String,
    pub root: Option<String>,
    #[serde(default = "default_text_format")]
    pub format: String,
}

#[mcp_tool(
    name = "project_management_skill",
    description = "Return a project management guide for WorkMesh."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ProjectManagementSkillTool {
    pub root: Option<String>,
    /// Skill name to fetch (defaults to workmesh-mcp)
    pub name: Option<String>,
    #[serde(default = "default_text_format")]
    pub format: String,
}

fn default_sort() -> String {
    "id".to_string()
}

fn default_board_by() -> String {
    "status".to_string()
}

fn default_format() -> String {
    "json".to_string()
}

fn default_text_format() -> String {
    "text".to_string()
}

fn default_include_body() -> bool {
    true
}

fn default_notes_section() -> String {
    "notes".to_string()
}

fn default_status() -> String {
    "To Do".to_string()
}

fn default_priority() -> String {
    "P2".to_string()
}

fn default_phase() -> String {
    "Phase1".to_string()
}

fn default_zoom() -> i32 {
    3
}

fn default_archive_before() -> String {
    "30d".to_string()
}

fn default_touch() -> bool {
    true
}

fn is_done_status(status: &str) -> bool {
    status.eq_ignore_ascii_case("done")
}

// Generates enum WorkmeshTools with variants for each tool
tool_box!(
    WorkmeshTools,
    [
        VersionTool,
        ReadmeTool,
        DoctorTool,
        ContextShowTool,
        ContextSetTool,
        ContextClearTool,
        FocusShowTool,
        FocusSetTool,
        FocusClearTool,
        WorktreeListTool,
        WorktreeCreateTool,
        WorktreeAttachTool,
        WorktreeDetachTool,
        WorktreeDoctorTool,
        ListTasksTool,
        ShowTaskTool,
        NextTaskTool,
        NextTasksTool,
        ReadyTasksTool,
        BoardTool,
        BlockersTool,
        ExportTasksTool,
        StatsTool,
        SetStatusTool,
        SetFieldTool,
        AddLabelTool,
        RemoveLabelTool,
        AddDependencyTool,
        RemoveDependencyTool,
        BulkSetStatusTool,
        BulkSetFieldTool,
        BulkAddLabelTool,
        BulkRemoveLabelTool,
        BulkAddDependencyTool,
        BulkRemoveDependencyTool,
        BulkAddNoteTool,
        ArchiveTool,
        MigrateTool,
        MigrateAuditTool,
        MigratePlanTool,
        MigrateApplyTool,
        ClaimTaskTool,
        ReleaseTaskTool,
        AddNoteTool,
        SetBodyTool,
        SetSectionTool,
        AddTaskTool,
        AddDiscoveredTool,
        ProjectInitTool,
        QuickstartTool,
        ValidateTool,
        FixIdsTool,
        RekeyPromptTool,
        RekeyApplyTool,
        GraphExportTool,
        IssuesExportTool,
        IndexRebuildTool,
        IndexRefreshTool,
        IndexVerifyTool,
        CheckpointTool,
        ResumeTool,
        WorkingSetTool,
        SessionJournalTool,
        CheckpointDiffTool,
        SessionSaveTool,
        SessionListTool,
        SessionShowTool,
        SessionResumeTool,
        GanttTextTool,
        GanttFileTool,
        GanttSvgTool,
        BestPracticesTool,
        SkillContentTool,
        HelpTool,
        ToolInfoTool,
        ProjectManagementSkillTool
    ]
);

pub struct WorkmeshServerHandler {
    pub context: McpContext,
}

#[async_trait]
impl ServerHandler for WorkmeshServerHandler {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: WorkmeshTools::tools(),
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        let tool = WorkmeshTools::try_from(params).map_err(CallToolError::new)?;
        match tool {
            WorkmeshTools::VersionTool(tool) => tool.call(&self.context),
            WorkmeshTools::ReadmeTool(tool) => tool.call(&self.context),
            WorkmeshTools::DoctorTool(tool) => tool.call(&self.context),
            WorkmeshTools::ContextShowTool(tool) => tool.call(&self.context),
            WorkmeshTools::ContextSetTool(tool) => tool.call(&self.context),
            WorkmeshTools::ContextClearTool(tool) => tool.call(&self.context),
            WorkmeshTools::FocusShowTool(tool) => tool.call(&self.context),
            WorkmeshTools::FocusSetTool(tool) => tool.call(&self.context),
            WorkmeshTools::FocusClearTool(tool) => tool.call(&self.context),
            WorkmeshTools::WorktreeListTool(tool) => tool.call(&self.context),
            WorkmeshTools::WorktreeCreateTool(tool) => tool.call(&self.context),
            WorkmeshTools::WorktreeAttachTool(tool) => tool.call(&self.context),
            WorkmeshTools::WorktreeDetachTool(tool) => tool.call(&self.context),
            WorkmeshTools::WorktreeDoctorTool(tool) => tool.call(&self.context),
            WorkmeshTools::ListTasksTool(tool) => tool.call(&self.context),
            WorkmeshTools::ShowTaskTool(tool) => tool.call(&self.context),
            WorkmeshTools::NextTaskTool(tool) => tool.call(&self.context),
            WorkmeshTools::NextTasksTool(tool) => tool.call(&self.context),
            WorkmeshTools::ReadyTasksTool(tool) => tool.call(&self.context),
            WorkmeshTools::BoardTool(tool) => tool.call(&self.context),
            WorkmeshTools::BlockersTool(tool) => tool.call(&self.context),
            WorkmeshTools::ExportTasksTool(tool) => tool.call(&self.context),
            WorkmeshTools::StatsTool(tool) => tool.call(&self.context),
            WorkmeshTools::SetStatusTool(tool) => tool.call(&self.context),
            WorkmeshTools::SetFieldTool(tool) => tool.call(&self.context),
            WorkmeshTools::AddLabelTool(tool) => tool.call(&self.context),
            WorkmeshTools::RemoveLabelTool(tool) => tool.call(&self.context),
            WorkmeshTools::AddDependencyTool(tool) => tool.call(&self.context),
            WorkmeshTools::RemoveDependencyTool(tool) => tool.call(&self.context),
            WorkmeshTools::BulkSetStatusTool(tool) => tool.call(&self.context),
            WorkmeshTools::BulkSetFieldTool(tool) => tool.call(&self.context),
            WorkmeshTools::BulkAddLabelTool(tool) => tool.call(&self.context),
            WorkmeshTools::BulkRemoveLabelTool(tool) => tool.call(&self.context),
            WorkmeshTools::BulkAddDependencyTool(tool) => tool.call(&self.context),
            WorkmeshTools::BulkRemoveDependencyTool(tool) => tool.call(&self.context),
            WorkmeshTools::BulkAddNoteTool(tool) => tool.call(&self.context),
            WorkmeshTools::ArchiveTool(tool) => tool.call(&self.context),
            WorkmeshTools::MigrateTool(tool) => tool.call(&self.context),
            WorkmeshTools::MigrateAuditTool(tool) => tool.call(&self.context),
            WorkmeshTools::MigratePlanTool(tool) => tool.call(&self.context),
            WorkmeshTools::MigrateApplyTool(tool) => tool.call(&self.context),
            WorkmeshTools::ClaimTaskTool(tool) => tool.call(&self.context),
            WorkmeshTools::ReleaseTaskTool(tool) => tool.call(&self.context),
            WorkmeshTools::AddNoteTool(tool) => tool.call(&self.context),
            WorkmeshTools::SetBodyTool(tool) => tool.call(&self.context),
            WorkmeshTools::SetSectionTool(tool) => tool.call(&self.context),
            WorkmeshTools::AddTaskTool(tool) => tool.call(&self.context),
            WorkmeshTools::AddDiscoveredTool(tool) => tool.call(&self.context),
            WorkmeshTools::ProjectInitTool(tool) => tool.call(&self.context),
            WorkmeshTools::QuickstartTool(tool) => tool.call(&self.context),
            WorkmeshTools::ValidateTool(tool) => tool.call(&self.context),
            WorkmeshTools::FixIdsTool(tool) => tool.call(&self.context),
            WorkmeshTools::RekeyPromptTool(tool) => tool.call(&self.context),
            WorkmeshTools::RekeyApplyTool(tool) => tool.call(&self.context),
            WorkmeshTools::GraphExportTool(tool) => tool.call(&self.context),
            WorkmeshTools::IssuesExportTool(tool) => tool.call(&self.context),
            WorkmeshTools::IndexRebuildTool(tool) => tool.call(&self.context),
            WorkmeshTools::IndexRefreshTool(tool) => tool.call(&self.context),
            WorkmeshTools::IndexVerifyTool(tool) => tool.call(&self.context),
            WorkmeshTools::CheckpointTool(tool) => tool.call(&self.context),
            WorkmeshTools::ResumeTool(tool) => tool.call(&self.context),
            WorkmeshTools::WorkingSetTool(tool) => tool.call(&self.context),
            WorkmeshTools::SessionJournalTool(tool) => tool.call(&self.context),
            WorkmeshTools::CheckpointDiffTool(tool) => tool.call(&self.context),
            WorkmeshTools::SessionSaveTool(tool) => tool.call(&self.context),
            WorkmeshTools::SessionListTool(tool) => tool.call(&self.context),
            WorkmeshTools::SessionShowTool(tool) => tool.call(&self.context),
            WorkmeshTools::SessionResumeTool(tool) => tool.call(&self.context),
            WorkmeshTools::GanttTextTool(tool) => tool.call(&self.context),
            WorkmeshTools::GanttFileTool(tool) => tool.call(&self.context),
            WorkmeshTools::GanttSvgTool(tool) => tool.call(&self.context),
            WorkmeshTools::BestPracticesTool(tool) => tool.call(&self.context),
            WorkmeshTools::SkillContentTool(tool) => tool.call(&self.context),
            WorkmeshTools::HelpTool(tool) => tool.call(&self.context),
            WorkmeshTools::ToolInfoTool(tool) => tool.call(&self.context),
            WorkmeshTools::ProjectManagementSkillTool(tool) => tool.call(&self.context),
        }
    }
}

impl VersionTool {
    fn call(&self, _context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let payload = serde_json::json!({
            "name": "workmesh",
            "version": env!("CARGO_PKG_VERSION"),
            "full": version::FULL,
        });

        if self.format == "text" {
            return ok_text(format!(
                "workmesh {}\n{}\n",
                payload
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default(),
                payload
                    .get("full")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
            ));
        }

        ok_json(payload)
    }
}

impl ReadmeTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let path = repo_root.join("README.json");
        let raw = std::fs::read_to_string(&path)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;

        if self.format == "text" {
            return ok_text(raw);
        }
        ok_json(serde_json::json!({
            "ok": true,
            "path": path,
            "readme": parsed
        }))
    }
}

impl DoctorTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let report = doctor_report(&repo_root, "workmesh-mcp");
        if self.format == "text" {
            return ok_text(
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string()),
            );
        }
        ok_json(report)
    }
}

fn call_context_show(
    context: &McpContext,
    root: Option<&str>,
    format: &str,
    payload_key: &str,
    legacy_focus: bool,
) -> Result<CallToolResult, CallToolError> {
    let backlog_dir = match resolve_root(context, root) {
        Ok(dir) => dir,
        Err(err) => return ok_json(err),
    };
    let loaded = load_context_state(&backlog_dir);
    if format == "text" {
        if let Some(ctx) = loaded {
            return ok_text(format!(
                "project_id: {}\nobjective: {}\nscope.mode: {:?}\nscope.epic_id: {}\nscope.task_ids: {}\n",
                ctx.project_id.unwrap_or_else(|| "(none)".into()),
                ctx.objective.unwrap_or_else(|| "(none)".into()),
                ctx.scope.mode,
                ctx.scope.epic_id.unwrap_or_else(|| "(none)".into()),
                if ctx.scope.task_ids.is_empty() {
                    "(empty)".into()
                } else {
                    ctx.scope.task_ids.join(", ")
                }
            ));
        }
        return ok_text(format!("(no {} set)", payload_key));
    }
    let mut payload = serde_json::json!({
        "path": context_path(&backlog_dir)
    });
    payload[payload_key] = if legacy_focus {
        match loaded.as_ref() {
            Some(state) => legacy_focus_payload(state),
            None => serde_json::Value::Null,
        }
    } else {
        serde_json::to_value(&loaded).map_err(|err| CallToolError::from_message(err.to_string()))?
    };
    ok_json(payload)
}

fn call_context_set(
    context: &McpContext,
    root: Option<&str>,
    project_id: Option<String>,
    epic_id: Option<String>,
    objective: Option<String>,
    tasks: Option<ListInput>,
    audit_action: &str,
    payload_key: &str,
    legacy_focus: bool,
) -> Result<CallToolResult, CallToolError> {
    let backlog_dir = match resolve_root(context, root) {
        Ok(dir) => dir,
        Err(err) => return ok_json(err),
    };
    let repo_root = resolve_repo_root(context, root);
    let inferred_project = infer_project_id(&repo_root);
    let task_ids = parse_list_input(tasks);
    let scope = if epic_id
        .as_deref()
        .map(|id| !id.trim().is_empty())
        .unwrap_or(false)
    {
        ContextScope {
            mode: ContextScopeMode::Epic,
            epic_id: epic_id.clone(),
            task_ids: Vec::new(),
        }
    } else if !task_ids.is_empty() {
        ContextScope {
            mode: ContextScopeMode::Tasks,
            epic_id: None,
            task_ids: task_ids.clone(),
        }
    } else {
        ContextScope {
            mode: ContextScopeMode::None,
            epic_id: None,
            task_ids: Vec::new(),
        }
    };
    let state = ContextState {
        version: 1,
        project_id: project_id.or(inferred_project),
        objective,
        scope,
        updated_at: None,
    };
    let path = save_context(&backlog_dir, state.clone())
        .map_err(|err| CallToolError::from_message(err.to_string()))?;
    audit_event(
        &backlog_dir,
        audit_action,
        state.scope.epic_id.as_deref(),
        serde_json::json!({
            "project_id": state.project_id.clone(),
            "objective": state.objective.clone(),
            "scope": state.scope.clone()
        }),
    )?;
    let mut payload = serde_json::json!({
        "ok": true,
        "path": path
    });
    payload[payload_key] = if legacy_focus {
        legacy_focus_payload(&state)
    } else {
        serde_json::to_value(&state).map_err(|err| CallToolError::from_message(err.to_string()))?
    };
    ok_json(payload)
}

fn call_context_clear(
    context: &McpContext,
    root: Option<&str>,
    audit_action: &str,
) -> Result<CallToolResult, CallToolError> {
    let backlog_dir = match resolve_root(context, root) {
        Ok(dir) => dir,
        Err(err) => return ok_json(err),
    };
    let cleared =
        clear_context(&backlog_dir).map_err(|err| CallToolError::from_message(err.to_string()))?;
    if cleared {
        audit_event(&backlog_dir, audit_action, None, serde_json::json!({}))?;
    }
    ok_json(serde_json::json!({"ok": true, "cleared": cleared}))
}

fn legacy_focus_payload(state: &ContextState) -> serde_json::Value {
    let (epic_id, working_set) = match state.scope.mode {
        ContextScopeMode::Epic => (state.scope.epic_id.clone(), Vec::new()),
        ContextScopeMode::Tasks => (None, state.scope.task_ids.clone()),
        ContextScopeMode::None => (None, Vec::new()),
    };
    serde_json::json!({
        "project_id": state.project_id.clone(),
        "epic_id": epic_id,
        "objective": state.objective.clone(),
        "working_set": working_set
    })
}

impl ContextShowTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        call_context_show(
            context,
            self.root.as_deref(),
            &self.format,
            "context",
            false,
        )
    }
}

impl ContextSetTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        call_context_set(
            context,
            self.root.as_deref(),
            self.project_id.clone(),
            self.epic_id.clone(),
            self.objective.clone(),
            self.tasks.clone(),
            "context_set",
            "context",
            false,
        )
    }
}

impl ContextClearTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        call_context_clear(context, self.root.as_deref(), "context_clear")
    }
}

impl FocusShowTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        call_context_show(context, self.root.as_deref(), &self.format, "focus", true)
    }
}

impl FocusSetTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        call_context_set(
            context,
            self.root.as_deref(),
            self.project_id.clone(),
            self.epic_id.clone(),
            self.objective.clone(),
            self.tasks.clone(),
            "focus_set",
            "focus",
            true,
        )
    }
}

impl FocusClearTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        call_context_clear(context, self.root.as_deref(), "focus_clear")
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    absolute.canonicalize().unwrap_or(absolute)
}

fn normalize_path_string(path: &Path) -> String {
    normalize_path(path).to_string_lossy().to_string()
}

fn best_effort_worktree_binding(
    home: &Path,
    cwd: &Path,
    repo_root: Option<&str>,
) -> Option<WorktreeBinding> {
    let registry = find_worktree_record_by_path(home, cwd).ok().flatten();
    let branch =
        current_worktree_branch(cwd).or_else(|| registry.as_ref().and_then(|r| r.branch.clone()));
    let id = registry.as_ref().map(|r| r.id.clone());
    let repo_root_value = registry
        .as_ref()
        .map(|r| r.repo_root.clone())
        .or_else(|| repo_root.map(|value| normalize_path_string(Path::new(value))));
    if branch.is_none() && id.is_none() {
        return None;
    }
    Some(WorktreeBinding {
        id,
        path: normalize_path_string(cwd),
        branch,
        repo_root: repo_root_value,
    })
}

impl WorktreeListTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let repo_root = repo_root_from_backlog(&backlog_dir);
        let home =
            resolve_workmesh_home().map_err(|err| CallToolError::from_message(err.to_string()))?;
        let entries = list_worktree_views(&repo_root, &home)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        if self.format == "text" {
            if entries.is_empty() {
                return ok_text("(no worktrees)".to_string());
            }
            let body = entries
                .iter()
                .map(|entry| {
                    format!(
                        "{} | branch={} | sources={} | issues={}",
                        entry.path,
                        entry.branch.clone().unwrap_or_else(|| "-".to_string()),
                        entry.source.join("+"),
                        if entry.issues.is_empty() {
                            "ok".to_string()
                        } else {
                            entry.issues.join(",")
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            return ok_text(body);
        }
        ok_json(serde_json::json!({
            "repo_root": normalize_path_string(&repo_root),
            "registry_path": workmesh_core::worktrees::worktrees_registry_path(&home),
            "worktrees": entries
        }))
    }
}

impl WorktreeCreateTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let repo_root = repo_root_from_backlog(&backlog_dir);
        let home =
            resolve_workmesh_home().map_err(|err| CallToolError::from_message(err.to_string()))?;

        let created = create_git_worktree(
            &repo_root,
            Path::new(self.path.trim()),
            self.branch.trim(),
            self.from.as_deref(),
        )
        .map_err(|err| CallToolError::from_message(err.to_string()))?;
        let record = upsert_worktree_record(
            &home,
            WorktreeRecord {
                id: String::new(),
                repo_root: normalize_path_string(&repo_root),
                path: created.path.clone(),
                branch: created.branch.clone().or_else(|| Some(self.branch.clone())),
                created_at: String::new(),
                updated_at: String::new(),
                attached_session_id: read_current_session_id(&home),
            },
        )
        .map_err(|err| CallToolError::from_message(err.to_string()))?;

        let should_seed_context = self.project_id.is_some()
            || self.epic_id.is_some()
            || self.objective.is_some()
            || self
                .tasks
                .as_ref()
                .map(|_| !parse_list_input(self.tasks.clone()).is_empty())
                .unwrap_or(false);
        let mut context_seeded = false;
        let mut warnings = Vec::new();
        if should_seed_context {
            let seed_root = normalize_path(Path::new(self.path.trim()));
            match resolve_backlog(&seed_root) {
                Ok(resolution) => {
                    let task_ids = parse_list_input(self.tasks.clone());
                    let inferred_epic = self
                        .epic_id
                        .clone()
                        .or_else(|| extract_task_id_from_branch(self.branch.trim()));
                    let scope = if inferred_epic
                        .as_deref()
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false)
                    {
                        ContextScope {
                            mode: ContextScopeMode::Epic,
                            epic_id: inferred_epic,
                            task_ids: Vec::new(),
                        }
                    } else if !task_ids.is_empty() {
                        ContextScope {
                            mode: ContextScopeMode::Tasks,
                            epic_id: None,
                            task_ids,
                        }
                    } else {
                        ContextScope {
                            mode: ContextScopeMode::None,
                            epic_id: None,
                            task_ids: Vec::new(),
                        }
                    };
                    let _ = save_context(
                        &resolution.backlog_dir,
                        ContextState {
                            version: 1,
                            project_id: self
                                .project_id
                                .clone()
                                .or_else(|| infer_project_id(&seed_root)),
                            objective: self.objective.clone(),
                            scope,
                            updated_at: None,
                        },
                    )
                    .map_err(|err| CallToolError::from_message(err.to_string()))?;
                    context_seeded = true;
                }
                Err(_) => warnings.push(format!(
                    "context seed skipped (no workmesh/tasks found under {})",
                    seed_root.display()
                )),
            }
        }

        ok_json(serde_json::json!({
            "ok": true,
            "worktree": created,
            "registry": record,
            "context_seeded": context_seeded,
            "warnings": warnings
        }))
    }
}

impl WorktreeAttachTool {
    fn call(&self, _context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let home =
            resolve_workmesh_home().map_err(|err| CallToolError::from_message(err.to_string()))?;
        let session_id = self
            .session_id
            .clone()
            .or_else(|| read_current_session_id(&home))
            .ok_or_else(|| CallToolError::from_message("No session id provided"))?;
        let sessions = load_sessions_latest(&home)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        let existing = sessions
            .into_iter()
            .find(|session| session.id == session_id)
            .ok_or_else(|| CallToolError::from_message("Session not found"))?;

        let path = self
            .path
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let existing_record = find_worktree_record_by_path(&home, &path)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        let repo_root = existing
            .repo_root
            .clone()
            .unwrap_or_else(|| normalize_path_string(&path));
        let branch = current_worktree_branch(&path);
        let record = upsert_worktree_record(
            &home,
            WorktreeRecord {
                id: existing_record
                    .as_ref()
                    .map(|record| record.id.clone())
                    .unwrap_or_default(),
                repo_root,
                path: normalize_path_string(&path),
                branch: branch.clone(),
                created_at: existing_record
                    .as_ref()
                    .map(|record| record.created_at.clone())
                    .unwrap_or_default(),
                updated_at: String::new(),
                attached_session_id: Some(session_id),
            },
        )
        .map_err(|err| CallToolError::from_message(err.to_string()))?;
        let binding = WorktreeBinding {
            id: Some(record.id.clone()),
            path: record.path.clone(),
            branch: record.branch.clone().or(branch),
            repo_root: Some(record.repo_root.clone()),
        };
        let mut updated = existing.clone();
        updated.updated_at = now_rfc3339();
        updated.worktree = Some(binding.clone());
        append_session_saved(&home, updated.clone())
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        set_current_session(&home, &updated.id)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        ok_json(serde_json::json!({
            "ok": true,
            "session": updated,
            "worktree": binding
        }))
    }
}

impl WorktreeDetachTool {
    fn call(&self, _context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let home =
            resolve_workmesh_home().map_err(|err| CallToolError::from_message(err.to_string()))?;
        let session_id = self
            .session_id
            .clone()
            .or_else(|| read_current_session_id(&home))
            .ok_or_else(|| CallToolError::from_message("No session id provided"))?;
        let sessions = load_sessions_latest(&home)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        let existing = sessions
            .into_iter()
            .find(|session| session.id == session_id)
            .ok_or_else(|| CallToolError::from_message("Session not found"))?;
        let mut updated = existing.clone();
        updated.updated_at = now_rfc3339();
        updated.worktree = None;
        append_session_saved(&home, updated.clone())
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        set_current_session(&home, &updated.id)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        ok_json(serde_json::to_value(updated).unwrap_or_default())
    }
}

impl WorktreeDoctorTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let repo_root = repo_root_from_backlog(&backlog_dir);
        let home =
            resolve_workmesh_home().map_err(|err| CallToolError::from_message(err.to_string()))?;
        let report = doctor_worktrees(&repo_root, &home)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        if self.format == "text" {
            if report.issues.is_empty() {
                return ok_text("worktrees: ok".to_string());
            }
            let body = report
                .issues
                .iter()
                .map(|issue| format!("- {}", issue))
                .collect::<Vec<_>>()
                .join("\n");
            return ok_text(format!(
                "worktrees: {} issue(s)\n{}",
                report.issues.len(),
                body
            ));
        }
        ok_json(serde_json::to_value(report).unwrap_or_default())
    }
}

impl ListTasksTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = if self.all {
            load_tasks_with_archive(&backlog_dir)
        } else {
            load_tasks(&backlog_dir)
        };
        let status = parse_list_input(self.status.clone());
        let kind = parse_list_input(self.kind.clone());
        let phase = parse_list_input(self.phase.clone());
        let priority = parse_list_input(self.priority.clone());
        let labels = parse_list_input(self.labels.clone());
        let filtered = filter_tasks(
            &tasks,
            if status.is_empty() {
                None
            } else {
                Some(status.as_slice())
            },
            if kind.is_empty() {
                None
            } else {
                Some(kind.as_slice())
            },
            if phase.is_empty() {
                None
            } else {
                Some(phase.as_slice())
            },
            if priority.is_empty() {
                None
            } else {
                Some(priority.as_slice())
            },
            if labels.is_empty() {
                None
            } else {
                Some(labels.as_slice())
            },
            self.depends_on.as_deref(),
            self.deps_satisfied,
            self.blocked,
            self.search.as_deref(),
        );
        let mut sorted = sort_tasks(filtered, &self.sort);
        if let Some(limit) = self.limit {
            sorted.truncate(limit as usize);
        }
        if self.format == "text" {
            let body = sorted
                .iter()
                .map(|task| render_task_line(task))
                .collect::<Vec<_>>()
                .join("\n");
            if self.include_hints {
                let hints = best_practice_hints()
                    .into_iter()
                    .map(|hint| format!("- {}", hint))
                    .collect::<Vec<_>>()
                    .join("\n");
                return ok_text(format!("{}\n\nBest practices:\n{}", body, hints));
            }
            return ok_text(body);
        }
        let tasks_json: Vec<_> = sorted
            .iter()
            .map(|task| task_to_json_value(task, false))
            .collect();
        let payload = if self.include_hints {
            serde_json::json!({"tasks": tasks_json, "hints": best_practice_hints()})
        } else {
            serde_json::Value::Array(tasks_json)
        };
        ok_json(payload)
    }
}

impl ShowTaskTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let task = find_task(&tasks, &self.task_id);
        let Some(task) = task else {
            return ok_json(
                serde_json::json!({"error": format!("Task not found: {}", self.task_id)}),
            );
        };
        if self.format == "text" {
            if let Some(path) = &task.file_path {
                let content = std::fs::read_to_string(path).unwrap_or_default();
                return ok_text(content);
            }
            return ok_text(String::new());
        }
        ok_json(task_to_json_value(task, self.include_body))
    }
}

impl NextTaskTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let context_state = load_context_state(&backlog_dir);
        let recommended = recommend_next_tasks_with_context(&tasks, context_state.as_ref());
        let Some(task) = recommended.first() else {
            return ok_json(serde_json::json!({"error": "No ready tasks"}));
        };
        if self.format == "text" {
            return ok_text(render_task_line(task));
        }
        ok_json(task_to_json_value(task, false))
    }
}

impl NextTasksTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let context_state = load_context_state(&backlog_dir);
        let mut next_tasks = recommend_next_tasks_with_context(&tasks, context_state.as_ref());
        if next_tasks.is_empty() {
            return ok_json(serde_json::json!({"error": "No ready tasks"}));
        }
        let limit = self.limit.unwrap_or(10);
        next_tasks.truncate(limit as usize);

        if self.format == "text" {
            let body = next_tasks
                .iter()
                .map(|task| render_task_line(task))
                .collect::<Vec<_>>()
                .join("\n");
            return ok_text(body);
        }
        let payload: Vec<serde_json::Value> = next_tasks
            .iter()
            .map(|task| task_to_json_value(task, false))
            .collect();
        ok_json(serde_json::Value::Array(payload))
    }
}

impl ReadyTasksTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let mut ready = ready_tasks(&tasks);
        if let Some(limit) = self.limit {
            ready.truncate(limit as usize);
        }
        if self.format == "text" {
            let body = ready
                .iter()
                .map(|task| render_task_line(task))
                .collect::<Vec<_>>()
                .join("\n");
            return ok_text(body);
        }
        let payload: Vec<serde_json::Value> = ready
            .iter()
            .map(|task| task_to_json_value(task, false))
            .collect();
        ok_json(serde_json::Value::Array(payload))
    }
}

impl BoardTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = if self.all {
            load_tasks_with_archive(&backlog_dir)
        } else {
            load_tasks(&backlog_dir)
        };

        let by = match self.by.trim().to_lowercase().as_str() {
            "status" => BoardBy::Status,
            "phase" => BoardBy::Phase,
            "priority" => BoardBy::Priority,
            other => {
                return ok_json(serde_json::json!({
                    "error": format!("Invalid board by: {}", other),
                    "allowed": ["status","phase","priority"]
                }));
            }
        };

        let context_state = if self.focus {
            load_context_state(&backlog_dir)
        } else {
            None
        };
        let scope_ids = context_state
            .as_ref()
            .and_then(|c| scope_ids_from_context(&tasks, c));
        let lanes = board_lanes(&tasks, by, scope_ids.as_ref());

        if self.format == "text" {
            let mut out = String::new();
            for (key, lane_tasks) in lanes {
                out.push_str(&format!("## {} ({})\n", key, lane_tasks.len()));
                for task in lane_tasks {
                    out.push_str(&render_task_line(task));
                    out.push('\n');
                }
                out.push('\n');
            }
            return ok_text(out.trim_end().to_string());
        }

        let payload: Vec<serde_json::Value> = lanes
            .into_iter()
            .map(|(key, lane_tasks)| {
                let tasks_json: Vec<serde_json::Value> = lane_tasks
                    .into_iter()
                    .map(|t| task_to_json_value(t, false))
                    .collect();
                serde_json::json!({
                    "lane": key,
                    "count": tasks_json.len(),
                    "tasks": tasks_json,
                })
            })
            .collect();
        ok_json(serde_json::Value::Array(payload))
    }
}

impl BlockersTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = if self.all {
            load_tasks_with_archive(&backlog_dir)
        } else {
            load_tasks(&backlog_dir)
        };
        let context_state = load_context_state(&backlog_dir);
        let report =
            blockers_report_with_context(&tasks, context_state.as_ref(), self.epic_id.as_deref());

        if self.format == "text" {
            let mut out = String::new();
            out.push_str(&format!("Scope: {}\n", report.scope));
            if !report.warnings.is_empty() {
                out.push_str("Warnings:\n");
                for w in report.warnings.iter() {
                    out.push_str(&format!("- {}\n", w));
                }
            }
            if report.blocked_tasks.is_empty() {
                out.push_str("Blocked tasks: (none)\n");
            } else {
                out.push_str("Blocked tasks:\n");
                for entry in report.blocked_tasks.iter() {
                    let mut parts = Vec::new();
                    if !entry.blockers.is_empty() {
                        parts.push(format!("blocked_by=[{}]", entry.blockers.join(", ")));
                    }
                    if !entry.missing_refs.is_empty() {
                        parts.push(format!("missing_refs=[{}]", entry.missing_refs.join(", ")));
                    }
                    out.push_str(&format!(
                        "- {}: {} ({}) {}\n",
                        entry.id,
                        entry.title,
                        entry.status,
                        parts.join(" ")
                    ));
                }
            }
            if report.top_blockers.is_empty() {
                out.push_str("Top blockers: (none)\n");
            } else {
                out.push_str("Top blockers:\n");
                for b in report.top_blockers.iter().take(10) {
                    out.push_str(&format!("- {} blocks {}\n", b.id, b.blocked_count));
                }
            }
            return ok_text(out.trim_end().to_string());
        }

        ok_json(serde_json::to_value(&report).unwrap_or_else(|_| serde_json::json!({})))
    }
}

impl ExportTasksTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let payload: Vec<_> = tasks
            .iter()
            .map(|task| task_to_json_value(task, self.include_body))
            .collect();
        ok_json(serde_json::Value::Array(payload))
    }
}

impl StatsTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let counts = status_counts(&tasks);
        if self.format == "text" {
            let body = counts
                .iter()
                .map(|(key, value)| format!("{}: {}", key, value))
                .collect::<Vec<_>>()
                .join("\n");
            return ok_text(body);
        }
        let mut map = serde_json::Map::new();
        for (key, value) in counts {
            map.insert(key, serde_json::Value::from(value as u64));
        }
        ok_json(serde_json::Value::Object(map))
    }
}

impl SetStatusTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let task = find_task(&tasks, &self.task_id);
        let Some(task) = task else {
            return ok_json(
                serde_json::json!({"error": format!("Task not found: {}", self.task_id)}),
            );
        };
        if is_done_status(&self.status) {
            if let Err(err) = ensure_can_mark_done(&tasks, task) {
                return ok_json(serde_json::json!({"error": err}));
            }
        }
        let path = task
            .file_path
            .as_ref()
            .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
        update_task_field(path, "status", Some(self.status.clone().into()))
            .map_err(CallToolError::new)?;
        if self.touch || is_done_status(&self.status) {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))
                .map_err(CallToolError::new)?;
        }
        audit_event(
            &backlog_dir,
            "set_status",
            Some(&task.id),
            serde_json::json!({ "status": self.status.clone() }),
        )?;
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(serde_json::json!({"ok": true, "id": task.id, "status": self.status.clone()}))
    }
}

impl SetFieldTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let task = find_task(&tasks, &self.task_id);
        let Some(task) = task else {
            return ok_json(
                serde_json::json!({"error": format!("Task not found: {}", self.task_id)}),
            );
        };
        let path = task
            .file_path
            .as_ref()
            .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
        update_task_field_or_section(path, &self.field, Some(&self.value))
            .map_err(CallToolError::new)?;
        if self.touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))
                .map_err(CallToolError::new)?;
        }
        audit_event(
            &backlog_dir,
            "set_field",
            Some(&task.id),
            serde_json::json!({ "field": self.field.clone(), "value": self.value.clone() }),
        )?;
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(
            serde_json::json!({"ok": true, "id": task.id, "field": self.field.clone(), "value": self.value.clone()}),
        )
    }
}

impl AddLabelTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        update_list_field(
            context,
            self.root.as_deref(),
            &self.task_id,
            "labels",
            &self.label,
            true,
            self.touch,
        )
    }
}

impl RemoveLabelTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        update_list_field(
            context,
            self.root.as_deref(),
            &self.task_id,
            "labels",
            &self.label,
            false,
            self.touch,
        )
    }
}

impl AddDependencyTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        update_list_field(
            context,
            self.root.as_deref(),
            &self.task_id,
            "dependencies",
            &self.dependency,
            true,
            self.touch,
        )
    }
}

impl RemoveDependencyTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        update_list_field(
            context,
            self.root.as_deref(),
            &self.task_id,
            "dependencies",
            &self.dependency,
            false,
            self.touch,
        )
    }
}

impl BulkSetStatusTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let ids = match parse_bulk_ids(self.tasks.clone()) {
            Ok(ids) => ids,
            Err(err) => return ok_json(err),
        };
        let (selected, missing) = select_tasks_with_missing(&tasks, &ids);
        let mut updated = Vec::new();
        for task in selected {
            if is_done_status(&self.status) {
                if let Err(err) = ensure_can_mark_done(&tasks, task) {
                    return ok_json(serde_json::json!({"error": err}));
                }
            }
            let path = task
                .file_path
                .as_ref()
                .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
            update_task_field(path, "status", Some(self.status.clone().into()))
                .map_err(CallToolError::new)?;
            if self.touch || is_done_status(&self.status) {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))
                    .map_err(CallToolError::new)?;
            }
            audit_event(
                &backlog_dir,
                "bulk_set_status",
                Some(&task.id),
                serde_json::json!({ "status": self.status.clone() }),
            )?;
            updated.push(task.id.clone());
        }
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(bulk_result(updated, missing))
    }
}

impl BulkSetFieldTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let ids = match parse_bulk_ids(self.tasks.clone()) {
            Ok(ids) => ids,
            Err(err) => return ok_json(err),
        };
        let (selected, missing) = select_tasks_with_missing(&tasks, &ids);
        let mut updated = Vec::new();
        for task in selected {
            let path = task
                .file_path
                .as_ref()
                .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
            update_task_field_or_section(path, &self.field, Some(&self.value))
                .map_err(CallToolError::new)?;
            if self.touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))
                    .map_err(CallToolError::new)?;
            }
            audit_event(
                &backlog_dir,
                "bulk_set_field",
                Some(&task.id),
                serde_json::json!({ "field": self.field.clone(), "value": self.value.clone() }),
            )?;
            updated.push(task.id.clone());
        }
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(bulk_result(updated, missing))
    }
}

impl BulkAddLabelTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let ids = match parse_bulk_ids(self.tasks.clone()) {
            Ok(ids) => ids,
            Err(err) => return ok_json(err),
        };
        let (selected, missing) = select_tasks_with_missing(&tasks, &ids);
        let mut updated = Vec::new();
        for task in selected {
            let path = task
                .file_path
                .as_ref()
                .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
            let mut current = task.labels.clone();
            if !current.contains(&self.label) {
                current.push(self.label.clone());
            }
            set_list_field(path, "labels", current).map_err(CallToolError::new)?;
            if self.touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))
                    .map_err(CallToolError::new)?;
            }
            audit_event(
                &backlog_dir,
                "bulk_label_add",
                Some(&task.id),
                serde_json::json!({ "label": self.label.clone() }),
            )?;
            updated.push(task.id.clone());
        }
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(bulk_result(updated, missing))
    }
}

impl BulkRemoveLabelTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let ids = match parse_bulk_ids(self.tasks.clone()) {
            Ok(ids) => ids,
            Err(err) => return ok_json(err),
        };
        let (selected, missing) = select_tasks_with_missing(&tasks, &ids);
        let mut updated = Vec::new();
        for task in selected {
            let path = task
                .file_path
                .as_ref()
                .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
            let mut current = task.labels.clone();
            current.retain(|entry| entry != &self.label);
            set_list_field(path, "labels", current).map_err(CallToolError::new)?;
            if self.touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))
                    .map_err(CallToolError::new)?;
            }
            audit_event(
                &backlog_dir,
                "bulk_label_remove",
                Some(&task.id),
                serde_json::json!({ "label": self.label.clone() }),
            )?;
            updated.push(task.id.clone());
        }
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(bulk_result(updated, missing))
    }
}

impl BulkAddDependencyTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let ids = match parse_bulk_ids(self.tasks.clone()) {
            Ok(ids) => ids,
            Err(err) => return ok_json(err),
        };
        let (selected, missing) = select_tasks_with_missing(&tasks, &ids);
        let mut updated = Vec::new();
        for task in selected {
            let path = task
                .file_path
                .as_ref()
                .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
            let mut current = task.dependencies.clone();
            if !current.contains(&self.dependency) {
                current.push(self.dependency.clone());
            }
            set_list_field(path, "dependencies", current).map_err(CallToolError::new)?;
            if self.touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))
                    .map_err(CallToolError::new)?;
            }
            audit_event(
                &backlog_dir,
                "bulk_dependency_add",
                Some(&task.id),
                serde_json::json!({ "dependency": self.dependency.clone() }),
            )?;
            updated.push(task.id.clone());
        }
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(bulk_result(updated, missing))
    }
}

impl BulkRemoveDependencyTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let ids = match parse_bulk_ids(self.tasks.clone()) {
            Ok(ids) => ids,
            Err(err) => return ok_json(err),
        };
        let (selected, missing) = select_tasks_with_missing(&tasks, &ids);
        let mut updated = Vec::new();
        for task in selected {
            let path = task
                .file_path
                .as_ref()
                .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
            let mut current = task.dependencies.clone();
            current.retain(|entry| entry != &self.dependency);
            set_list_field(path, "dependencies", current).map_err(CallToolError::new)?;
            if self.touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))
                    .map_err(CallToolError::new)?;
            }
            audit_event(
                &backlog_dir,
                "bulk_dependency_remove",
                Some(&task.id),
                serde_json::json!({ "dependency": self.dependency.clone() }),
            )?;
            updated.push(task.id.clone());
        }
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(bulk_result(updated, missing))
    }
}

impl BulkAddNoteTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let ids = match parse_bulk_ids(self.tasks.clone()) {
            Ok(ids) => ids,
            Err(err) => return ok_json(err),
        };
        let (selected, missing) = select_tasks_with_missing(&tasks, &ids);
        let mut updated = Vec::new();
        for task in selected {
            let path = task
                .file_path
                .as_ref()
                .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
            let new_body = append_note(&task.body, &self.note, &self.section);
            update_body(path, &new_body).map_err(CallToolError::new)?;
            if self.touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))
                    .map_err(CallToolError::new)?;
            }
            audit_event(
                &backlog_dir,
                "bulk_note",
                Some(&task.id),
                serde_json::json!({ "section": self.section.clone(), "note": self.note.clone() }),
            )?;
            updated.push(task.id.clone());
        }
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(bulk_result(updated, missing))
    }
}

impl ArchiveTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let before = parse_before_date(&self.before)?;
        let result = archive_tasks(
            &backlog_dir,
            &tasks,
            &ArchiveOptions {
                before,
                status: self.status.clone(),
            },
        )
        .map_err(CallToolError::new)?;
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(serde_json::json!({
            "archived": result.archived,
            "skipped": result.skipped,
            "archive_dir": result.archive_dir
        }))
    }
}

impl MigrateTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let resolution = resolve_backlog(&repo_root).map_err(CallToolError::new)?;
        let target = self
            .to
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("workmesh");
        let result = migrate_backlog(&resolution, target).map_err(CallToolError::new)?;
        ok_json(serde_json::json!({
            "from": result.from,
            "to": result.to
        }))
    }
}

impl MigrateAuditTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let report = audit_deprecations(&repo_root).map_err(CallToolError::new)?;
        ok_json(serde_json::to_value(report).unwrap_or_default())
    }
}

impl MigratePlanTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let report = audit_deprecations(&repo_root).map_err(CallToolError::new)?;
        let include = parse_list_input(self.include.clone());
        let exclude = parse_list_input(self.exclude.clone());
        let plan = plan_migrations(&report, &MigrationPlanOptions { include, exclude });
        ok_json(serde_json::to_value(plan).unwrap_or_default())
    }
}

impl MigrateApplyTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let report = audit_deprecations(&repo_root).map_err(CallToolError::new)?;
        let include = parse_list_input(self.include.clone());
        let exclude = parse_list_input(self.exclude.clone());
        let plan = plan_migrations(&report, &MigrationPlanOptions { include, exclude });
        let result = apply_migration_plan(
            &repo_root,
            &plan,
            &MigrationApplyOptions {
                dry_run: !self.apply,
                backup: self.backup,
            },
        )
        .map_err(CallToolError::new)?;
        ok_json(serde_json::to_value(result).unwrap_or_default())
    }
}

impl ClaimTaskTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let task = find_task(&tasks, &self.task_id);
        let Some(task) = task else {
            return ok_json(
                serde_json::json!({"error": format!("Task not found: {}", self.task_id)}),
            );
        };
        let path = task
            .file_path
            .as_ref()
            .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
        let mut assignee = task.assignee.clone();
        if !assignee.iter().any(|value| value == &self.owner) {
            assignee.push(self.owner.clone());
            set_list_field(path, "assignee", assignee).map_err(CallToolError::new)?;
        }
        let expires_at = self.minutes.map(timestamp_plus_minutes);
        let lease = Lease {
            owner: self.owner.clone(),
            acquired_at: Some(now_timestamp()),
            expires_at,
        };
        update_lease_fields(path, Some(&lease)).map_err(CallToolError::new)?;
        if self.touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))
                .map_err(CallToolError::new)?;
        }
        audit_event(
            &backlog_dir,
            "claim",
            Some(&task.id),
            serde_json::json!({
                "owner": lease.owner.clone(),
                "expires_at": lease.expires_at.clone(),
            }),
        )?;
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(serde_json::json!({"ok": true, "id": task.id, "owner": lease.owner.clone()}))
    }
}

impl ReleaseTaskTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let task = find_task(&tasks, &self.task_id);
        let Some(task) = task else {
            return ok_json(
                serde_json::json!({"error": format!("Task not found: {}", self.task_id)}),
            );
        };
        let path = task
            .file_path
            .as_ref()
            .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
        update_lease_fields(path, None).map_err(CallToolError::new)?;
        if self.touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))
                .map_err(CallToolError::new)?;
        }
        audit_event(
            &backlog_dir,
            "release",
            Some(&task.id),
            serde_json::json!({}),
        )?;
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(serde_json::json!({"ok": true, "id": task.id}))
    }
}

impl AddNoteTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let task = find_task(&tasks, &self.task_id);
        let Some(task) = task else {
            return ok_json(
                serde_json::json!({"error": format!("Task not found: {}", self.task_id)}),
            );
        };
        let path = task
            .file_path
            .as_ref()
            .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
        let section_key = if self.section == "notes" {
            "notes"
        } else {
            "impl"
        };
        let new_body = append_note(&task.body, &self.note, section_key);
        update_body(path, &new_body).map_err(CallToolError::new)?;
        if self.touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))
                .map_err(CallToolError::new)?;
        }
        audit_event(
            &backlog_dir,
            "note",
            Some(&task.id),
            serde_json::json!({ "section": self.section.clone(), "note": self.note.clone() }),
        )?;
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(serde_json::json!({"ok": true, "id": task.id, "section": self.section}))
    }
}

impl SetBodyTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let task = find_task(&tasks, &self.task_id);
        let Some(task) = task else {
            return ok_json(
                serde_json::json!({"error": format!("Task not found: {}", self.task_id)}),
            );
        };
        let path = task
            .file_path
            .as_ref()
            .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
        update_body(path, &self.body).map_err(CallToolError::new)?;
        if self.touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))
                .map_err(CallToolError::new)?;
        }
        audit_event(
            &backlog_dir,
            "set_body",
            Some(&task.id),
            serde_json::json!({ "length": self.body.len() }),
        )?;
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(serde_json::json!({"ok": true, "id": task.id}))
    }
}

impl SetSectionTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let task = find_task(&tasks, &self.task_id);
        let Some(task) = task else {
            return ok_json(
                serde_json::json!({"error": format!("Task not found: {}", self.task_id)}),
            );
        };
        let path = task
            .file_path
            .as_ref()
            .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
        let new_body = replace_section(&task.body, &self.section, &self.content);
        update_body(path, &new_body).map_err(CallToolError::new)?;
        if self.touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))
                .map_err(CallToolError::new)?;
        }
        audit_event(
            &backlog_dir,
            "set_section",
            Some(&task.id),
            serde_json::json!({ "section": self.section.clone(), "length": self.content.len() }),
        )?;
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(serde_json::json!({"ok": true, "id": task.id, "section": self.section}))
    }
}

impl AddTaskTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let tasks_dir = backlog_dir.join("tasks");
        let task_id = match self.task_id.clone() {
            Some(value) => value,
            None => {
                let repo_root = repo_root_from_backlog(&backlog_dir);
                let branch = core_git_branch(&repo_root).unwrap_or_else(|| "work".to_string());
                let initiative = ensure_branch_initiative(&repo_root, &branch)
                    .map_err(|e| CallToolError::from_message(e.to_string()))?;
                next_namespaced_task_id(&tasks, &initiative)
            }
        };
        let labels = parse_list_input(self.labels.clone());
        let dependencies = parse_list_input(self.dependencies.clone());
        let assignee = parse_list_input(self.assignee.clone());
        let path = create_task_file(
            &tasks_dir,
            &task_id,
            &self.title,
            &self.status,
            &self.priority,
            &self.phase,
            &dependencies,
            &labels,
            &assignee,
        )
        .map_err(CallToolError::new)?;
        audit_event(
            &backlog_dir,
            "add_task",
            Some(&task_id),
            serde_json::json!({ "title": self.title.clone() }),
        )?;
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        let mut hints = best_practice_hints();
        if dependencies.is_empty() {
            let mut enriched = vec![
                "No dependencies were provided.",
                "If this task is blocked by other work, add dependencies now.",
            ];
            enriched.extend(hints);
            hints = enriched;
        }
        ok_json(serde_json::json!({
            "ok": true,
            "id": task_id,
            "path": path,
            "hints": hints,
            "next_steps": [
                "Add dependencies with add_dependency if this task is blocked.",
                "Add labels for better filtering.",
                "Add a note if there is important context.",
            ]
        }))
    }
}

impl AddDiscoveredTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let tasks_dir = backlog_dir.join("tasks");
        let task_id = match self.task_id.clone() {
            Some(value) => value,
            None => {
                let repo_root = repo_root_from_backlog(&backlog_dir);
                let branch = core_git_branch(&repo_root).unwrap_or_else(|| "work".to_string());
                let initiative = ensure_branch_initiative(&repo_root, &branch)
                    .map_err(|e| CallToolError::from_message(e.to_string()))?;
                next_namespaced_task_id(&tasks, &initiative)
            }
        };
        let labels = parse_list_input(self.labels.clone());
        let dependencies = parse_list_input(self.dependencies.clone());
        let assignee = parse_list_input(self.assignee.clone());
        let path = create_task_file(
            &tasks_dir,
            &task_id,
            &self.title,
            &self.status,
            &self.priority,
            &self.phase,
            &dependencies,
            &labels,
            &assignee,
        )
        .map_err(CallToolError::new)?;
        update_task_field(
            &path,
            "discovered_from",
            Some(FieldValue::List(vec![self.from.clone()])),
        )
        .map_err(CallToolError::new)?;
        audit_event(
            &backlog_dir,
            "add_discovered",
            Some(&task_id),
            serde_json::json!({ "from": self.from.clone(), "title": self.title.clone() }),
        )?;
        refresh_index_best_effort(&backlog_dir);
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(serde_json::json!({
            "ok": true,
            "id": task_id,
            "path": path,
            "from": self.from.clone(),
        }))
    }
}

impl ProjectInitTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let repo_root = repo_root_from_backlog(&backlog_dir);
        let path = ensure_project_docs(&repo_root, &self.project_id, self.name.as_deref())
            .map_err(CallToolError::new)?;
        audit_event(
            &backlog_dir,
            "project_init",
            None,
            serde_json::json!({ "project_id": self.project_id.clone() }),
        )?;
        maybe_auto_checkpoint(&backlog_dir);
        ok_json(serde_json::json!({
            "ok": true,
            "project_id": self.project_id,
            "path": path,
        }))
    }
}

impl QuickstartTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let result = quickstart(
            &repo_root,
            &self.project_id,
            self.name.as_deref(),
            self.agents_snippet,
        )
        .map_err(CallToolError::new)?;
        ok_json(serde_json::to_value(result).unwrap_or_default())
    }
}

impl ValidateTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let report = validate_tasks(&tasks, Some(&backlog_dir));
        ok_json(serde_json::to_value(report).unwrap_or_default())
    }
}

impl FixIdsTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let report =
            fix_duplicate_task_ids(&backlog_dir, &tasks, FixIdsOptions { apply: self.apply })
                .map_err(CallToolError::new)?;

        if self.apply {
            audit_event(
                &backlog_dir,
                "fix_ids",
                None,
                serde_json::json!({ "changes": report.changes.len() }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir);
        }

        ok_json(serde_json::json!({
            "ok": true,
            "apply": self.apply,
            "changes": report.changes.iter().map(|c| serde_json::json!({
                "old_id": c.old_id,
                "new_id": c.new_id,
                "old_path": c.old_path,
                "new_path": c.new_path,
                "uid": c.uid,
            })).collect::<Vec<_>>(),
            "warnings": report.warnings,
        }))
    }
}

impl RekeyPromptTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let prompt = render_rekey_prompt(
            &backlog_dir,
            RekeyPromptOptions {
                include_body: self.include_body,
                include_archive: self.all,
                limit: self.limit.map(|v| v as usize),
            },
        );
        if self.format == "json" {
            ok_json(serde_json::json!({ "ok": true, "prompt": prompt }))
        } else {
            ok_text(prompt)
        }
    }
}

impl RekeyApplyTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let request = parse_rekey_request(&self.mapping_json).map_err(CallToolError::new)?;
        let report = rekey_apply(
            &backlog_dir,
            &request,
            RekeyApplyOptions {
                apply: self.apply,
                strict: request.strict,
                include_archive: self.all,
            },
        )
        .map_err(CallToolError::new)?;

        if self.apply {
            audit_event(
                &backlog_dir,
                "rekey_apply",
                None,
                serde_json::json!({ "changes": report.changes.len(), "strict": request.strict }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir);
        }

        ok_json(serde_json::to_value(report).unwrap_or_default())
    }
}

impl GraphExportTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let graph = graph_export(&tasks);
        if self.pretty {
            ok_text(serde_json::to_string_pretty(&graph).unwrap_or_else(|_| "{}".to_string()))
        } else {
            ok_text(serde_json::to_string(&graph).unwrap_or_else(|_| "{}".to_string()))
        }
    }
}

impl IssuesExportTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let payload = tasks_to_jsonl(&tasks, self.include_body);
        ok_text(payload)
    }
}

impl IndexRebuildTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let summary = rebuild_index(&backlog_dir).map_err(CallToolError::new)?;
        ok_json(serde_json::to_value(summary).unwrap_or_default())
    }
}

impl IndexRefreshTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let summary = refresh_index(&backlog_dir).map_err(CallToolError::new)?;
        ok_json(serde_json::to_value(summary).unwrap_or_default())
    }
}

impl IndexVerifyTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let report = verify_index(&backlog_dir).map_err(CallToolError::new)?;
        ok_json(serde_json::to_value(report).unwrap_or_default())
    }
}

impl CheckpointTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let options = CheckpointOptions {
            project_id: self.project.clone(),
            checkpoint_id: self.id.clone(),
            audit_limit: self.audit_limit.unwrap_or(20) as usize,
        };
        let result =
            write_checkpoint(&backlog_dir, &tasks, &options).map_err(CallToolError::new)?;
        if self.format == "text" {
            return ok_text(format!(
                "Checkpoint: {}\nJSON: {}\nMarkdown: {}",
                result.snapshot.checkpoint_id,
                result.json_path.display(),
                result.markdown_path.display()
            ));
        }
        ok_json(serde_json::to_value(result.snapshot).unwrap_or_default())
    }
}

impl ResumeTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let repo_root = repo_root_from_backlog(&backlog_dir);
        let project_id = resolve_project_id(&repo_root, &tasks, self.project.as_deref());
        let summary = resume_summary(&repo_root, &project_id, self.id.as_deref())
            .map_err(CallToolError::new)?;
        let Some(summary) = summary else {
            return ok_text("No checkpoint found".to_string());
        };
        if self.format == "text" {
            return ok_text(render_resume(&summary));
        }
        ok_json(serde_json::to_value(summary.snapshot).unwrap_or_default())
    }
}

impl WorkingSetTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let repo_root = repo_root_from_backlog(&backlog_dir);
        let project_id = resolve_project_id(&repo_root, &tasks, self.project.as_deref());
        let selected = match self.tasks.clone() {
            Some(input) => {
                let ids = parse_list_input(Some(input));
                select_tasks_by_ids(&tasks, &ids)
            }
            None => tasks
                .iter()
                .filter(|task| task.status.eq_ignore_ascii_case("in progress"))
                .collect(),
        };
        let summaries: Vec<_> = selected.iter().map(|task| task_summary(task)).collect();
        let path = write_working_set(&repo_root, &project_id, &summaries, self.note.as_deref())
            .map_err(CallToolError::new)?;
        ok_json(serde_json::json!({"path": path}))
    }
}

impl SessionJournalTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let repo_root = repo_root_from_backlog(&backlog_dir);
        let project_id = resolve_project_id(&repo_root, &tasks, self.project.as_deref());
        let path = append_session_journal(
            &repo_root,
            &project_id,
            self.task.as_deref(),
            self.next.as_deref(),
            self.note.as_deref(),
        )
        .map_err(CallToolError::new)?;
        ok_json(serde_json::json!({"path": path}))
    }
}

impl CheckpointDiffTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let repo_root = repo_root_from_backlog(&backlog_dir);
        let project_id = resolve_project_id(&repo_root, &tasks, self.project.as_deref());
        let summary = resume_summary(&repo_root, &project_id, self.id.as_deref())
            .map_err(CallToolError::new)?;
        let Some(summary) = summary else {
            return ok_text("No checkpoint found".to_string());
        };
        let report = diff_since_checkpoint(&repo_root, &backlog_dir, &tasks, &summary.snapshot);
        if self.format == "text" {
            return ok_text(render_diff(&report));
        }
        ok_json(serde_json::to_value(report).unwrap_or_default())
    }
}

impl SessionSaveTool {
    fn call(&self, _context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let home =
            resolve_workmesh_home().map_err(|err| CallToolError::from_message(err.to_string()))?;

        let cwd = self
            .cwd
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let cwd_str = cwd.to_string_lossy().to_string();

        let tasks_override = parse_list_input(self.tasks.clone());

        let mut repo_root: Option<String> = None;
        let mut project_id: Option<String> = self.project.clone();
        let mut epic_id: Option<String> = None;
        let mut working_set: Vec<String> = tasks_override;
        let mut git: Option<GitSnapshot> = None;
        let mut checkpoint: Option<CheckpointRef> = None;
        let mut recent_changes: Option<RecentChanges> = None;

        if let Ok(backlog_dir) = locate_backlog_dir(&cwd) {
            let rr = repo_root_from_backlog(&backlog_dir);
            repo_root = Some(rr.to_string_lossy().to_string());
            let repo_tasks = load_tasks(&backlog_dir);
            epic_id = load_context_state(&backlog_dir).and_then(|c| c.scope.epic_id);

            if project_id.is_none() {
                project_id = Some(resolve_project_id(
                    &rr,
                    &repo_tasks,
                    self.project.as_deref(),
                ));
            }

            if working_set.is_empty() {
                working_set = repo_tasks
                    .iter()
                    .filter(|task| {
                        task.status.eq_ignore_ascii_case("in progress") || is_lease_active(task)
                    })
                    .map(|task| task.id.clone())
                    .collect();
            }

            git = Some(best_effort_git_snapshot(&rr));
            if let Some(pid) = project_id.as_deref() {
                if let Ok(Some(summary)) = resume_summary(&rr, pid, None) {
                    checkpoint = Some(CheckpointRef {
                        path: summary.checkpoint_path.to_string_lossy().to_string(),
                        timestamp: Some(summary.snapshot.generated_at.clone()),
                    });
                    recent_changes = Some(RecentChanges {
                        dirs: summary.snapshot.top_level_dirs.clone(),
                        files: summary.snapshot.changed_files.clone(),
                    });
                }
            }
        }

        let now = now_rfc3339();
        if epic_id.is_none() {
            if let Some(branch) = git.as_ref().and_then(|g| g.branch.as_deref()) {
                epic_id = extract_task_id_from_branch(branch);
            }
        }
        let session = AgentSession {
            worktree: best_effort_worktree_binding(&home, &cwd, repo_root.as_deref()),
            id: new_session_id(),
            created_at: now.clone(),
            updated_at: now,
            cwd: cwd_str,
            repo_root,
            project_id,
            epic_id,
            objective: self.objective.clone(),
            working_set,
            notes: self.notes.clone(),
            git,
            checkpoint,
            recent_changes,
            handoff: None,
        };

        append_session_saved(&home, session.clone())
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        set_current_session(&home, &session.id)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;

        if self.format == "text" {
            return ok_text(format!("Saved session {}", session.id));
        }
        ok_json(serde_json::to_value(session).unwrap_or_default())
    }
}

impl SessionListTool {
    fn call(&self, _context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let home =
            resolve_workmesh_home().map_err(|err| CallToolError::from_message(err.to_string()))?;
        let mut sessions = load_sessions_latest(&home)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        if let Some(limit) = self.limit {
            sessions.truncate(limit as usize);
        }
        if self.format == "text" {
            if sessions.is_empty() {
                return ok_text("(no sessions)".to_string());
            }
            let body = sessions
                .iter()
                .map(render_session_line)
                .collect::<Vec<_>>()
                .join("\n");
            return ok_text(body);
        }
        ok_json(serde_json::to_value(sessions).unwrap_or_default())
    }
}

impl SessionShowTool {
    fn call(&self, _context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let home =
            resolve_workmesh_home().map_err(|err| CallToolError::from_message(err.to_string()))?;
        let sessions = load_sessions_latest(&home)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        let session = sessions
            .into_iter()
            .find(|s| s.id == self.session_id)
            .ok_or_else(|| CallToolError::from_message("Session not found"))?;
        if self.format == "text" {
            return ok_text(render_session_detail(&session));
        }
        ok_json(serde_json::to_value(session).unwrap_or_default())
    }
}

impl SessionResumeTool {
    fn call(&self, _context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let home =
            resolve_workmesh_home().map_err(|err| CallToolError::from_message(err.to_string()))?;
        let id = self
            .session_id
            .clone()
            .or_else(|| read_current_session_id(&home))
            .ok_or_else(|| CallToolError::from_message("No session id provided"))?;
        let sessions = load_sessions_latest(&home)
            .map_err(|err| CallToolError::from_message(err.to_string()))?;
        let session = sessions
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| CallToolError::from_message("Session not found"))?;
        let script = resume_script(&session);
        if self.format == "text" {
            let mut body = render_session_detail(&session);
            body.push_str("\n\nSuggested resume:\n");
            body.push_str(&script.join("\n"));
            return ok_text(body);
        }
        ok_json(serde_json::json!({ "session": session, "resume_script": script }))
    }
}

impl BestPracticesTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        if resolve_root(context, self.root.as_deref()).is_err() {
            return ok_json(serde_json::json!({"error": ROOT_REQUIRED_ERROR}));
        }
        if self.format == "json" {
            return ok_json(serde_json::json!({
                "best_practices": best_practice_hints()
            }));
        }
        let body = best_practice_hints()
            .into_iter()
            .map(|hint| format!("- {}", hint))
            .collect::<Vec<_>>()
            .join("\n");
        ok_text(body)
    }
}

impl GanttTextTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let text = plantuml_gantt(&tasks, self.start.as_deref(), None, self.zoom, None, true);
        ok_text(text)
    }
}

impl GanttFileTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let text = plantuml_gantt(&tasks, self.start.as_deref(), None, self.zoom, None, true);
        let path = write_text_file(Path::new(&self.output), &text).map_err(CallToolError::new)?;
        ok_json(serde_json::json!({"ok": true, "path": path}))
    }
}

impl GanttSvgTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let text = plantuml_gantt(&tasks, self.start.as_deref(), None, self.zoom, None, true);
        let cmd = match &self.plantuml_cmd {
            Some(cmd) => Some(parse_command_string(cmd)?),
            None => None,
        };
        let jar_path = self.plantuml_jar.as_ref().map(Path::new);
        let svg = match render_plantuml_svg(&text, cmd, jar_path, None) {
            Ok(svg) => svg,
            Err(err) => {
                return ok_json(serde_json::json!({"error": err.to_string()}));
            }
        };
        if let Some(output) = &self.output {
            let path = write_text_file(Path::new(output), &svg).map_err(CallToolError::new)?;
            return ok_json(serde_json::json!({"ok": true, "path": path}));
        }
        ok_text(svg)
    }
}

fn best_effort_git_snapshot(repo_root: &Path) -> GitSnapshot {
    let branch = std::process::Command::new("git")
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
        .filter(|value| !value.trim().is_empty());

    let head_sha = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-parse")
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
        .filter(|value| !value.trim().is_empty());

    let dirty = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("status")
        .arg("--porcelain")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(!String::from_utf8_lossy(&out.stdout).trim().is_empty())
            } else {
                None
            }
        });

    GitSnapshot {
        branch,
        head_sha,
        dirty,
    }
}

fn render_session_line(session: &AgentSession) -> String {
    format!(
        "{} | {} | {} | {}",
        session.id, session.updated_at, session.cwd, session.objective
    )
}

fn render_session_detail(session: &AgentSession) -> String {
    let mut lines = Vec::new();
    lines.push(format!("id: {}", session.id));
    lines.push(format!("updated_at: {}", session.updated_at));
    lines.push(format!("cwd: {}", session.cwd));
    if let Some(repo_root) = session.repo_root.as_deref() {
        lines.push(format!("repo_root: {}", repo_root));
    }
    if let Some(project_id) = session.project_id.as_deref() {
        lines.push(format!("project_id: {}", project_id));
    }
    if let Some(epic_id) = session.epic_id.as_deref() {
        lines.push(format!("epic_id: {}", epic_id));
    }
    lines.push(format!("objective: {}", session.objective));
    if !session.working_set.is_empty() {
        lines.push(format!("working_set: {}", session.working_set.join(", ")));
    }
    if let Some(notes) = session.notes.as_deref() {
        if !notes.trim().is_empty() {
            lines.push(format!("notes: {}", notes));
        }
    }
    if let Some(git) = session.git.as_ref() {
        if git.branch.is_some() || git.head_sha.is_some() || git.dirty.is_some() {
            lines.push(format!(
                "git: branch={} sha={} dirty={}",
                git.branch.clone().unwrap_or_else(|| "-".to_string()),
                git.head_sha.clone().unwrap_or_else(|| "-".to_string()),
                git.dirty
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string())
            ));
        }
    }
    if let Some(checkpoint) = session.checkpoint.as_ref() {
        lines.push(format!("checkpoint: {}", checkpoint.path));
    }
    if let Some(worktree) = session.worktree.as_ref() {
        lines.push(format!("worktree.path: {}", worktree.path));
        if let Some(branch) = worktree.branch.as_deref() {
            lines.push(format!("worktree.branch: {}", branch));
        }
        if let Some(id) = worktree.id.as_deref() {
            lines.push(format!("worktree.id: {}", id));
        }
    }
    if let Some(handoff) = session.handoff.as_ref() {
        if !handoff.completed.is_empty() {
            lines.push(format!(
                "handoff.completed: {}",
                handoff.completed.join(" | ")
            ));
        }
        if !handoff.remaining.is_empty() {
            lines.push(format!(
                "handoff.remaining: {}",
                handoff.remaining.join(" | ")
            ));
        }
        if !handoff.decisions.is_empty() {
            lines.push(format!(
                "handoff.decisions: {}",
                handoff.decisions.join(" | ")
            ));
        }
        if !handoff.unknowns.is_empty() {
            lines.push(format!(
                "handoff.unknowns: {}",
                handoff.unknowns.join(" | ")
            ));
        }
        if let Some(next_step) = handoff.next_step.as_deref() {
            if !next_step.trim().is_empty() {
                lines.push(format!("handoff.next_step: {}", next_step));
            }
        }
    }
    lines.join("\n")
}

fn resume_script(session: &AgentSession) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(worktree) = session.worktree.as_ref() {
        lines.push(format!("cd {}", worktree.path));
    } else {
        lines.push(format!("cd {}", session.cwd));
    }

    if let Some(repo_root) = session.repo_root.as_deref() {
        lines.push(format!("workmesh --root {} context show", repo_root));
    }

    if let (Some(repo_root), Some(project_id)) =
        (session.repo_root.as_deref(), session.project_id.as_deref())
    {
        lines.push(format!(
            "workmesh --root {} resume --project {}",
            repo_root, project_id
        ));
        lines.push(format!("workmesh --root {} ready", repo_root));
    }

    if let Some(repo_root) = session.repo_root.as_deref() {
        if let Some(task_id) = session.working_set.first() {
            lines.push(format!("workmesh --root {} show {}", repo_root, task_id));
            lines.push(format!(
                "workmesh --root {} claim {} you",
                repo_root, task_id
            ));
        }
    }

    lines
}

fn parse_command_string(raw: &str) -> Result<Vec<String>, CallToolError> {
    // `shell_words` is Unix-shell oriented and treats backslashes as escapes, which breaks Windows
    // strings like `cmd /C C:\path\plantuml.cmd`. Keep parsing predictable on Windows.
    if cfg!(windows) {
        Ok(raw
            .split_whitespace()
            .map(|part| part.to_string())
            .collect())
    } else {
        shell_words::split(raw).map_err(CallToolError::new)
    }
}

impl SkillContentTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let name = self
            .name
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("workmesh");
        let skill = match read_skill_content(&repo_root, name) {
            Ok(result) => result,
            Err(err) => return ok_json(err),
        };
        if self.format == "json" {
            return ok_json(serde_json::json!({
                "name": name,
                "source": skill.source,
                "content": skill.content,
            }));
        }
        ok_text(skill.content)
    }
}

impl HelpTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        if resolve_root(context, self.root.as_deref()).is_err() {
            return ok_json(serde_json::json!({"error": ROOT_REQUIRED_ERROR}));
        }
        if self.format == "json" {
            let payload = serde_json::json!({
                "summary": "workmesh MCP help",
                "best_practices": best_practice_hints(),
                "tools": tool_catalog(),
                "notes": [
                    "root is optional if the server is started inside a repo with workmesh/tasks, .workmesh/tasks, tasks/, or legacy backlog/tasks",
                    "Dependencies are first-class. Use them to model blockers.",
                    "Use validate to catch missing or broken dependencies.",
                    "List-style arguments accept CSV strings or JSON arrays.",
                    format!("Task kind is free-form (not enforced). Suggested kinds: {}", recommended_kinds().join(", ")),
                ]
            });
            return ok_json(payload);
        }
        let catalog = tool_catalog()
            .iter()
            .map(|tool| {
                let name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let summary = tool.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                format!("- {}: {}", name, summary)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let hint_lines = best_practice_hints()
            .into_iter()
            .map(|hint| format!("- {}", hint))
            .collect::<Vec<_>>()
            .join("\n");
        let body = format!(
            "workmesh MCP help\n\nPurpose:\n  Manage Markdown-backed backlogs with first-class dependencies.\n\nRoot resolution:\n  - MCP tools can infer root from CWD.\n\nTask kind:\n  - Free-form (not enforced). Suggested: {}\n\nAvailable tools:\n{}\n\nBest practices:\n{}\n",
            recommended_kinds().join(", "),
            catalog,
            hint_lines
        );
        ok_text(body)
    }
}

fn sdk_tool_definitions() -> Vec<serde_json::Value> {
    WorkmeshTools::tools()
        .into_iter()
        .filter_map(|tool| serde_json::to_value(tool).ok())
        .collect()
}

fn sdk_tool_definition(name: &str) -> Option<serde_json::Value> {
    let needle = name.trim();
    if needle.is_empty() {
        return None;
    }
    sdk_tool_definitions().into_iter().find(|value| {
        value
            .get("name")
            .and_then(|v| v.as_str())
            .map(|n| n == needle)
            .unwrap_or(false)
    })
}

fn tool_examples(name: &str) -> Vec<serde_json::Value> {
    let name = name.trim();
    match name {
        "version" => vec![serde_json::json!({
            "tool": "version",
            "arguments": { "format": "json" }
        })],
        "list_tasks" => vec![serde_json::json!({
            "tool": "list_tasks",
            "arguments": {
                "status": ["To Do"],
                "kind": ["bug"],
                "sort": "id",
                "format": "json"
            }
        })],
        "show_task" => vec![serde_json::json!({
            "tool": "show_task",
            "arguments": { "task_id": "task-001", "format": "json", "include_body": true }
        })],
        "next_task" => vec![serde_json::json!({
            "tool": "next_task",
            "arguments": { "format": "json" }
        })],
        "ready_tasks" => vec![serde_json::json!({
            "tool": "ready_tasks",
            "arguments": { "format": "json", "limit": 10 }
        })],
        "worktree_list" => vec![serde_json::json!({
            "tool": "worktree_list",
            "arguments": { "format": "json" }
        })],
        "worktree_create" => vec![serde_json::json!({
            "tool": "worktree_create",
            "arguments": {
                "path": "../repo-feature-a",
                "branch": "feature/a",
                "project_id": "demo",
                "objective": "Implement feature A",
                "format": "json"
            }
        })],
        "worktree_attach" => vec![serde_json::json!({
            "tool": "worktree_attach",
            "arguments": { "path": "../repo-feature-a", "format": "json" }
        })],
        "worktree_detach" => vec![serde_json::json!({
            "tool": "worktree_detach",
            "arguments": { "format": "json" }
        })],
        "worktree_doctor" => vec![serde_json::json!({
            "tool": "worktree_doctor",
            "arguments": { "format": "json" }
        })],
        "set_status" => vec![serde_json::json!({
            "tool": "set_status",
            "arguments": { "task_id": "task-001", "status": "In Progress", "touch": true }
        })],
        "set_field" => vec![serde_json::json!({
            "tool": "set_field",
            "arguments": { "task_id": "task-001", "field": "kind", "value": "bug", "touch": true }
        })],
        "add_task" => vec![serde_json::json!({
            "tool": "add_task",
            "arguments": { "title": "Investigate flaky test", "priority": "P2", "phase": "Phase1" }
        })],
        "add_discovered" => vec![serde_json::json!({
            "tool": "add_discovered",
            "arguments": { "from": "task-001", "title": "New edge case discovered", "priority": "P2", "phase": "Phase1" }
        })],
        "graph_export" => vec![serde_json::json!({
            "tool": "graph_export",
            "arguments": { "pretty": true }
        })],
        "issues_export" => vec![serde_json::json!({
            "tool": "issues_export",
            "arguments": { "include_body": false }
        })],
        "index_rebuild" => vec![serde_json::json!({
            "tool": "index_rebuild",
            "arguments": {}
        })],
        "checkpoint" => vec![serde_json::json!({
            "tool": "checkpoint",
            "arguments": { "project": "workmesh", "json": true }
        })],
        "resume" => vec![serde_json::json!({
            "tool": "resume",
            "arguments": { "project": "workmesh", "json": true }
        })],
        "help" => vec![serde_json::json!({
            "tool": "help",
            "arguments": { "format": "json" }
        })],
        "tool_info" => vec![serde_json::json!({
            "tool": "tool_info",
            "arguments": { "name": "list_tasks", "format": "text" }
        })],
        _ => vec![serde_json::json!({
            "tool": name,
            "arguments": {}
        })],
    }
}

fn tool_info_payload(name: &str) -> Option<serde_json::Value> {
    let name = name.trim();
    let tool_def = sdk_tool_definition(name)?;
    let summary = tool_catalog()
        .into_iter()
        .find(|tool| {
            tool.get("name")
                .and_then(|v| v.as_str())
                .map(|n| n == name)
                .unwrap_or(false)
        })
        .and_then(|tool| {
            tool.get("summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    let mut notes = vec![
        "root is optional if the server is started inside a repo with workmesh/tasks, .workmesh/tasks, tasks/, or legacy backlog/tasks".to_string(),
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

impl ToolInfoTool {
    fn call(&self, _context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let name = self.name.trim();
        let Some(info) = tool_info_payload(name) else {
            let available = sdk_tool_definitions()
                .into_iter()
                .filter_map(|tool| {
                    tool.get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect::<Vec<_>>();
            return ok_json(
                serde_json::json!({ "error": format!("Unknown tool: {}", name), "available": available }),
            );
        };

        if self.format == "json" {
            return ok_json(info);
        }
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
        let body = format!(
            "Tool: {name}\n\nSummary:\n  {summary}\n\nTool definition:\n{tool_def}\n\nExamples:\n{examples}\n\nNotes:\n{notes}\n",
            name = name,
            summary = summary,
            tool_def = serde_json::to_string_pretty(&tool_def).unwrap_or_default(),
            examples = examples,
            notes = notes
        );
        ok_text(body)
    }
}

impl ProjectManagementSkillTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let skill_name = self
            .name
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("workmesh-mcp");
        let skill = match read_skill_content(&repo_root, skill_name) {
            Ok(result) => result,
            Err(err) => return ok_json(err),
        };
        if self.format == "json" {
            return ok_json(serde_json::json!({
                "summary": "workmesh project management skill",
                "name": skill_name,
                "source": skill.source,
                "content": skill.content,
            }));
        }
        ok_text(skill.content)
    }
}

fn update_list_field(
    context: &McpContext,
    root: Option<&str>,
    task_id: &str,
    field: &str,
    value: &str,
    add: bool,
    touch: bool,
) -> Result<CallToolResult, CallToolError> {
    let backlog_dir = match resolve_root(context, root) {
        Ok(dir) => dir,
        Err(err) => return ok_json(err),
    };
    let tasks = load_tasks(&backlog_dir);
    let task = find_task(&tasks, task_id);
    let Some(task) = task else {
        return ok_json(serde_json::json!({"error": format!("Task not found: {}", task_id)}));
    };
    let path = task
        .file_path
        .as_ref()
        .ok_or_else(|| CallToolError::from_message("Missing task path"))?;
    let mut current = match field {
        "labels" => task.labels.clone(),
        "dependencies" => task.dependencies.clone(),
        _ => Vec::new(),
    };
    let value = value.trim();
    if add {
        if !current.contains(&value.to_string()) {
            current.push(value.to_string());
        }
    } else {
        current.retain(|entry| entry != value);
    }
    set_list_field(path, field, current.clone()).map_err(CallToolError::new)?;
    if touch {
        update_task_field(path, "updated_date", Some(now_timestamp().into()))
            .map_err(CallToolError::new)?;
    }
    let action = match (field, add) {
        ("labels", true) => "label_add",
        ("labels", false) => "label_remove",
        ("dependencies", true) => "dependency_add",
        ("dependencies", false) => "dependency_remove",
        _ => "update_list",
    };
    audit_event(
        &backlog_dir,
        action,
        Some(&task.id),
        serde_json::json!({ "field": field, "value": value, "add": add }),
    )?;
    refresh_index_best_effort(&backlog_dir);
    maybe_auto_checkpoint(&backlog_dir);
    let payload = if field == "labels" {
        serde_json::json!({"ok": true, "id": task.id, "labels": current})
    } else {
        serde_json::json!({"ok": true, "id": task.id, "dependencies": current})
    };
    ok_json(payload)
}

fn find_task<'a>(tasks: &'a [Task], task_id: &str) -> Option<&'a Task> {
    let target = task_id.to_lowercase();
    tasks.iter().find(|task| task.id.to_lowercase() == target)
}

fn select_tasks_by_ids<'a>(tasks: &'a [Task], ids: &[String]) -> Vec<&'a Task> {
    let mut selected = Vec::new();
    for id in ids {
        if let Some(task) = find_task(tasks, id) {
            selected.push(task);
        }
    }
    selected
}

fn select_tasks_with_missing<'a>(
    tasks: &'a [Task],
    ids: &[String],
) -> (Vec<&'a Task>, Vec<String>) {
    let mut selected = Vec::new();
    let mut missing = Vec::new();
    for id in ids {
        if let Some(task) = find_task(tasks, id) {
            selected.push(task);
        } else {
            missing.push(id.to_string());
        }
    }
    (selected, missing)
}

fn normalize_task_ids(ids: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for id in ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn parse_bulk_ids(input: Option<ListInput>) -> Result<Vec<String>, serde_json::Value> {
    let ids = normalize_task_ids(parse_list_input(input));
    if ids.is_empty() {
        return Err(serde_json::json!({"error": "tasks required"}));
    }
    Ok(ids)
}

fn bulk_result(updated: Vec<String>, missing: Vec<String>) -> serde_json::Value {
    serde_json::json!({
        "ok": missing.is_empty(),
        "updated": updated,
        "missing": missing,
    })
}

fn auto_checkpoint_enabled() -> bool {
    std::env::var("WORKMESH_AUTO_CHECKPOINT")
        .ok()
        .map(|value| value.trim().to_lowercase())
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn auto_session_enabled() -> bool {
    std::env::var("WORKMESH_AUTO_SESSION")
        .ok()
        .map(|value| value.trim().to_lowercase())
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn maybe_auto_checkpoint(backlog_dir: &Path) {
    let tasks = load_tasks(backlog_dir);
    if auto_checkpoint_enabled() {
        let options = CheckpointOptions {
            project_id: None,
            checkpoint_id: None,
            audit_limit: 10,
        };
        let _ = write_checkpoint(backlog_dir, &tasks, &options);
    }

    if auto_session_enabled() {
        let _ = auto_update_current_session(backlog_dir, &tasks);
    }
}

fn auto_update_current_session(backlog_dir: &Path, tasks: &[Task]) -> Result<(), anyhow::Error> {
    let home = resolve_workmesh_home()?;
    let Some(current_id) = read_current_session_id(&home) else {
        return Ok(());
    };
    let sessions = load_sessions_latest(&home)?;
    let Some(existing) = sessions.into_iter().find(|s| s.id == current_id) else {
        return Ok(());
    };

    let rr = repo_root_from_backlog(backlog_dir);
    let repo_root = rr.to_string_lossy().to_string();
    let project_id = resolve_project_id(&rr, tasks, None);
    let epic_id = load_context_state(backlog_dir).and_then(|c| c.scope.epic_id);

    let working_set: Vec<String> = tasks
        .iter()
        .filter(|task| task.status.eq_ignore_ascii_case("in progress") || is_lease_active(task))
        .map(|task| task.id.clone())
        .collect();

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cwd_str = cwd.to_string_lossy().to_string();

    let mut checkpoint: Option<CheckpointRef> = None;
    let mut recent_changes: Option<RecentChanges> = None;
    if let Ok(Some(summary)) = resume_summary(&rr, &project_id, None) {
        checkpoint = Some(CheckpointRef {
            path: summary.checkpoint_path.to_string_lossy().to_string(),
            timestamp: Some(summary.snapshot.generated_at.clone()),
        });
        recent_changes = Some(RecentChanges {
            dirs: summary.snapshot.top_level_dirs.clone(),
            files: summary.snapshot.changed_files.clone(),
        });
    }

    let now = now_rfc3339();
    let worktree = best_effort_worktree_binding(&home, &cwd, Some(repo_root.as_str()))
        .or(existing.worktree.clone());
    let updated = AgentSession {
        id: existing.id.clone(),
        created_at: existing.created_at.clone(),
        updated_at: now,
        cwd: cwd_str,
        repo_root: Some(repo_root),
        project_id: Some(project_id),
        epic_id: epic_id.or(existing.epic_id.clone()),
        objective: existing.objective.clone(),
        working_set,
        notes: existing.notes.clone(),
        git: Some(best_effort_git_snapshot(&rr)),
        checkpoint,
        recent_changes,
        handoff: existing.handoff.clone(),
        worktree,
    };

    append_session_saved(&home, updated.clone())?;
    set_current_session(&home, &updated.id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    fn text_payload(result: CallToolResult) -> String {
        result
            .content
            .first()
            .expect("tool content")
            .as_text_content()
            .expect("text content")
            .text
            .clone()
    }

    fn write_task(tasks_dir: &Path, id: &str, title: &str, status: &str) {
        let filename = format!("{} - {}.md", id, title.to_lowercase());
        let path = tasks_dir.join(filename);
        let content = format!(
            "---\n\
id: {id}\n\
title: {title}\n\
kind: task\n\
status: {status}\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
---\n\n\
Body\n",
            id = id,
            title = title,
            status = status
        );
        std::fs::write(path, content).expect("write task");
    }

    fn write_task_with_meta(
        tasks_dir: &Path,
        id: &str,
        title: &str,
        status: &str,
        priority: &str,
        phase: &str,
    ) {
        let filename = format!("{} - {}.md", id, title.to_lowercase());
        let path = tasks_dir.join(filename);
        let content = format!(
            "---\n\
id: {id}\n\
title: {title}\n\
kind: task\n\
status: {status}\n\
priority: {priority}\n\
phase: {phase}\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
---\n\n\
Body\n",
            id = id,
            title = title,
            status = status,
            priority = priority,
            phase = phase
        );
        std::fs::write(path, content).expect("write task");
    }

    fn init_repo() -> (TempDir, String, McpContext) {
        let temp = TempDir::new().expect("tempdir");
        let repo_root = temp.path().to_path_buf();

        // Minimal docs scaffold so tools that look for docs don't fail.
        std::fs::create_dir_all(
            repo_root
                .join("docs")
                .join("projects")
                .join("alpha")
                .join("updates"),
        )
        .expect("docs");

        // WorkMesh layout.
        let tasks_dir = repo_root.join("workmesh").join("tasks");
        std::fs::create_dir_all(&tasks_dir).expect("tasks");

        let root_arg = repo_root.to_string_lossy().to_string();
        let context = McpContext {
            default_root: Some(repo_root.clone()),
        };
        (temp, root_arg, context)
    }

    #[test]
    fn mcp_list_tasks_all_includes_archive() {
        let (temp, root_arg, context) = init_repo();
        let tasks_dir = temp.path().join("workmesh").join("tasks");
        let archive_dir = temp.path().join("workmesh").join("archive").join("2026-02");
        std::fs::create_dir_all(&archive_dir).expect("archive");

        write_task(&tasks_dir, "task-001", "Active", "To Do");
        write_task(&archive_dir, "task-002", "Archived", "Done");

        let list_active = ListTasksTool {
            root: Some(root_arg.clone()),
            all: false,
            status: None,
            kind: None,
            phase: None,
            priority: None,
            labels: None,
            depends_on: None,
            deps_satisfied: None,
            blocked: None,
            search: None,
            sort: "id".to_string(),
            limit: None,
            format: "json".to_string(),
            include_hints: false,
        }
        .call(&context)
        .expect("list");
        let parsed: serde_json::Value =
            serde_json::from_str(&text_payload(list_active)).expect("json");
        let ids: Vec<_> = parsed
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.get("id").unwrap().as_str().unwrap().to_string())
            .collect();
        assert!(ids.contains(&"task-001".to_string()));
        assert!(!ids.contains(&"task-002".to_string()));

        let list_all = ListTasksTool {
            all: true,
            ..ListTasksTool {
                root: Some(root_arg),
                all: false,
                status: None,
                kind: None,
                phase: None,
                priority: None,
                labels: None,
                depends_on: None,
                deps_satisfied: None,
                blocked: None,
                search: None,
                sort: "id".to_string(),
                limit: None,
                format: "json".to_string(),
                include_hints: false,
            }
        }
        .call(&context)
        .expect("list all");
        let parsed: serde_json::Value =
            serde_json::from_str(&text_payload(list_all)).expect("json");
        let ids: Vec<_> = parsed
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.get("id").unwrap().as_str().unwrap().to_string())
            .collect();
        assert!(ids.contains(&"task-001".to_string()));
        assert!(ids.contains(&"task-002".to_string()));
    }

    #[test]
    fn mcp_readme_returns_readme_json() {
        let (temp, root_arg, context) = init_repo();
        let repo_root = temp.path().to_path_buf();
        let readme_path = repo_root.join("README.json");
        std::fs::write(&readme_path, "{\"name\":\"WorkMesh\",\"tagline\":\"test\"}")
            .expect("write readme");

        let result = ReadmeTool {
            root: Some(root_arg),
            format: "json".to_string(),
        }
        .call(&context)
        .expect("readme");

        let parsed: serde_json::Value = serde_json::from_str(&text_payload(result)).expect("json");
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["readme"]["name"], "WorkMesh");
    }

    #[test]
    fn mcp_next_tasks_returns_ordered_candidates() {
        let (temp, root_arg, context) = init_repo();
        let tasks_dir = temp.path().join("workmesh").join("tasks");

        write_task_with_meta(&tasks_dir, "task-010", "Low", "To Do", "P3", "Phase2");
        write_task_with_meta(&tasks_dir, "task-002", "High", "To Do", "P1", "Phase2");
        write_task_with_meta(
            &tasks_dir,
            "task-001",
            "HighPhase1",
            "To Do",
            "P1",
            "Phase1",
        );
        // Context: explicit task scope should win even if otherwise lower priority.
        let context_path = temp.path().join("workmesh").join("context.json");
        std::fs::write(
            &context_path,
            r#"{"version":1,"project_id":null,"objective":null,"scope":{"mode":"tasks","epic_id":null,"task_ids":["task-010"]}}"#,
        )
        .expect("write context");

        let result = NextTasksTool {
            root: Some(root_arg),
            format: "json".to_string(),
            limit: None,
        }
        .call(&context)
        .expect("next_tasks");

        let parsed: serde_json::Value = serde_json::from_str(&text_payload(result)).expect("json");
        let ids: Vec<String> = parsed
            .as_array()
            .expect("array")
            .iter()
            .filter_map(|v| v.get("id").and_then(|x| x.as_str()).map(|s| s.to_string()))
            .collect();
        assert_eq!(ids[0], "task-010");
        assert_eq!(ids[1], "task-001");
        assert_eq!(ids[2], "task-002");
    }

    #[test]
    fn mcp_set_status_mutates_task_and_touches_by_default() {
        let (temp, root_arg, context) = init_repo();
        let tasks_dir = temp.path().join("workmesh").join("tasks");
        write_task(&tasks_dir, "task-001", "Active", "To Do");

        let tool = SetStatusTool {
            task_id: "task-001".to_string(),
            status: "In Progress".to_string(),
            root: Some(root_arg),
            touch: true,
        };
        let _ = tool.call(&context).expect("set status");

        let listed = ListTasksTool {
            root: Some(temp.path().to_string_lossy().to_string()),
            all: false,
            status: None,
            kind: None,
            phase: None,
            priority: None,
            labels: None,
            depends_on: None,
            deps_satisfied: None,
            blocked: None,
            search: None,
            sort: "id".to_string(),
            limit: None,
            format: "json".to_string(),
            include_hints: false,
        }
        .call(&context)
        .expect("list");
        let parsed: serde_json::Value = serde_json::from_str(&text_payload(listed)).expect("json");
        let task = parsed
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v.get("id").unwrap().as_str().unwrap() == "task-001")
            .expect("task");
        assert_eq!(task.get("status").unwrap().as_str().unwrap(), "In Progress");
        // updated_date is the "touched" field.
        assert!(task.get("updated_date").unwrap().as_str().is_some());
    }

    #[test]
    fn mcp_add_task_creates_markdown_file() {
        let (temp, root_arg, context) = init_repo();
        let tool = AddTaskTool {
            title: "New task".to_string(),
            root: Some(root_arg),
            task_id: None,
            status: "To Do".to_string(),
            priority: "P2".to_string(),
            phase: "Phase1".to_string(),
            labels: None,
            dependencies: None,
            assignee: None,
        };
        let result = tool.call(&context).expect("add task");
        let created: serde_json::Value = serde_json::from_str(&text_payload(result)).expect("json");
        assert!(created.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));

        // Ensure it shows up in list.
        let listed = ListTasksTool {
            root: Some(temp.path().to_string_lossy().to_string()),
            all: false,
            status: None,
            kind: None,
            phase: None,
            priority: None,
            labels: None,
            depends_on: None,
            deps_satisfied: None,
            blocked: None,
            search: Some("New task".to_string()),
            sort: "id".to_string(),
            limit: None,
            format: "json".to_string(),
            include_hints: false,
        }
        .call(&context)
        .expect("list");
        let parsed: serde_json::Value = serde_json::from_str(&text_payload(listed)).expect("json");
        assert!(!parsed.as_array().unwrap().is_empty());
    }

    #[test]
    fn mcp_doctor_returns_layout_and_context() {
        let (temp, root_arg, context) = init_repo();
        let tasks_dir = temp.path().join("workmesh").join("tasks");
        write_task(&tasks_dir, "task-001", "Seed", "To Do");

        // Context.
        let context_path = temp.path().join("workmesh").join("context.json");
        std::fs::write(
            &context_path,
            r#"{"version":1,"project_id":"demo","objective":"Ship","scope":{"mode":"epic","epic_id":"task-001","task_ids":[]}}"#,
        )
        .expect("write context");

        // Derived index file.
        let index_dir = temp.path().join("workmesh").join(".index");
        std::fs::create_dir_all(&index_dir).expect("mkdir index");
        std::fs::write(index_dir.join("tasks.jsonl"), "{\"id\":\"task-001\"}\n").expect("index");

        let tool = DoctorTool {
            root: Some(root_arg),
            format: "json".to_string(),
        };
        let result = tool.call(&context).expect("doctor");
        let parsed: serde_json::Value = serde_json::from_str(&text_payload(result)).expect("json");
        assert_eq!(parsed["layout"].as_str(), Some("workmesh"));
        assert_eq!(parsed["context"]["project_id"].as_str(), Some("demo"));
        assert_eq!(parsed["index"]["present"].as_bool(), Some(true));
        assert_eq!(parsed["index"]["entries"].as_i64(), Some(1));
    }

    #[test]
    fn mcp_board_can_scope_to_context_task_scope() {
        let (temp, root_arg, context) = init_repo();
        let tasks_dir = temp.path().join("workmesh").join("tasks");
        write_task(&tasks_dir, "task-001", "A", "To Do");
        write_task(&tasks_dir, "task-002", "B", "To Do");
        write_task(&tasks_dir, "task-003", "C", "In Progress");

        // Context: scope to task-003 only (no epic scope).
        let context_path = temp.path().join("workmesh").join("context.json");
        std::fs::write(
            &context_path,
            r#"{"version":1,"project_id":"demo","objective":null,"scope":{"mode":"tasks","epic_id":null,"task_ids":["task-003"]}}"#,
        )
        .expect("write context");

        let tool = BoardTool {
            root: Some(root_arg),
            all: false,
            by: "status".to_string(),
            focus: true,
            format: "json".to_string(),
        };
        let result = tool.call(&context).expect("board");
        let parsed: serde_json::Value = serde_json::from_str(&text_payload(result)).expect("json");
        let mut all_ids: Vec<String> = Vec::new();
        for lane in parsed.as_array().unwrap().iter() {
            let Some(tasks) = lane.get("tasks").and_then(|t| t.as_array()) else {
                continue;
            };
            for task in tasks.iter() {
                if let Some(id) = task.get("id").and_then(|v| v.as_str()) {
                    all_ids.push(id.to_string());
                }
            }
        }
        assert_eq!(all_ids, vec!["task-003".to_string()]);
    }

    #[test]
    fn mcp_blockers_scopes_to_context_epic() {
        let (temp, root_arg, context) = init_repo();
        let tasks_dir = temp.path().join("workmesh").join("tasks");

        // Write an epic, a blocker, and a child blocked by the blocker.
        std::fs::write(
            tasks_dir.join("task-100 - epic.md"),
            "---\n\
id: task-100\n\
title: Epic\n\
kind: epic\n\
status: In Progress\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
relationships:\n\
  parent: []\n\
  blocked_by: []\n\
  child: []\n\
  discovered_from: []\n\
---\n\n",
        )
        .expect("epic");
        std::fs::write(
            tasks_dir.join("task-101 - child.md"),
            "---\n\
id: task-101\n\
title: Child\n\
kind: task\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: [task-102]\n\
labels: []\n\
assignee: []\n\
relationships:\n\
  parent: [task-100]\n\
  blocked_by: [task-102]\n\
  child: []\n\
  discovered_from: []\n\
---\n\n",
        )
        .expect("child");
        std::fs::write(
            tasks_dir.join("task-102 - blocker.md"),
            "---\n\
id: task-102\n\
title: Blocker\n\
kind: task\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
labels: []\n\
assignee: []\n\
relationships:\n\
  parent: [task-100]\n\
  blocked_by: []\n\
  child: []\n\
  discovered_from: []\n\
---\n\n",
        )
        .expect("blocker");
        // Another task outside the epic, also blocked by task-102; should be excluded when scoped.
        std::fs::write(
            tasks_dir.join("task-200 - other.md"),
            "---\n\
id: task-200\n\
title: Other\n\
kind: task\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: [task-102]\n\
labels: []\n\
assignee: []\n\
---\n\n",
        )
        .expect("other");

        // Context epic.
        let context_path = temp.path().join("workmesh").join("context.json");
        std::fs::write(
            &context_path,
            r#"{"version":1,"project_id":"demo","objective":null,"scope":{"mode":"epic","epic_id":"task-100","task_ids":[]}}"#,
        )
        .expect("write context");

        let tool = BlockersTool {
            root: Some(root_arg),
            all: false,
            epic_id: None,
            format: "json".to_string(),
        };
        let result = tool.call(&context).expect("blockers");
        let parsed: serde_json::Value = serde_json::from_str(&text_payload(result)).expect("json");
        let blocked = parsed
            .get("blocked_tasks")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(blocked.len(), 1);
        assert_eq!(
            blocked[0].get("id").and_then(|v| v.as_str()),
            Some("task-101")
        );
    }
}
