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
