use std::path::{Path, PathBuf};

use async_trait::async_trait;
use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{
    schema_utils::CallToolError, CallToolRequestParams, CallToolResult, ListToolsResult,
    PaginatedRequestParams, RpcError, TextContent,
};
use rust_mcp_sdk::tool_box;
use rust_mcp_sdk::{mcp_server::ServerHandler, McpServer};
use serde::{Deserialize, Serialize};

use workmesh_core::audit::{append_audit_event, AuditEvent};
use workmesh_core::backlog::{locate_backlog_dir, resolve_backlog_dir, BacklogError};
use workmesh_core::project::{ensure_project_docs, repo_root_from_backlog};
use workmesh_core::quickstart::quickstart;
use workmesh_core::gantt::{plantuml_gantt, render_plantuml_svg, write_text_file};
use workmesh_core::index::{rebuild_index, refresh_index, verify_index};
use workmesh_core::task::{load_tasks, Lease, Task};
use workmesh_core::task_ops::{
    append_note, create_task_file, filter_tasks, graph_export, next_task, ready_tasks,
    now_timestamp, timestamp_plus_minutes,
    render_task_line, replace_section, set_list_field, sort_tasks, status_counts,
    FieldValue, task_to_json_value, tasks_to_jsonl, update_body, update_lease_fields, update_task_field,
    update_task_field_or_section, validate_tasks,
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

fn resolve_root(
    context: &McpContext,
    root: Option<&str>,
) -> Result<PathBuf, serde_json::Value> {
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
                Err(serde_json::json!({"error": format!("No tasks found under {}", root_path.display())}))
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

fn read_skill_content(repo_root: &Path, name: &str) -> Result<(PathBuf, String), serde_json::Value> {
    let path = repo_root
        .join(".codex")
        .join("skills")
        .join(name)
        .join("SKILL.md");
    if !path.exists() {
        return Err(serde_json::json!({
            "error": format!("Skill not found: {}", name),
            "path": path.to_string_lossy(),
        }));
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|err| serde_json::json!({"error": format!("Failed to read skill: {}", err)}))?;
    Ok((path, content))
}

fn ok_text(content: String) -> Result<CallToolResult, CallToolError> {
    Ok(CallToolResult::text_content(vec![TextContent::from(
        content,
    )]))
}

fn ok_json(value: serde_json::Value) -> Result<CallToolResult, CallToolError> {
    let text = serde_json::to_string_pretty(&value)
        .unwrap_or_else(|_| "{}".to_string());
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
    ]
}

fn tool_catalog() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({"name": "list_tasks", "summary": "List tasks with filters and sorting."}),
        serde_json::json!({"name": "show_task", "summary": "Show a single task by id."}),
        serde_json::json!({"name": "next_task", "summary": "Get the next ready task (lowest id, deps satisfied)."}),
        serde_json::json!({"name": "ready_tasks", "summary": "List tasks with deps satisfied (ready work)."}),
        serde_json::json!({"name": "set_status", "summary": "Update task status."}),
        serde_json::json!({"name": "set_field", "summary": "Update a front matter field."}),
        serde_json::json!({"name": "add_label", "summary": "Add a label to a task."}),
        serde_json::json!({"name": "remove_label", "summary": "Remove a label from a task."}),
        serde_json::json!({"name": "add_dependency", "summary": "Add a dependency to a task."}),
        serde_json::json!({"name": "remove_dependency", "summary": "Remove a dependency from a task."}),
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
        serde_json::json!({"name": "gantt_text", "summary": "Return PlantUML gantt text."}),
        serde_json::json!({"name": "gantt_file", "summary": "Write PlantUML gantt to a file."}),
        serde_json::json!({"name": "gantt_svg", "summary": "Render gantt SVG via PlantUML."}),
        serde_json::json!({"name": "help", "summary": "Show available tools and best practices."}),
        serde_json::json!({"name": "skill_content", "summary": "Return SKILL.md content for a repo skill."}),
        serde_json::json!({"name": "project_management_skill", "summary": "Return project management guide."}),
    ]
}

#[mcp_tool(name = "list_tasks", description = "List tasks with optional filters.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListTasksTool {
    pub root: Option<String>,
    pub status: Option<ListInput>,
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

#[mcp_tool(name = "next_task", description = "Return the next ready task.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NextTaskTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[mcp_tool(name = "ready_tasks", description = "List ready tasks (deps satisfied, status To Do).")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadyTasksTool {
    pub root: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
    pub limit: Option<u32>,
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
    #[serde(default)]
    pub touch: bool,
}

#[mcp_tool(name = "set_field", description = "Set a front matter field value.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetFieldTool {
    pub task_id: String,
    pub field: String,
    pub value: String,
    pub root: Option<String>,
    #[serde(default)]
    pub touch: bool,
}

#[mcp_tool(name = "add_label", description = "Add a label to a task.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddLabelTool {
    pub task_id: String,
    pub label: String,
    pub root: Option<String>,
    #[serde(default)]
    pub touch: bool,
}

#[mcp_tool(name = "remove_label", description = "Remove a label from a task.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RemoveLabelTool {
    pub task_id: String,
    pub label: String,
    pub root: Option<String>,
    #[serde(default)]
    pub touch: bool,
}

#[mcp_tool(name = "add_dependency", description = "Add a dependency to a task.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddDependencyTool {
    pub task_id: String,
    pub dependency: String,
    pub root: Option<String>,
    #[serde(default)]
    pub touch: bool,
}

#[mcp_tool(name = "remove_dependency", description = "Remove a dependency from a task.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RemoveDependencyTool {
    pub task_id: String,
    pub dependency: String,
    pub root: Option<String>,
    #[serde(default)]
    pub touch: bool,
}

#[mcp_tool(name = "claim_task", description = "Claim a task lease.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ClaimTaskTool {
    pub task_id: String,
    pub owner: String,
    pub root: Option<String>,
    pub minutes: Option<i64>,
    #[serde(default)]
    pub touch: bool,
}

#[mcp_tool(name = "release_task", description = "Release a task lease.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReleaseTaskTool {
    pub task_id: String,
    pub root: Option<String>,
    #[serde(default)]
    pub touch: bool,
}

#[mcp_tool(name = "add_note", description = "Append a note to Notes or Implementation Notes.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddNoteTool {
    pub task_id: String,
    pub note: String,
    pub root: Option<String>,
    #[serde(default = "default_notes_section")]
    pub section: String,
    #[serde(default)]
    pub touch: bool,
}

#[mcp_tool(name = "set_body", description = "Replace full task body (all content after front matter).")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetBodyTool {
    pub task_id: String,
    pub body: String,
    pub root: Option<String>,
    #[serde(default)]
    pub touch: bool,
}

#[mcp_tool(name = "set_section", description = "Replace a named section in the task body.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetSectionTool {
    pub task_id: String,
    pub section: String,
    pub content: String,
    pub root: Option<String>,
    #[serde(default)]
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

#[mcp_tool(name = "add_discovered", description = "Create a task discovered from another task.")]
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

#[mcp_tool(name = "quickstart", description = "Scaffold docs + backlog + seed task.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct QuickstartTool {
    pub project_id: String,
    pub root: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub agents_snippet: bool,
}

#[mcp_tool(name = "validate", description = "Validate task metadata and dependencies.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ValidateTool {
    pub root: Option<String>,
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

#[mcp_tool(name = "gantt_text", description = "Return PlantUML gantt text for current tasks.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GanttTextTool {
    pub root: Option<String>,
    pub start: Option<String>,
    #[serde(default = "default_zoom")]
    pub zoom: i32,
}

#[mcp_tool(name = "gantt_file", description = "Write PlantUML gantt text to a file and return the path.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GanttFileTool {
    pub output: String,
    pub root: Option<String>,
    pub start: Option<String>,
    #[serde(default = "default_zoom")]
    pub zoom: i32,
}

#[mcp_tool(name = "gantt_svg", description = "Render gantt SVG via PlantUML; return SVG or a file path.")]
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

#[mcp_tool(name = "skill_content", description = "Return SKILL.md content for a repo skill.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SkillContentTool {
    pub root: Option<String>,
    pub name: Option<String>,
    #[serde(default = "default_text_format")]
    pub format: String,
}

#[mcp_tool(name = "help", description = "Show available tools and best practices.")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct HelpTool {
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
    #[serde(default = "default_text_format")]
    pub format: String,
}

fn default_sort() -> String {
    "id".to_string()
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

// Generates enum WorkmeshTools with variants for each tool
tool_box!(
    WorkmeshTools,
    [
        ListTasksTool,
        ShowTaskTool,
        NextTaskTool,
        ReadyTasksTool,
        StatsTool,
        SetStatusTool,
        SetFieldTool,
        AddLabelTool,
        RemoveLabelTool,
        AddDependencyTool,
        RemoveDependencyTool,
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
        GraphExportTool,
        IssuesExportTool,
        IndexRebuildTool,
        IndexRefreshTool,
        IndexVerifyTool,
        GanttTextTool,
        GanttFileTool,
        GanttSvgTool,
        SkillContentTool,
        HelpTool,
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
            WorkmeshTools::ListTasksTool(tool) => tool.call(&self.context),
            WorkmeshTools::ShowTaskTool(tool) => tool.call(&self.context),
            WorkmeshTools::NextTaskTool(tool) => tool.call(&self.context),
            WorkmeshTools::ReadyTasksTool(tool) => tool.call(&self.context),
            WorkmeshTools::StatsTool(tool) => tool.call(&self.context),
            WorkmeshTools::SetStatusTool(tool) => tool.call(&self.context),
            WorkmeshTools::SetFieldTool(tool) => tool.call(&self.context),
            WorkmeshTools::AddLabelTool(tool) => tool.call(&self.context),
            WorkmeshTools::RemoveLabelTool(tool) => tool.call(&self.context),
            WorkmeshTools::AddDependencyTool(tool) => tool.call(&self.context),
            WorkmeshTools::RemoveDependencyTool(tool) => tool.call(&self.context),
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
            WorkmeshTools::GraphExportTool(tool) => tool.call(&self.context),
            WorkmeshTools::IssuesExportTool(tool) => tool.call(&self.context),
            WorkmeshTools::IndexRebuildTool(tool) => tool.call(&self.context),
            WorkmeshTools::IndexRefreshTool(tool) => tool.call(&self.context),
            WorkmeshTools::IndexVerifyTool(tool) => tool.call(&self.context),
            WorkmeshTools::GanttTextTool(tool) => tool.call(&self.context),
            WorkmeshTools::GanttFileTool(tool) => tool.call(&self.context),
            WorkmeshTools::GanttSvgTool(tool) => tool.call(&self.context),
            WorkmeshTools::SkillContentTool(tool) => tool.call(&self.context),
            WorkmeshTools::HelpTool(tool) => tool.call(&self.context),
            WorkmeshTools::ProjectManagementSkillTool(tool) => tool.call(&self.context),
        }
    }
}

impl ListTasksTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let status = parse_list_input(self.status.clone());
        let phase = parse_list_input(self.phase.clone());
        let priority = parse_list_input(self.priority.clone());
        let labels = parse_list_input(self.labels.clone());
        let filtered = filter_tasks(
            &tasks,
            if status.is_empty() { None } else { Some(status.as_slice()) },
            if phase.is_empty() { None } else { Some(phase.as_slice()) },
            if priority.is_empty() { None } else { Some(priority.as_slice()) },
            if labels.is_empty() { None } else { Some(labels.as_slice()) },
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
            return ok_json(serde_json::json!({"error": format!("Task not found: {}", self.task_id)}));
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
        let task = next_task(&tasks);
        let Some(task) = task else {
            return ok_json(serde_json::json!({"error": "No ready tasks"}));
        };
        if self.format == "text" {
            return ok_text(render_task_line(&task));
        }
        ok_json(task_to_json_value(&task, false))
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
            return ok_json(serde_json::json!({"error": format!("Task not found: {}", self.task_id)}));
        };
        let path = task.file_path.as_ref().ok_or_else(|| CallToolError::from_message("Missing task path"))?;
        update_task_field(path, "status", Some(self.status.clone().into()))
            .map_err(CallToolError::new)?;
        if self.touch {
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
            return ok_json(serde_json::json!({"error": format!("Task not found: {}", self.task_id)}));
        };
        let path = task.file_path.as_ref().ok_or_else(|| CallToolError::from_message("Missing task path"))?;
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
        ok_json(serde_json::json!({"ok": true, "id": task.id, "field": self.field.clone(), "value": self.value.clone()}))
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

impl ClaimTaskTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let task = find_task(&tasks, &self.task_id);
        let Some(task) = task else {
            return ok_json(serde_json::json!({"error": format!("Task not found: {}", self.task_id)}));
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
            return ok_json(serde_json::json!({"error": format!("Task not found: {}", self.task_id)}));
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
            return ok_json(serde_json::json!({"error": format!("Task not found: {}", self.task_id)}));
        };
        let path = task.file_path.as_ref().ok_or_else(|| CallToolError::from_message("Missing task path"))?;
        let section_key = if self.section == "notes" { "notes" } else { "impl" };
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
            return ok_json(serde_json::json!({"error": format!("Task not found: {}", self.task_id)}));
        };
        let path = task.file_path.as_ref().ok_or_else(|| CallToolError::from_message("Missing task path"))?;
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
            return ok_json(serde_json::json!({"error": format!("Task not found: {}", self.task_id)}));
        };
        let path = task.file_path.as_ref().ok_or_else(|| CallToolError::from_message("Missing task path"))?;
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
        let task_id = self.task_id.clone().unwrap_or_else(|| next_id(&tasks));
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
        let task_id = self.task_id.clone().unwrap_or_else(|| next_id(&tasks));
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

impl GanttTextTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let backlog_dir = match resolve_root(context, self.root.as_deref()) {
            Ok(dir) => dir,
            Err(err) => return ok_json(err),
        };
        let tasks = load_tasks(&backlog_dir);
        let text = plantuml_gantt(
            &tasks,
            self.start.as_deref(),
            None,
            self.zoom,
            None,
            true,
        );
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
        let text = plantuml_gantt(
            &tasks,
            self.start.as_deref(),
            None,
            self.zoom,
            None,
            true,
        );
        let path = write_text_file(Path::new(&self.output), &text)
            .map_err(CallToolError::new)?;
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
        let text = plantuml_gantt(
            &tasks,
            self.start.as_deref(),
            None,
            self.zoom,
            None,
            true,
        );
        let cmd = match &self.plantuml_cmd {
            Some(cmd) => Some(shell_words::split(cmd).map_err(CallToolError::new)?),
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

impl SkillContentTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let name = self
            .name
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("workmesh");
        let (path, content) = match read_skill_content(&repo_root, name) {
            Ok(result) => result,
            Err(err) => return ok_json(err),
        };
        if self.format == "json" {
            return ok_json(serde_json::json!({
                "name": name,
                "path": path,
                "content": content,
            }));
        }
        ok_text(content)
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
                    "root is optional if the server is started inside a repo with tasks/ or backlog/tasks",
                    "Dependencies are first-class. Use them to model blockers.",
                    "Use validate to catch missing or broken dependencies.",
                    "List-style arguments accept CSV strings or JSON arrays.",
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
            "workmesh MCP help\n\nPurpose:\n  Manage Markdown-backed backlogs with first-class dependencies.\n\nRoot resolution:\n  - MCP tools can infer root from CWD.\n\nAvailable tools:\n{}\n\nBest practices:\n{}\n",
            catalog, hint_lines
        );
        ok_text(body)
    }
}

impl ProjectManagementSkillTool {
    fn call(&self, context: &McpContext) -> Result<CallToolResult, CallToolError> {
        let repo_root = resolve_repo_root(context, self.root.as_deref());
        let skill_name = "workmesh";
        let (path, content) = match read_skill_content(&repo_root, skill_name) {
            Ok(result) => result,
            Err(err) => return ok_json(err),
        };
        if self.format == "json" {
            return ok_json(serde_json::json!({
                "summary": "workmesh project management skill",
                "name": skill_name,
                "path": path,
                "content": content,
            }));
        }
        ok_text(content)
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
    let path = task.file_path.as_ref().ok_or_else(|| CallToolError::from_message("Missing task path"))?;
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

fn next_id(tasks: &[Task]) -> String {
    let mut max_num = 0;
    for task in tasks {
        max_num = max_num.max(task.id_num());
    }
    format!("task-{:03}", max_num + 1)
}

