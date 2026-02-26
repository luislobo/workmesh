use std::{
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};

use axum::{http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use workmesh_core::backlog::{locate_backlog_dir, resolve_backlog_dir, BacklogError};
use workmesh_core::task::load_tasks;
use workmesh_core::task_ops::{filter_tasks, sort_tasks, status_counts, task_to_json_value};
use workmesh_render::RenderError;

use crate::toolhost::{ProviderEntry, ProviderInfo, ProviderTool, ToolError, ToolHost};

const ROOT_REQUIRED_ERROR: &str =
    "root is required unless the service runs in a repo containing tasks/ or backlog/tasks";

#[derive(Debug, Deserialize)]
pub struct InvokeRequest {
    pub request_id: Option<String>,
    pub namespace: Option<String>,
    pub tool: String,
    #[serde(default)]
    pub arguments: Value,
}

pub async fn list_providers() -> impl IntoResponse {
    let providers = match tool_host().read() {
        Ok(host) => host.providers(),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "ok": false,
                    "error": {
                        "code": "INTERNAL",
                        "message": "provider registry lock poisoned",
                        "details": {}
                    }
                })),
            )
                .into_response();
        }
    };
    (
        StatusCode::OK,
        Json(json!({ "ok": true, "providers": providers })),
    )
        .into_response()
}

pub async fn invoke(Json(payload): Json<InvokeRequest>) -> impl IntoResponse {
    let namespace = payload.namespace.as_deref().unwrap_or("workmesh");
    let result = match tool_host().read() {
        Ok(host) => host.invoke(namespace, &payload.tool, &payload.arguments),
        Err(_) => Err(ToolError::internal("provider registry lock poisoned")),
    };
    match result {
        Ok(result) => (
            StatusCode::OK,
            Json(json!({
                "request_id": payload.request_id,
                "ok": true,
                "result": result,
                "meta": {
                    "namespace": namespace,
                    "tool": payload.tool
                }
            })),
        )
            .into_response(),
        Err(err) => error_response(
            err.status,
            payload.request_id,
            err.code,
            err.message,
            err.details,
        ),
    }
}

fn error_response(
    status: StatusCode,
    request_id: Option<String>,
    code: &'static str,
    message: String,
    details: Value,
) -> axum::response::Response {
    (
        status,
        Json(json!({
            "request_id": request_id,
            "ok": false,
            "error": {
                "code": code,
                "message": message,
                "details": details,
            }
        })),
    )
        .into_response()
}

fn tool_host() -> &'static RwLock<ToolHost> {
    static HOST: OnceLock<RwLock<ToolHost>> = OnceLock::new();
    HOST.get_or_init(|| RwLock::new(build_tool_host()))
}

pub fn reload_tool_host() -> Result<usize, String> {
    let mut host = tool_host()
        .write()
        .map_err(|_| "provider registry lock poisoned".to_string())?;
    *host = build_tool_host();
    Ok(host.providers().len())
}

fn build_tool_host() -> ToolHost {
    let mut host = ToolHost::new();
    host.register(ProviderEntry::new(
        workmesh_provider_info(),
        dispatch_workmesh_tool,
    ));
    host.register(ProviderEntry::new(
        system_provider_info(),
        dispatch_system_tool,
    ));
    host.register(ProviderEntry::new(
        render_provider_info(),
        dispatch_render_tool,
    ));
    host
}

fn workmesh_provider_info() -> ProviderInfo {
    ProviderInfo {
        namespace: "workmesh",
        version: crate::version::FULL,
        tools: vec![
            ProviderTool {
                name: "list_tasks",
                description: "List tasks with optional status filter.",
            },
            ProviderTool {
                name: "show_task",
                description: "Show a single task by id.",
            },
            ProviderTool {
                name: "stats",
                description: "Return counts by status.",
            },
        ],
    }
}

fn system_provider_info() -> ProviderInfo {
    ProviderInfo {
        namespace: "system",
        version: crate::version::FULL,
        tools: vec![
            ProviderTool {
                name: "ping",
                description: "Basic availability probe.",
            },
            ProviderTool {
                name: "version",
                description: "Return service version information.",
            },
        ],
    }
}

fn render_provider_info() -> ProviderInfo {
    ProviderInfo {
        namespace: "render",
        version: crate::version::FULL,
        tools: vec![
            ProviderTool {
                name: "render_table",
                description: "Render tabular data for terminal output.",
            },
            ProviderTool {
                name: "render_kv",
                description: "Render aligned key-value output.",
            },
            ProviderTool {
                name: "render_stats",
                description: "Render compact or table-based metrics.",
            },
            ProviderTool {
                name: "render_progress",
                description: "Render progress bars from percent or current/total values.",
            },
            ProviderTool {
                name: "render_tree",
                description: "Render nested objects/arrays as an ASCII tree.",
            },
            ProviderTool {
                name: "render_diff",
                description: "Render a unified text diff from before/after input.",
            },
            ProviderTool {
                name: "render_logs",
                description: "Render normalized logs as table output.",
            },
            ProviderTool {
                name: "render_alerts",
                description: "Render alert/notification lines.",
            },
            ProviderTool {
                name: "render_list",
                description: "Render ordered/unordered/checklist output.",
            },
            ProviderTool {
                name: "render_chart_bar",
                description: "Render ASCII bar charts from numeric values.",
            },
            ProviderTool {
                name: "render_sparkline",
                description: "Render compact sparkline output.",
            },
            ProviderTool {
                name: "render_timeline",
                description: "Render chronological events with status markers.",
            },
        ],
    }
}

fn dispatch_system_tool(tool: &str, _arguments: &Value) -> Result<Value, ToolError> {
    match tool {
        "ping" => Ok(json!({
            "pong": true,
            "timestamp": Utc::now().to_rfc3339(),
        })),
        "version" => Ok(json!({
            "service": "workmesh-service",
            "version": crate::version::FULL,
        })),
        _ => Err(ToolError::not_found(format!("Tool not found: {}", tool))),
    }
}

fn dispatch_workmesh_tool(tool: &str, arguments: &Value) -> Result<Value, ToolError> {
    match tool {
        "list_tasks" => list_tasks(arguments),
        "show_task" => show_task(arguments),
        "stats" => stats(arguments),
        _ => Err(ToolError::not_found(format!("Tool not found: {}", tool))),
    }
}

fn dispatch_render_tool(tool: &str, arguments: &Value) -> Result<Value, ToolError> {
    workmesh_render::dispatch_tool(tool, arguments).map_err(|err| match err {
        RenderError::InvalidArgument(message) => ToolError::invalid_argument(message),
        RenderError::NotFound(message) => ToolError::not_found(message),
        RenderError::Internal(message) => ToolError::internal(message),
    })
}

fn list_tasks(arguments: &Value) -> Result<Value, ToolError> {
    let backlog_dir = resolve_root(arguments.get("root").and_then(|value| value.as_str()))?;
    let tasks = load_tasks(&backlog_dir);
    let status_filter = parse_list_argument(arguments.get("status"))?;
    let sort_key = arguments
        .get("sort")
        .and_then(|value| value.as_str())
        .unwrap_or("id");
    let include_body = arguments
        .get("include_body")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let filtered = filter_tasks(
        &tasks,
        status_filter.as_ref().map(|values| values.as_slice()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    let sorted = sort_tasks(filtered, sort_key);
    let rows: Vec<Value> = sorted
        .iter()
        .map(|task| task_to_json_value(task, include_body))
        .collect();
    Ok(json!({
        "tasks": rows,
        "count": rows.len(),
        "root": backlog_dir.display().to_string(),
    }))
}

fn show_task(arguments: &Value) -> Result<Value, ToolError> {
    let backlog_dir = resolve_root(arguments.get("root").and_then(|value| value.as_str()))?;
    let task_id = arguments
        .get("task_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ToolError::invalid_argument("task_id is required"))?;
    let include_body = arguments
        .get("include_body")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let tasks = load_tasks(&backlog_dir);
    let task = tasks
        .iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| ToolError::not_found(format!("Task not found: {}", task_id)))?;
    Ok(json!({
        "task": task_to_json_value(task, include_body),
    }))
}

fn stats(arguments: &Value) -> Result<Value, ToolError> {
    let backlog_dir = resolve_root(arguments.get("root").and_then(|value| value.as_str()))?;
    let tasks = load_tasks(&backlog_dir);
    let counts = status_counts(&tasks)
        .into_iter()
        .map(|(status, count)| json!({ "status": status, "count": count }))
        .collect::<Vec<_>>();
    Ok(json!({
        "counts": counts,
    }))
}

fn resolve_root(root: Option<&str>) -> Result<PathBuf, ToolError> {
    let resolved = if let Some(raw_root) = root {
        let trimmed = raw_root.trim();
        if trimmed.is_empty() {
            locate_backlog_dir(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        } else {
            resolve_backlog_dir(Path::new(trimmed))
        }
    } else {
        locate_backlog_dir(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    };

    match resolved {
        Ok(path) => Ok(path),
        Err(BacklogError::NotFound(_)) => Err(ToolError::invalid_argument(ROOT_REQUIRED_ERROR)),
    }
}

fn parse_list_argument(value: Option<&Value>) -> Result<Option<Vec<String>>, ToolError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if let Some(single) = value.as_str() {
        let parsed = single
            .split(',')
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        return Ok(Some(parsed));
    }
    if let Some(list) = value.as_array() {
        let mut parsed = Vec::with_capacity(list.len());
        for item in list {
            let value = item.as_str().ok_or_else(|| {
                ToolError::invalid_argument("status list entries must be strings")
            })?;
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                parsed.push(trimmed.to_string());
            }
        }
        return Ok(Some(parsed));
    }
    Err(ToolError::invalid_argument(
        "status must be a string or an array of strings",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    fn write_task_fixture(root: &Path, id: &str, title: &str) {
        let task_dir = root.join("workmesh/tasks");
        fs::create_dir_all(&task_dir).expect("task dir");
        let file = task_dir.join(format!("{id} - fixture.md"));
        let content = format!(
            "---\nid: {id}\nuid: 01TEST{id}\ntitle: {title}\nkind: task\nstatus: To Do\npriority: P1\nphase: Phase1\ndependencies: []\nlabels: []\nassignee: []\nrelationships:\n  blocked_by: []\n  parent: []\n  child: []\n  discovered_from: []\n---\n\nDescription:\n--------------------------------------------------\n- Ship {title}.\n\nAcceptance Criteria:\n--------------------------------------------------\n- {title} behavior is implemented.\n\nDefinition of Done:\n--------------------------------------------------\n- Description goals met and acceptance criteria satisfied.\n- Code/config committed.\n- Docs updated if needed.\n"
        );
        fs::write(file, content).expect("write task");
    }

    #[test]
    fn list_tasks_returns_results() {
        let root = TempDir::new().expect("tempdir");
        write_task_fixture(root.path(), "task-main-900", "alpha");
        let args = json!({
            "root": root.path().display().to_string(),
        });
        let result = dispatch_workmesh_tool("list_tasks", &args).expect("list");
        assert_eq!(result["count"], 1);
        assert_eq!(result["tasks"][0]["id"], "task-main-900");
    }

    #[test]
    fn stats_returns_status_counts() {
        let root = TempDir::new().expect("tempdir");
        write_task_fixture(root.path(), "task-main-901", "beta");
        let args = json!({
            "root": root.path().display().to_string(),
        });
        let result = dispatch_workmesh_tool("stats", &args).expect("stats");
        let counts = result["counts"].as_array().expect("counts array");
        assert!(!counts.is_empty());
    }

    #[test]
    fn unknown_tool_returns_not_found() {
        let err = dispatch_workmesh_tool("no_such_tool", &json!({})).expect_err("expected err");
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert_eq!(err.code, "NOT_FOUND");
    }

    #[test]
    fn system_provider_ping_returns_pong() {
        let result = dispatch_system_tool("ping", &json!({})).expect("ping");
        assert_eq!(result["pong"], true);
    }

    #[test]
    fn provider_list_exposes_workmesh_and_system() {
        let providers = tool_host().read().expect("lock").providers();
        let namespaces = providers
            .iter()
            .map(|provider| provider.namespace)
            .collect::<Vec<_>>();
        assert!(namespaces.contains(&"workmesh"));
        assert!(namespaces.contains(&"system"));
        assert!(namespaces.contains(&"render"));
    }

    #[test]
    fn render_provider_invokes_tool() {
        let result = dispatch_render_tool(
            "render_list",
            &json!({
                "data": [{ "text": "one" }, { "text": "two" }],
                "configuration": { "ordered": true }
            }),
        )
        .expect("render list");
        let text = result["text"].as_str().expect("text");
        assert!(text.contains("1."));
    }
}
