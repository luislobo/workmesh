use std::collections::BTreeSet;
use std::process::Command;

use async_trait::async_trait;
use tempfile::TempDir;

use rust_mcp_sdk::schema::{
    CallToolRequestParams, ClientCapabilities, Implementation, InitializeRequestParams,
    LATEST_PROTOCOL_VERSION,
};
use rust_mcp_sdk::{
    mcp_client::{client_runtime, ClientHandler, McpClientOptions},
    McpClient, StdioTransport, ToMcpClientHandler, TransportOptions,
};

struct NoopClientHandler;

#[async_trait]
impl ClientHandler for NoopClientHandler {}

fn client_details() -> InitializeRequestParams {
    InitializeRequestParams {
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "workmesh-mcp-parity".into(),
            version: "0.1.0".into(),
            title: Some("WorkMesh MCP Parity".into()),
            description: Some("CLI/MCP parity test".into()),
            icons: vec![],
            website_url: None,
        },
        protocol_version: LATEST_PROTOCOL_VERSION.into(),
        meta: None,
    }
}

fn cli() -> Command {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_workmesh") {
        return Command::new(path);
    }
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let root = std::path::Path::new(&manifest_dir)
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root");
    let candidate = root.join("target").join("debug").join("workmesh");
    Command::new(candidate)
}

fn write_task(
    dir: &std::path::Path,
    id: &str,
    title: &str,
    status: &str,
    dependencies: &[&str],
) {
    let filename = format!("{} - {}.md", id, title.to_lowercase());
    let path = dir.join(filename);
    let deps = if dependencies.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", dependencies.join(", "))
    };
    let content = format!(
        "---\nid: {id}\ntitle: {title}\nstatus: {status}\npriority: P2\nphase: Phase3\ndependencies: {deps}\nlabels: []\nassignee: []\n---\n\nBody\n",
        id = id,
        title = title,
        status = status,
        deps = deps
    );
    std::fs::write(path, content).expect("write task");
}

fn ids_from_json(text: &str) -> BTreeSet<String> {
    let value: serde_json::Value = serde_json::from_str(text).expect("json array");
    value
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|item| item.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
        .collect()
}

#[tokio::test]
async fn cli_and_mcp_list_ready_parity() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha", "Done", &[]);
    write_task(&tasks_dir, "task-002", "Beta", "To Do", &["task-001"]);
    write_task(&tasks_dir, "task-003", "Gamma", "To Do", &["task-002"]);
    write_task(&tasks_dir, "task-004", "Delta", "To Do", &[]);

    // CLI list
    let cli_list = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("list")
        .arg("--json")
        .output()
        .expect("cli list");
    assert!(cli_list.status.success());
    let cli_list_text = String::from_utf8_lossy(&cli_list.stdout).to_string();

    // CLI ready
    let cli_ready = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("ready")
        .arg("--json")
        .output()
        .expect("cli ready");
    assert!(cli_ready.status.success());
    let cli_ready_text = String::from_utf8_lossy(&cli_ready.stdout).to_string();

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
                serde_json::json!({"root": temp.path().display().to_string(), "format": "json"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            meta: None,
            task: None,
        })
        .await
        .expect("mcp list");
    let mcp_list_text = list_result
        .content
        .first()
        .unwrap()
        .as_text_content()
        .unwrap()
        .text
        .clone();

    let ready_result = client
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
        .expect("mcp ready");
    let mcp_ready_text = ready_result
        .content
        .first()
        .unwrap()
        .as_text_content()
        .unwrap()
        .text
        .clone();

    client.shut_down().await.expect("shutdown");

    let cli_list_ids = ids_from_json(&cli_list_text);
    let mcp_list_ids = ids_from_json(&mcp_list_text);
    assert_eq!(cli_list_ids, mcp_list_ids);

    let cli_ready_ids = ids_from_json(&cli_ready_text);
    let mcp_ready_ids = ids_from_json(&mcp_ready_text);
    assert_eq!(cli_ready_ids, mcp_ready_ids);

    assert!(cli_ready_ids.contains("task-002"));
    assert!(cli_ready_ids.contains("task-004"));
    assert!(!cli_ready_ids.contains("task-003"));
}
