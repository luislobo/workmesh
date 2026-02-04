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
        vec!["--root".into(), temp.path().display().to_string()],
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
