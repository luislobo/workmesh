use tempfile::TempDir;

use rust_mcp_sdk::schema::{
    CallToolRequestParams, ClientCapabilities, Implementation, InitializeRequestParams,
    LATEST_PROTOCOL_VERSION,
};
use rust_mcp_sdk::{
    mcp_client::{client_runtime, ClientHandler, McpClientOptions},
    McpClient, StdioTransport, ToMcpClientHandler, TransportOptions,
};

use async_trait::async_trait;
// Note: server lifecycle is controlled by the MCP client runtime; this test avoids
// forcing process exit so it can be stable in CI across platforms.

struct NoopClientHandler;

#[async_trait]
impl ClientHandler for NoopClientHandler {}

fn client_details() -> InitializeRequestParams {
    InitializeRequestParams {
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "workmesh-mcp-test".into(),
            version: "0.1.0".into(),
            title: Some("WorkMesh MCP Test".into()),
            description: Some("Integration test client".into()),
            icons: vec![],
            website_url: None,
        },
        protocol_version: LATEST_PROTOCOL_VERSION.into(),
        meta: None,
    }
}

fn write_task(dir: &std::path::Path, id: &str, title: &str, status: &str) {
    let filename = format!("{} - {}.md", id, title.to_lowercase());
    let path = dir.join(filename);
    let content = format!(
        "---\nid: {id}\ntitle: {title}\nstatus: {status}\npriority: P2\nphase: Phase3\ndependencies: []\nlabels: []\nassignee: []\n---\n\nBody\n",
        id = id,
        title = title,
        status = status
    );
    std::fs::write(path, content).expect("write task");
}

#[tokio::test]
async fn mcp_list_tasks_and_checkpoint() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha", "To Do");

    let project_dir = temp.path().join("docs").join("projects").join("alpha");
    std::fs::create_dir_all(project_dir.join("updates")).expect("updates dir");

    let server_bin = env!("CARGO_BIN_EXE_workmesh-mcp");
    let transport = StdioTransport::create_with_server_launch(
        server_bin,
        vec![],
        None,
        TransportOptions::default(),
    )
    .expect("transport");

    let client = client_runtime::create_client(McpClientOptions {
        client_details: client_details(),
        transport,
        handler: NoopClientHandler.to_mcp_client_handler(),
        task_store: None,
        server_task_store: None,
    });

    client.clone().start().await.expect("start client");

    let version_result = client
        .request_tool_call(CallToolRequestParams {
            name: "version".to_string(),
            arguments: Some(serde_json::json!({}).as_object().unwrap().clone()),
            meta: None,
            task: None,
        })
        .await
        .expect("version");
    let version_text = version_result
        .content
        .first()
        .unwrap()
        .as_text_content()
        .unwrap()
        .text
        .clone();
    assert!(version_text.contains("version"));

    let list_result = client
        .request_tool_call(CallToolRequestParams {
            name: "list_tasks".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string()})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("list tasks");
    let list_text = list_result
        .content
        .first()
        .unwrap()
        .as_text_content()
        .unwrap()
        .text
        .clone();
    assert!(list_text.contains("task-001"));

    let checkpoint_result = client
        .request_tool_call(CallToolRequestParams {
            name: "checkpoint".to_string(),
            arguments: Some(
                serde_json::json!({
                    "root": temp.path().display().to_string(),
                    "project": "alpha",
                    "id": "20260204-123000",
                    "format": "json"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("checkpoint");
    let checkpoint_text = checkpoint_result
        .content
        .first()
        .unwrap()
        .as_text_content()
        .unwrap()
        .text
        .clone();
    assert!(checkpoint_text.contains("checkpoint_id"));

    client.shut_down().await.expect("shutdown");
}

#[tokio::test]
async fn mcp_list_tasks_all_includes_archived_tasks() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    let archive_dir = backlog_dir.join("archive").join("2026-02");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
    std::fs::create_dir_all(&archive_dir).expect("archive dir");

    write_task(&tasks_dir, "task-001", "Alpha", "To Do");
    write_task(&archive_dir, "task-002", "Beta", "Done");

    let project_dir = temp.path().join("docs").join("projects").join("alpha");
    std::fs::create_dir_all(project_dir.join("updates")).expect("updates dir");

    let server_bin = env!("CARGO_BIN_EXE_workmesh-mcp");
    let transport = StdioTransport::create_with_server_launch(
        server_bin,
        vec![],
        None,
        TransportOptions::default(),
    )
    .expect("transport");

    let client = client_runtime::create_client(McpClientOptions {
        client_details: client_details(),
        transport,
        handler: NoopClientHandler.to_mcp_client_handler(),
        task_store: None,
        server_task_store: None,
    });

    client.clone().start().await.expect("start client");

    let list_active = client
        .request_tool_call(CallToolRequestParams {
            name: "list_tasks".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string(), "format": "json"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("list tasks");
    let list_text = list_active
        .content
        .first()
        .unwrap()
        .as_text_content()
        .unwrap()
        .text
        .clone();
    let parsed: serde_json::Value = serde_json::from_str(&list_text).expect("json");
    let ids: Vec<_> = parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.get("id").unwrap().as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"task-001".to_string()));
    assert!(!ids.contains(&"task-002".to_string()));

    let list_all = client
        .request_tool_call(CallToolRequestParams {
            name: "list_tasks".to_string(),
            arguments: Some(
                serde_json::json!({
                    "root": temp.path().display().to_string(),
                    "all": true,
                    "format": "json"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("list tasks all");
    let list_text = list_all
        .content
        .first()
        .unwrap()
        .as_text_content()
        .unwrap()
        .text
        .clone();
    let parsed: serde_json::Value = serde_json::from_str(&list_text).expect("json");
    let ids: Vec<_> = parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.get("id").unwrap().as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"task-001".to_string()));
    assert!(ids.contains(&"task-002".to_string()));

    client.shut_down().await.expect("shutdown");
}

#[tokio::test]
async fn mcp_smoke_more_tools() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha", "To Do");
    write_task(&tasks_dir, "task-002", "Beta", "To Do");

    let project_dir = temp.path().join("docs").join("projects").join("alpha");
    std::fs::create_dir_all(project_dir.join("updates")).expect("updates dir");

    // Provide a skill file so skill tools can succeed.
    let skill_dir = temp.path().join("skills").join("workmesh");
    std::fs::create_dir_all(&skill_dir).expect("skills dir");
    std::fs::write(skill_dir.join("SKILL.md"), "# skill\n").expect("write skill");

    let server_bin = env!("CARGO_BIN_EXE_workmesh-mcp");
    let transport = StdioTransport::create_with_server_launch(
        server_bin,
        vec![],
        None,
        TransportOptions::default(),
    )
    .expect("transport");

    let client = client_runtime::create_client(McpClientOptions {
        client_details: client_details(),
        transport,
        handler: NoopClientHandler.to_mcp_client_handler(),
        task_store: None,
        server_task_store: None,
    });

    client.clone().start().await.expect("start client");

    let root = temp.path().display().to_string();

    client
        .request_tool_call(CallToolRequestParams {
            name: "focus_set".to_string(),
            arguments: Some(
                serde_json::json!({
                    "root": root,
                    "project_id": "alpha",
                    "epic_id": "task-001",
                    "objective": "Ship"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("focus_set");

    let focus_show = client
        .request_tool_call(CallToolRequestParams {
            name: "focus_show".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string(), "format": "json"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("focus_show");
    let focus_text = focus_show
        .content
        .first()
        .unwrap()
        .as_text_content()
        .unwrap()
        .text
        .clone();
    assert!(focus_text.contains("project_id"));

    let ready = client
        .request_tool_call(CallToolRequestParams {
            name: "ready_tasks".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string(), "format": "json"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("ready_tasks");
    assert!(!ready.content.is_empty());

    let next = client
        .request_tool_call(CallToolRequestParams {
            name: "next_task".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string(), "format": "json"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("next_task");
    assert!(!next.content.is_empty());

    let show = client
        .request_tool_call(CallToolRequestParams {
            name: "show_task".to_string(),
            arguments: Some(
                serde_json::json!({
                    "root": temp.path().display().to_string(),
                    "task_id": "task-001",
                    "format": "json"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("show_task");
    assert!(!show.content.is_empty());

    let stats = client
        .request_tool_call(CallToolRequestParams {
            name: "stats".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string(), "format": "json"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("stats");
    assert!(!stats.content.is_empty());

    let validate = client
        .request_tool_call(CallToolRequestParams {
            name: "validate".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string(), "format": "json"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("validate");
    assert!(!validate.content.is_empty());

    client
        .request_tool_call(CallToolRequestParams {
            name: "index_rebuild".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string()})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("index_rebuild");

    let graph = client
        .request_tool_call(CallToolRequestParams {
            name: "graph_export".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string(), "pretty": true})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("graph_export");
    assert!(!graph.content.is_empty());

    let issues_export = client
        .request_tool_call(CallToolRequestParams {
            name: "issues_export".to_string(),
            arguments: Some(
                serde_json::json!({
                    "root": temp.path().display().to_string(),
                    "include_body": false
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("issues_export");
    assert!(!issues_export.content.is_empty());

    let tool_info = client
        .request_tool_call(CallToolRequestParams {
            name: "tool_info".to_string(),
            arguments: Some(
                serde_json::json!({"name": "list_tasks", "format": "text"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("tool_info");
    assert!(!tool_info.content.is_empty());

    let best = client
        .request_tool_call(CallToolRequestParams {
            name: "best_practices".to_string(),
            arguments: Some(
                serde_json::json!({"format": "text"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("best_practices");
    let best_text = best
        .content
        .first()
        .unwrap()
        .as_text_content()
        .unwrap()
        .text
        .clone();
    assert!(best_text.to_lowercase().contains("derived"));

    let skill = client
        .request_tool_call(CallToolRequestParams {
            name: "project_management_skill".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string(), "format": "json"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("project_management_skill");
    assert!(!skill.content.is_empty());

    let checkpoint = client
        .request_tool_call(CallToolRequestParams {
            name: "checkpoint".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string(), "project": "alpha", "format": "json"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("checkpoint");
    assert!(!checkpoint.content.is_empty());

    let resume = client
        .request_tool_call(CallToolRequestParams {
            name: "resume".to_string(),
            arguments: Some(
                serde_json::json!({"root": temp.path().display().to_string(), "project": "alpha", "format": "text"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("resume");
    assert!(!resume.content.is_empty());

    client.shut_down().await.expect("shutdown");
}
