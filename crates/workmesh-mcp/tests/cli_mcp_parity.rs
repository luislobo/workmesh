use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

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

use workmesh_core::task::parse_task_file;

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

fn mcp_bin() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_workmesh-mcp") {
        return PathBuf::from(path);
    }
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let root = std::path::Path::new(&manifest_dir)
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root");
    root.join("target").join("debug").join("workmesh-mcp")
}

async fn start_client(root: &Path) -> Arc<rust_mcp_sdk::mcp_client::ClientRuntime> {
    let transport = StdioTransport::create_with_server_launch(
        mcp_bin().display().to_string(),
        vec!["--root".into(), root.display().to_string()],
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
    client
}

async fn call_tool_text(
    client: &Arc<rust_mcp_sdk::mcp_client::ClientRuntime>,
    name: &str,
    args: serde_json::Value,
) -> String {
    let result = client
        .request_tool_call(CallToolRequestParams {
            name: name.to_string(),
            arguments: Some(args.as_object().unwrap().clone()),
            meta: None,
            task: None,
        })
        .await
        .expect("tool call");
    result
        .content
        .first()
        .unwrap()
        .as_text_content()
        .unwrap()
        .text
        .clone()
}

fn write_task(
    dir: &Path,
    id: &str,
    title: &str,
    status: &str,
    dependencies: &[&str],
) -> PathBuf {
    let filename = format!("{} - {}.md", id, title.to_lowercase());
    let path = dir.join(filename);
    let deps = if dependencies.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", dependencies.join(", "))
    };
    let content = format!(
        "---\nid: {id}\ntitle: {title}\nstatus: {status}\npriority: P2\nphase: Phase3\ndependencies: {deps}\nlabels: []\nassignee: []\n---\n\n## Notes\n- initial\n",
        id = id,
        title = title,
        status = status,
        deps = deps
    );
    std::fs::write(&path, content).expect("write task");
    path
}

fn find_task_path(tasks_dir: &Path, id: &str) -> PathBuf {
    let entries = std::fs::read_dir(tasks_dir).expect("read tasks dir");
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.to_lowercase().starts_with(&id.to_lowercase()) {
            return entry.path();
        }
    }
    panic!("task file not found for {}", id);
}

fn write_fake_plantuml(dir: &Path) -> PathBuf {
    let path = dir.join("fake-plantuml.sh");
    let script = "#!/bin/sh\ncat >/dev/null\necho \"<svg></svg>\"\n";
    std::fs::write(&path, script).expect("write plantuml script");
    let mut perms = std::fs::metadata(&path).expect("stat").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).expect("chmod");
    path
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

fn normalize_tasks(value: serde_json::Value) -> Vec<serde_json::Value> {
    let mut items = value.as_array().cloned().unwrap_or_default();
    items.sort_by_key(|item| {
        item.get("id")
            .and_then(|id| id.as_str())
            .unwrap_or("")
            .to_string()
    });
    items
}

fn graph_signature(value: serde_json::Value) -> (BTreeSet<String>, BTreeSet<String>) {
    let nodes = value
        .get("nodes")
        .and_then(|nodes| nodes.as_array())
        .cloned()
        .unwrap_or_default();
    let edges = value
        .get("edges")
        .and_then(|edges| edges.as_array())
        .cloned()
        .unwrap_or_default();
    let node_ids = nodes
        .iter()
        .filter_map(|node| node.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
        .collect();
    let edge_ids = edges
        .iter()
        .filter_map(|edge| {
            let from = edge.get("from")?.as_str()?;
            let to = edge.get("to")?.as_str()?;
            let edge_type = edge.get("edge_type")?.as_str()?;
            Some(format!("{}:{}:{}", from, edge_type, to))
        })
        .collect();
    (node_ids, edge_ids)
}

fn parse_lineset(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
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

    let cli_list = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("list")
        .arg("--json")
        .output()
        .expect("cli list");
    assert!(cli_list.status.success());
    let cli_list_text = String::from_utf8_lossy(&cli_list.stdout).to_string();

    let cli_ready = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("ready")
        .arg("--json")
        .output()
        .expect("cli ready");
    assert!(cli_ready.status.success());
    let cli_ready_text = String::from_utf8_lossy(&cli_ready.stdout).to_string();

    let client = start_client(temp.path()).await;

    let mcp_list_text = call_tool_text(
        &client,
        "list_tasks",
        serde_json::json!({"root": temp.path().display().to_string(), "format": "json"}),
    )
    .await;

    let mcp_ready_text = call_tool_text(
        &client,
        "ready_tasks",
        serde_json::json!({"root": temp.path().display().to_string(), "format": "json"}),
    )
    .await;

    client.shut_down().await.expect("shutdown");

    assert_eq!(ids_from_json(&cli_list_text), ids_from_json(&mcp_list_text));
    assert_eq!(ids_from_json(&cli_ready_text), ids_from_json(&mcp_ready_text));
}

#[tokio::test]
async fn cli_and_mcp_show_next_stats_export_parity() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha", "Done", &[]);
    write_task(&tasks_dir, "task-002", "Beta", "To Do", &["task-001"]);

    let cli_show = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("task-002")
        .arg("--json")
        .output()
        .expect("cli show");
    assert!(cli_show.status.success());
    let cli_show_value: serde_json::Value =
        serde_json::from_slice(&cli_show.stdout).expect("cli show json");

    let cli_next = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("next")
        .arg("--json")
        .output()
        .expect("cli next");
    assert!(cli_next.status.success());
    let cli_next_value: serde_json::Value =
        serde_json::from_slice(&cli_next.stdout).expect("cli next json");

    let cli_stats = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("stats")
        .arg("--json")
        .output()
        .expect("cli stats");
    assert!(cli_stats.status.success());
    let cli_stats_value: serde_json::Value =
        serde_json::from_slice(&cli_stats.stdout).expect("cli stats json");

    let cli_export = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("export")
        .arg("--pretty")
        .output()
        .expect("cli export");
    assert!(cli_export.status.success());
    let cli_export_value: serde_json::Value =
        serde_json::from_slice(&cli_export.stdout).expect("cli export json");

    let client = start_client(temp.path()).await;

    let mcp_show_text = call_tool_text(
        &client,
        "show_task",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "format": "json",
            "include_body": true
        }),
    )
    .await;
    let mcp_show_value: serde_json::Value = serde_json::from_str(&mcp_show_text).unwrap();

    let mcp_next_text = call_tool_text(
        &client,
        "next_task",
        serde_json::json!({"root": temp.path().display().to_string(), "format": "json"}),
    )
    .await;
    let mcp_next_value: serde_json::Value = serde_json::from_str(&mcp_next_text).unwrap();

    let mcp_stats_text = call_tool_text(
        &client,
        "stats",
        serde_json::json!({"root": temp.path().display().to_string(), "format": "json"}),
    )
    .await;
    let mcp_stats_value: serde_json::Value = serde_json::from_str(&mcp_stats_text).unwrap();

    let mcp_export_text = call_tool_text(
        &client,
        "export_tasks",
        serde_json::json!({"root": temp.path().display().to_string(), "include_body": true}),
    )
    .await;
    let mcp_export_value: serde_json::Value = serde_json::from_str(&mcp_export_text).unwrap();

    client.shut_down().await.expect("shutdown");

    assert_eq!(cli_show_value, mcp_show_value);
    assert_eq!(cli_next_value.get("id"), mcp_next_value.get("id"));
    assert_eq!(cli_stats_value, mcp_stats_value);

    let cli_export_norm = normalize_tasks(cli_export_value);
    let mcp_export_norm = normalize_tasks(mcp_export_value);
    assert_eq!(cli_export_norm, mcp_export_norm);
}

#[tokio::test]
async fn cli_and_mcp_graph_issues_index_gantt_parity() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");

    write_task(&tasks_dir, "task-001", "Alpha", "Done", &[]);
    write_task(&tasks_dir, "task-002", "Beta", "To Do", &["task-001"]);

    let cli_graph = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("graph-export")
        .arg("--pretty")
        .output()
        .expect("cli graph");
    assert!(cli_graph.status.success());
    let cli_graph_value: serde_json::Value =
        serde_json::from_slice(&cli_graph.stdout).expect("cli graph json");

    let cli_issues = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("issues-export")
        .output()
        .expect("cli issues");
    assert!(cli_issues.status.success());
    let cli_issues_text = String::from_utf8_lossy(&cli_issues.stdout).to_string();

    let cli_index = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("index-rebuild")
        .arg("--json")
        .output()
        .expect("cli index");
    assert!(cli_index.status.success());
    let cli_index_value: serde_json::Value =
        serde_json::from_slice(&cli_index.stdout).expect("cli index json");

    let cli_index_refresh = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("index-refresh")
        .arg("--json")
        .output()
        .expect("cli index refresh");
    assert!(cli_index_refresh.status.success());
    let cli_index_refresh_value: serde_json::Value =
        serde_json::from_slice(&cli_index_refresh.stdout).expect("cli index refresh json");

    let cli_index_verify = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("index-verify")
        .arg("--json")
        .output()
        .expect("cli index verify");
    assert!(cli_index_verify.status.success());
    let cli_index_verify_value: serde_json::Value =
        serde_json::from_slice(&cli_index_verify.stdout).expect("cli index verify json");

    let cli_gantt = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("gantt")
        .arg("--zoom")
        .arg("3")
        .output()
        .expect("cli gantt");
    assert!(cli_gantt.status.success());
    let cli_gantt_text = String::from_utf8_lossy(&cli_gantt.stdout).to_string();

    let cli_gantt_file_path = temp.path().join("gantt-cli.txt");
    let cli_gantt_file = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("gantt-file")
        .arg("--zoom")
        .arg("3")
        .arg("--output")
        .arg(&cli_gantt_file_path)
        .output()
        .expect("cli gantt-file");
    assert!(cli_gantt_file.status.success());
    let cli_gantt_file_text =
        std::fs::read_to_string(&cli_gantt_file_path).expect("gantt file text");

    let fake_plantuml = write_fake_plantuml(temp.path());
    let cli_gantt_svg_path = temp.path().join("gantt-cli.svg");
    let cli_gantt_svg = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("gantt-svg")
        .arg("--zoom")
        .arg("3")
        .arg("--output")
        .arg(&cli_gantt_svg_path)
        .arg("--plantuml-cmd")
        .arg(fake_plantuml.display().to_string())
        .output()
        .expect("cli gantt-svg");
    assert!(cli_gantt_svg.status.success());
    let cli_gantt_svg_text =
        std::fs::read_to_string(&cli_gantt_svg_path).expect("gantt svg text");

    let client = start_client(temp.path()).await;

    let mcp_graph_text = call_tool_text(
        &client,
        "graph_export",
        serde_json::json!({"root": temp.path().display().to_string(), "pretty": true}),
    )
    .await;
    let mcp_graph_value: serde_json::Value = serde_json::from_str(&mcp_graph_text).unwrap();

    let mcp_issues_text = call_tool_text(
        &client,
        "issues_export",
        serde_json::json!({"root": temp.path().display().to_string()}),
    )
    .await;

    let mcp_index_text = call_tool_text(
        &client,
        "index_rebuild",
        serde_json::json!({"root": temp.path().display().to_string()}),
    )
    .await;
    let mcp_index_value: serde_json::Value = serde_json::from_str(&mcp_index_text).unwrap();

    let mcp_index_refresh_text = call_tool_text(
        &client,
        "index_refresh",
        serde_json::json!({"root": temp.path().display().to_string()}),
    )
    .await;
    let mcp_index_refresh_value: serde_json::Value =
        serde_json::from_str(&mcp_index_refresh_text).unwrap();

    let mcp_index_verify_text = call_tool_text(
        &client,
        "index_verify",
        serde_json::json!({"root": temp.path().display().to_string()}),
    )
    .await;
    let mcp_index_verify_value: serde_json::Value =
        serde_json::from_str(&mcp_index_verify_text).unwrap();

    let mcp_gantt_text = call_tool_text(
        &client,
        "gantt_text",
        serde_json::json!({"root": temp.path().display().to_string(), "zoom": 3}),
    )
    .await;

    let mcp_gantt_file_path = temp.path().join("gantt-mcp.txt");
    let mcp_gantt_file_text = call_tool_text(
        &client,
        "gantt_file",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "zoom": 3,
            "output": mcp_gantt_file_path.display().to_string()
        }),
    )
    .await;
    let mcp_gantt_file_value: serde_json::Value = serde_json::from_str(&mcp_gantt_file_text).unwrap();
    let mcp_gantt_file_content =
        std::fs::read_to_string(&mcp_gantt_file_path).expect("mcp gantt file");

    let mcp_gantt_svg_path = temp.path().join("gantt-mcp.svg");
    let mcp_gantt_svg_text = call_tool_text(
        &client,
        "gantt_svg",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "zoom": 3,
            "output": mcp_gantt_svg_path.display().to_string(),
            "plantuml_cmd": fake_plantuml.display().to_string()
        }),
    )
    .await;
    let mcp_gantt_svg_value: serde_json::Value = serde_json::from_str(&mcp_gantt_svg_text).unwrap();
    let mcp_gantt_svg_content =
        std::fs::read_to_string(&mcp_gantt_svg_path).expect("mcp gantt svg");

    client.shut_down().await.expect("shutdown");

    let cli_graph_sig = graph_signature(cli_graph_value);
    let mcp_graph_sig = graph_signature(mcp_graph_value);
    assert_eq!(cli_graph_sig, mcp_graph_sig);

    assert_eq!(parse_lineset(&cli_issues_text), parse_lineset(&mcp_issues_text));

    assert!(cli_index_value.get("entries").is_some());
    assert_eq!(cli_index_value.get("entries"), mcp_index_value.get("entries"));
    assert_eq!(
        cli_index_refresh_value.get("entries"),
        mcp_index_refresh_value.get("entries")
    );
    assert_eq!(
        cli_index_verify_value.get("ok"),
        mcp_index_verify_value.get("ok")
    );

    assert_eq!(cli_gantt_text.trim(), mcp_gantt_text.trim());
    assert_eq!(cli_gantt_text.trim(), cli_gantt_file_text.trim());
    assert_eq!(cli_gantt_text.trim(), mcp_gantt_file_content.trim());
    assert_eq!(
        mcp_gantt_file_path.display().to_string(),
        mcp_gantt_file_value.get("path").and_then(|v| v.as_str()).unwrap_or("")
    );
    assert_eq!(
        mcp_gantt_svg_path.display().to_string(),
        mcp_gantt_svg_value.get("path").and_then(|v| v.as_str()).unwrap_or("")
    );
    assert_eq!(cli_gantt_svg_text.trim(), mcp_gantt_svg_content.trim());
}

#[tokio::test]
async fn cli_and_mcp_write_and_session_parity() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");

    let task_path = write_task(&tasks_dir, "task-001", "Alpha", "To Do", &[]);
    let task_path2 = write_task(&tasks_dir, "task-002", "Beta", "To Do", &[]);

    let cli_add = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("add")
        .arg("--id")
        .arg("task-003")
        .arg("--title")
        .arg("Gamma")
        .arg("--labels")
        .arg("seed")
        .output()
        .expect("cli add");
    assert!(cli_add.status.success());
    let task_path3 = find_task_path(&tasks_dir, "task-003");

    // CLI set-status
    let cli_status = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("set-status")
        .arg("task-001")
        .arg("In Progress")
        .output()
        .expect("cli set-status");
    assert!(cli_status.status.success());

    // MCP set_status
    let client = start_client(temp.path()).await;
    let _ = call_tool_text(
        &client,
        "add_task",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-004",
            "title": "Delta",
            "labels": ["seed2"]
        }),
    )
    .await;
    let task_path4 = find_task_path(&tasks_dir, "task-004");

    let cli_set_field = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("set-field")
        .arg("task-001")
        .arg("priority")
        .arg("P1")
        .output()
        .expect("cli set-field");
    assert!(cli_set_field.status.success());

    let _ = call_tool_text(
        &client,
        "set_status",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "status": "Done"
        }),
    )
    .await;
    let _ = call_tool_text(
        &client,
        "set_field",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "field": "phase",
            "value": "Phase2"
        }),
    )
    .await;

    // CLI label-add / remove
    let cli_label = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("label-add")
        .arg("task-001")
        .arg("docs")
        .output()
        .expect("cli label-add");
    assert!(cli_label.status.success());

    let cli_label_keep = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("label-add")
        .arg("task-001")
        .arg("keep")
        .output()
        .expect("cli label-add keep");
    assert!(cli_label_keep.status.success());

    let _ = call_tool_text(
        &client,
        "add_label",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "label": "infra"
        }),
    )
    .await;
    let _ = call_tool_text(
        &client,
        "add_label",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "label": "keep2"
        }),
    )
    .await;

    let cli_label_remove = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("label-remove")
        .arg("task-001")
        .arg("docs")
        .output()
        .expect("cli label-remove");
    assert!(cli_label_remove.status.success());

    let _ = call_tool_text(
        &client,
        "remove_label",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "label": "infra"
        }),
    )
    .await;

    // CLI dep-add / remove
    let cli_dep = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("dep-add")
        .arg("task-001")
        .arg("task-002")
        .output()
        .expect("cli dep-add");
    assert!(cli_dep.status.success());

    let cli_dep_extra = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("dep-add")
        .arg("task-001")
        .arg("task-003")
        .output()
        .expect("cli dep-add extra");
    assert!(cli_dep_extra.status.success());

    let cli_dep_remove = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("dep-remove")
        .arg("task-001")
        .arg("task-002")
        .output()
        .expect("cli dep-remove");
    assert!(cli_dep_remove.status.success());

    let _ = call_tool_text(
        &client,
        "add_dependency",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "dependency": "task-001"
        }),
    )
    .await;
    let _ = call_tool_text(
        &client,
        "remove_dependency",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "dependency": "task-001"
        }),
    )
    .await;

    // Notes + body + section
    let cli_note = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("note")
        .arg("task-001")
        .arg("cli note")
        .output()
        .expect("cli note");
    assert!(cli_note.status.success());

    let _ = call_tool_text(
        &client,
        "add_note",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "note": "mcp note"
        }),
    )
    .await;

    let cli_body = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("set-body")
        .arg("task-001")
        .arg("--text")
        .arg("Body via CLI")
        .output()
        .expect("cli set-body");
    assert!(cli_body.status.success());

    let _ = call_tool_text(
        &client,
        "set_body",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "body": "Body via MCP"
        }),
    )
    .await;

    let cli_section = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("set-section")
        .arg("task-001")
        .arg("Notes")
        .arg("--text")
        .arg("replaced")
        .output()
        .expect("cli set-section");
    assert!(cli_section.status.success());

    let _ = call_tool_text(
        &client,
        "set_section",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "section": "Notes",
            "content": "replaced"
        }),
    )
    .await;

    // Claim/release
    let cli_claim = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("claim")
        .arg("task-001")
        .arg("cli-user")
        .arg("--minutes")
        .arg("30")
        .output()
        .expect("cli claim");
    assert!(cli_claim.status.success());

    let _ = call_tool_text(
        &client,
        "claim_task",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002",
            "owner": "mcp-user",
            "minutes": 30
        }),
    )
    .await;

    let cli_release = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("release")
        .arg("task-001")
        .output()
        .expect("cli release");
    assert!(cli_release.status.success());

    let _ = call_tool_text(
        &client,
        "release_task",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "task_id": "task-002"
        }),
    )
    .await;

    // Discovered
    let cli_discovered = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("add-discovered")
        .arg("--from")
        .arg("task-001")
        .arg("--title")
        .arg("Discovered CLI")
        .arg("--labels")
        .arg("discovered")
        .output()
        .expect("cli add-discovered");
    assert!(cli_discovered.status.success());

    let _ = call_tool_text(
        &client,
        "add_discovered",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "from": "task-002",
            "title": "Discovered MCP",
            "labels": ["discovered"]
        }),
    )
    .await;

    // Session continuity
    let cli_checkpoint = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("checkpoint")
        .arg("--project")
        .arg("alpha")
        .arg("--id")
        .arg("20260204-130000")
        .output()
        .expect("cli checkpoint");
    assert!(cli_checkpoint.status.success());

    let _ = call_tool_text(
        &client,
        "resume",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "project": "alpha",
            "id": "20260204-130000",
            "format": "text"
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "working_set",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "project": "alpha",
            "tasks": ["task-001", "task-002"]
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "session_journal",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "project": "alpha",
            "task": "task-001",
            "next": "Review",
            "note": "ok"
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "checkpoint_diff",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "project": "alpha",
            "id": "20260204-130000",
            "format": "text"
        }),
    )
    .await;

    // Validate
    let cli_validate = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("validate")
        .output()
        .expect("cli validate");
    assert!(cli_validate.status.success());

    let _ = call_tool_text(
        &client,
        "validate",
        serde_json::json!({"root": temp.path().display().to_string()}),
    )
    .await;

    // Best practices + skills
    let skill_dir = temp
        .path()
        .join(".codex")
        .join("skills")
        .join("workmesh");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(skill_dir.join("SKILL.md"), "# WorkMesh skill\n")
        .expect("skill content");

    let best_practices = call_tool_text(
        &client,
        "best_practices",
        serde_json::json!({"root": temp.path().display().to_string(), "format": "text"}),
    )
    .await;
    assert!(best_practices.contains("dependencies"));

    let skill = call_tool_text(
        &client,
        "skill_content",
        serde_json::json!({"root": temp.path().display().to_string(), "name": "workmesh", "format": "text"}),
    )
    .await;
    assert!(skill.contains("WorkMesh skill"));

    let help = call_tool_text(
        &client,
        "help",
        serde_json::json!({"root": temp.path().display().to_string(), "format": "text"}),
    )
    .await;
    assert!(help.contains("workmesh MCP help"));

    let pm_skill = call_tool_text(
        &client,
        "project_management_skill",
        serde_json::json!({"root": temp.path().display().to_string(), "format": "text"}),
    )
    .await;
    assert!(pm_skill.contains("WorkMesh skill"));

    client.shut_down().await.expect("shutdown");

    let task = parse_task_file(&task_path).expect("parse task");
    assert_eq!(task.status, "In Progress");
    assert_eq!(task.priority, "P1");
    assert!(task.labels.contains(&"keep".to_string()));
    assert!(!task.labels.contains(&"docs".to_string()));
    assert!(task.dependencies.contains(&"task-003".to_string()));
    assert!(!task.dependencies.contains(&"task-002".to_string()));

    let task2 = parse_task_file(&task_path2).expect("parse task2");
    assert_eq!(task2.status, "Done");
    assert_eq!(task2.phase, "Phase2");
    assert!(task2.labels.contains(&"keep2".to_string()));
    assert!(!task2.labels.contains(&"infra".to_string()));
    assert!(task2.dependencies.is_empty());

    let task3 = parse_task_file(&task_path3).expect("parse task3");
    assert!(task3.labels.contains(&"seed".to_string()));

    let task4 = parse_task_file(&task_path4).expect("parse task4");
    assert!(task4.labels.contains(&"seed2".to_string()));
}

#[tokio::test]
async fn cli_and_mcp_project_scaffold_parity() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
    write_task(&tasks_dir, "task-001", "Seed", "To Do", &[]);

    let cli_init = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("project-init")
        .arg("alpha")
        .arg("--name")
        .arg("Alpha Project")
        .output()
        .expect("cli project-init");
    assert!(cli_init.status.success());

    let cli_quickstart = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("quickstart")
        .arg("beta")
        .arg("--name")
        .arg("Beta Project")
        .output()
        .expect("cli quickstart");
    assert!(cli_quickstart.status.success());

    let client = start_client(temp.path()).await;

    let _ = call_tool_text(
        &client,
        "project_init",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "project_id": "gamma",
            "name": "Gamma Project"
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "quickstart",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "project_id": "delta",
            "name": "Delta Project"
        }),
    )
    .await;

    client.shut_down().await.expect("shutdown");

    let alpha_docs = temp.path().join("docs").join("projects").join("alpha");
    let beta_docs = temp.path().join("docs").join("projects").join("beta");
    let gamma_docs = temp.path().join("docs").join("projects").join("gamma");
    let delta_docs = temp.path().join("docs").join("projects").join("delta");

    assert!(alpha_docs.join("README.md").is_file());
    assert!(beta_docs.join("README.md").is_file());
    assert!(gamma_docs.join("README.md").is_file());
    assert!(delta_docs.join("README.md").is_file());
}

#[test]
fn cli_best_practices_command() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
    write_task(&tasks_dir, "task-001", "Seed", "To Do", &[]);
    let output = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("best-practices")
        .output()
        .expect("cli best-practices");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(text.contains("Dependencies"));
}
