use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Output;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Once;
use std::sync::OnceLock;

use async_trait::async_trait;
use chrono::Local;
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

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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

fn assert_output_success(output: &Output, label: &str) {
    if output.status.success() {
        return;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!(
        "{label} failed: status={status:?}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}\n",
        label = label,
        status = output.status,
        stdout = stdout,
        stderr = stderr
    );
}

macro_rules! assert_output_ok {
    ($output:expr) => {
        assert_output_success(&$output, stringify!($output))
    };
}

fn cli() -> Command {
    static BUILD: Once = Once::new();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let root = std::path::Path::new(&manifest_dir)
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root");
    let profile_dir = cargo_profile_dir();
    let target_dir = cargo_target_dir();
    let candidate = profile_dir.join(exe_name("workmesh"));
    BUILD.call_once(|| {
        let status = Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("workmesh")
            .arg("--target-dir")
            .arg(&target_dir)
            // Coverage runs set LLVM_PROFILE_FILE; propagating it into nested `cargo build`
            // sometimes produces invalid `.profraw` artifacts that break `llvm-profdata merge`.
            .env_remove("LLVM_PROFILE_FILE")
            .current_dir(root)
            .status()
            .expect("build workmesh cli");
        assert!(status.success());
    });
    let mut cmd = Command::new(candidate);
    // Avoid interactive prompts (e.g. legacy backlog migration confirmation) in CI.
    cmd.stdin(Stdio::null());
    cmd.env("WORKMESH_NO_PROMPT", "1");
    cmd.env("RUST_BACKTRACE", "1");
    #[cfg(unix)]
    cmd.env("LLVM_PROFILE_FILE", "/dev/null");
    #[cfg(windows)]
    cmd.env("LLVM_PROFILE_FILE", "NUL");
    cmd
}

fn mcp_bin() -> PathBuf {
    // Cargo sets CARGO_BIN_EXE_* only for binaries built for the current package; when running
    // workspace-level tests with a custom --target-dir, it's more reliable to resolve relative to
    // the test executable and build into that same target dir on demand.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let root = std::path::Path::new(&manifest_dir)
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root");
    let profile_dir = cargo_profile_dir();
    let target_dir = cargo_target_dir();
    let candidate = profile_dir.join(exe_name("workmesh-mcp"));

    if candidate.exists() {
        return candidate;
    }

    let status = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg("workmesh-mcp")
        .arg("--target-dir")
        .arg(&target_dir)
        .env_remove("LLVM_PROFILE_FILE")
        .current_dir(root)
        .status()
        .expect("build workmesh-mcp");
    assert!(status.success());
    candidate
}

fn exe_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{}.exe", stem)
    } else {
        stem.to_string()
    }
}

fn cargo_profile_dir() -> PathBuf {
    // <target-dir>/<profile>/deps/<test-binary>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .expect("profile dir")
}

fn cargo_target_dir() -> PathBuf {
    cargo_profile_dir()
        .parent()
        .map(|p| p.to_path_buf())
        .expect("target dir")
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

async fn start_client(root: &Path) -> Arc<rust_mcp_sdk::mcp_client::ClientRuntime> {
    let mut env = std::collections::HashMap::new();
    // See note in `cli()`: prevent coverage flakiness from subprocess profile writes.
    #[cfg(unix)]
    env.insert("LLVM_PROFILE_FILE".to_string(), "/dev/null".to_string());
    #[cfg(windows)]
    env.insert("LLVM_PROFILE_FILE".to_string(), "NUL".to_string());
    let transport = StdioTransport::create_with_server_launch(
        mcp_bin().display().to_string(),
        vec!["--root".into(), root.display().to_string()],
        Some(env),
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

fn write_task(dir: &Path, id: &str, title: &str, status: &str, dependencies: &[&str]) -> PathBuf {
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

fn write_task_with_updated(
    dir: &Path,
    id: &str,
    title: &str,
    status: &str,
    updated: &str,
) -> PathBuf {
    let filename = format!("{} - {}.md", id, title.to_lowercase());
    let path = dir.join(filename);
    let content = format!(
        "---\nid: {id}\ntitle: {title}\nstatus: {status}\npriority: P2\nphase: Phase3\nupdated_date: {updated}\ndependencies: []\nlabels: []\nassignee: []\n---\n\n## Notes\n- initial\n",
        id = id,
        title = title,
        status = status,
        updated = updated
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
    let path = fake_plantuml_path(dir);
    let script = fake_plantuml_script();
    std::fs::write(&path, script).expect("write plantuml script");
    make_executable_best_effort(&path);
    path
}

#[tokio::test]
async fn cli_and_mcp_global_sessions_parity() {
    let _guard = env_lock().lock().expect("env lock");
    let home = TempDir::new().expect("home tempdir");
    std::env::set_var("WORKMESH_HOME", home.path());

    let repo = TempDir::new().expect("repo tempdir");
    let tasks_dir = repo.path().join("workmesh").join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
    write_task(&tasks_dir, "task-001", "Alpha", "To Do", &[]);

    let client = start_client(repo.path()).await;

    // MCP save -> CLI show
    let saved_text = call_tool_text(
        &client,
        "session_save",
        serde_json::json!({
            "objective": "Test objective (mcp)",
            "cwd": repo.path().display().to_string(),
            "format": "json"
        }),
    )
    .await;
    let saved_json: serde_json::Value =
        serde_json::from_str(&saved_text).expect("session_save json");
    let mcp_id = saved_json
        .get("id")
        .and_then(|v| v.as_str())
        .expect("id")
        .to_string();

    let show = cli()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("show")
        .arg(&mcp_id)
        .arg("--json")
        .output()
        .expect("cli session show");
    assert_output_ok!(show);
    let cli_show: serde_json::Value = serde_json::from_slice(&show.stdout).expect("cli json");
    assert_eq!(cli_show.get("id").and_then(|v| v.as_str()).unwrap(), mcp_id);

    // CLI save -> MCP show
    let cli_save = cli()
        .arg("--root")
        .arg(repo.path())
        .env("WORKMESH_HOME", home.path())
        .arg("session")
        .arg("save")
        .arg("--objective")
        .arg("Test objective (cli)")
        .arg("--cwd")
        .arg(repo.path())
        .arg("--json")
        .output()
        .expect("cli session save");
    assert_output_ok!(cli_save);
    let cli_save_json: serde_json::Value =
        serde_json::from_slice(&cli_save.stdout).expect("cli save json");
    let cli_id = cli_save_json
        .get("id")
        .and_then(|v| v.as_str())
        .expect("id")
        .to_string();

    let mcp_show_text = call_tool_text(
        &client,
        "session_show",
        serde_json::json!({
            "session_id": cli_id,
            "format": "json"
        }),
    )
    .await;
    let mcp_show_json: serde_json::Value =
        serde_json::from_str(&mcp_show_text).expect("session_show json");
    assert_eq!(
        mcp_show_json
            .get("objective")
            .and_then(|v| v.as_str())
            .unwrap(),
        "Test objective (cli)"
    );

    client.shut_down().await.expect("shutdown");
}

#[cfg(unix)]
fn fake_plantuml_path(dir: &Path) -> PathBuf {
    dir.join("fake-plantuml.sh")
}

#[cfg(windows)]
fn fake_plantuml_path(dir: &Path) -> PathBuf {
    dir.join("fake-plantuml.cmd")
}

#[cfg(unix)]
fn fake_plantuml_script() -> &'static str {
    "#!/bin/sh\ncat >/dev/null\necho \"<svg></svg>\"\n"
}

#[cfg(windows)]
fn fake_plantuml_script() -> &'static str {
    // `<` and `>` are special in batch files; caret-escape them so we print literal SVG.
    "@echo off\r\nmore >nul\r\necho ^<svg^>^</svg^>\r\n"
}

#[cfg(unix)]
fn make_executable_best_effort(path: &Path) {
    let mut perms = std::fs::metadata(path).expect("stat").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("chmod");
}

#[cfg(windows)]
fn make_executable_best_effort(_path: &Path) {
    // No-op: we'll invoke via `cmd /C`, so executable permissions are irrelevant.
}

fn plantuml_cmd_arg(script_path: &Path) -> String {
    #[cfg(windows)]
    {
        format!("cmd /C {}", script_path.display())
    }
    #[cfg(not(windows))]
    {
        script_path.display().to_string()
    }
}

fn ids_from_json(text: &str) -> BTreeSet<String> {
    let value: serde_json::Value = serde_json::from_str(text).expect("json array");
    value
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|item| {
            item.get("id")
                .and_then(|id| id.as_str())
                .map(|s| s.to_string())
        })
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
        .filter_map(|node| {
            node.get("id")
                .and_then(|id| id.as_str())
                .map(|s| s.to_string())
        })
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
    assert_output_ok!(cli_list);
    let cli_list_text = String::from_utf8_lossy(&cli_list.stdout).to_string();

    let cli_ready = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("ready")
        .arg("--json")
        .output()
        .expect("cli ready");
    assert_output_ok!(cli_ready);
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
    assert_eq!(
        ids_from_json(&cli_ready_text),
        ids_from_json(&mcp_ready_text)
    );
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
    assert_output_ok!(cli_show);
    let cli_show_value: serde_json::Value =
        serde_json::from_slice(&cli_show.stdout).expect("cli show json");

    let cli_next = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("next")
        .arg("--json")
        .output()
        .expect("cli next");
    assert_output_ok!(cli_next);
    let cli_next_value: serde_json::Value =
        serde_json::from_slice(&cli_next.stdout).expect("cli next json");

    let cli_stats = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("stats")
        .arg("--json")
        .output()
        .expect("cli stats");
    assert_output_ok!(cli_stats);
    let cli_stats_value: serde_json::Value =
        serde_json::from_slice(&cli_stats.stdout).expect("cli stats json");

    let cli_export = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("export")
        .arg("--pretty")
        .output()
        .expect("cli export");
    assert_output_ok!(cli_export);
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
    assert_output_ok!(cli_graph);
    let cli_graph_value: serde_json::Value =
        serde_json::from_slice(&cli_graph.stdout).expect("cli graph json");

    let cli_issues = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("issues-export")
        .output()
        .expect("cli issues");
    assert_output_ok!(cli_issues);
    let cli_issues_text = String::from_utf8_lossy(&cli_issues.stdout).to_string();

    let cli_index = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("index-rebuild")
        .arg("--json")
        .output()
        .expect("cli index");
    assert_output_ok!(cli_index);
    let cli_index_value: serde_json::Value =
        serde_json::from_slice(&cli_index.stdout).expect("cli index json");

    let cli_index_refresh = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("index-refresh")
        .arg("--json")
        .output()
        .expect("cli index refresh");
    assert_output_ok!(cli_index_refresh);
    let cli_index_refresh_value: serde_json::Value =
        serde_json::from_slice(&cli_index_refresh.stdout).expect("cli index refresh json");

    let cli_index_verify = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("index-verify")
        .arg("--json")
        .output()
        .expect("cli index verify");
    assert_output_ok!(cli_index_verify);
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
    assert_output_ok!(cli_gantt);
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
    assert_output_ok!(cli_gantt_file);
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
        .arg(plantuml_cmd_arg(&fake_plantuml))
        .output()
        .expect("cli gantt-svg");
    assert_output_ok!(cli_gantt_svg);
    let cli_gantt_svg_text = std::fs::read_to_string(&cli_gantt_svg_path).expect("gantt svg text");

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
    let mcp_gantt_file_value: serde_json::Value =
        serde_json::from_str(&mcp_gantt_file_text).unwrap();
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
            "plantuml_cmd": plantuml_cmd_arg(&fake_plantuml)
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

    assert_eq!(
        parse_lineset(&cli_issues_text),
        parse_lineset(&mcp_issues_text)
    );

    assert!(cli_index_value.get("entries").is_some());
    assert_eq!(
        cli_index_value.get("entries"),
        mcp_index_value.get("entries")
    );
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
        mcp_gantt_file_value
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    );
    assert_eq!(
        mcp_gantt_svg_path.display().to_string(),
        mcp_gantt_svg_value
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
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
    assert_output_ok!(cli_add);
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
    assert_output_ok!(cli_status);

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
    assert_output_ok!(cli_set_field);

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
    let task2_contents =
        std::fs::read_to_string(find_task_path(&tasks_dir, "task-002")).expect("read task-002");
    assert!(
        task2_contents.contains("updated_date:"),
        "set_status Done should set updated_date"
    );
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
    assert_output_ok!(cli_label);

    let cli_label_keep = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("label-add")
        .arg("task-001")
        .arg("keep")
        .output()
        .expect("cli label-add keep");
    assert_output_ok!(cli_label_keep);

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
    assert_output_ok!(cli_label_remove);

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
    assert_output_ok!(cli_dep);

    let cli_dep_extra = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("dep-add")
        .arg("task-001")
        .arg("task-003")
        .output()
        .expect("cli dep-add extra");
    assert_output_ok!(cli_dep_extra);

    let cli_dep_remove = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("dep-remove")
        .arg("task-001")
        .arg("task-002")
        .output()
        .expect("cli dep-remove");
    assert_output_ok!(cli_dep_remove);

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
    assert_output_ok!(cli_note);

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
    assert_output_ok!(cli_body);

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
    assert_output_ok!(cli_section);

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
    assert_output_ok!(cli_claim);

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
    assert_output_ok!(cli_release);

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
    assert_output_ok!(cli_discovered);

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
    assert_output_ok!(cli_checkpoint);

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
    assert_output_ok!(cli_validate);

    let _ = call_tool_text(
        &client,
        "validate",
        serde_json::json!({"root": temp.path().display().to_string()}),
    )
    .await;

    // Best practices + skills
    let skill_dir = temp.path().join(".codex").join("skills").join("workmesh");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(skill_dir.join("SKILL.md"), "# WorkMesh skill\n").expect("skill content");

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
    assert!(help.contains("Task kind"));

    let tool_info = call_tool_text(
        &client,
        "tool_info",
        serde_json::json!({"root": temp.path().display().to_string(), "name": "list_tasks", "format": "text"}),
    )
    .await;
    assert!(tool_info.contains("Tool: list_tasks"));
    assert!(tool_info.contains("\"name\": \"list_tasks\""));
    assert!(tool_info.contains("inputSchema"));

    // tool_info must cover every tool we advertise.
    let help_json = call_tool_text(
        &client,
        "help",
        serde_json::json!({"root": temp.path().display().to_string(), "format": "json"}),
    )
    .await;
    let help_value: serde_json::Value = serde_json::from_str(&help_json).expect("help json");
    let tools = help_value
        .get("tools")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for tool in tools {
        let Some(tool_name) = tool.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let info = call_tool_text(
            &client,
            "tool_info",
            serde_json::json!({"root": temp.path().display().to_string(), "name": tool_name, "format": "json"}),
        )
        .await;
        let value: serde_json::Value = serde_json::from_str(&info).expect("tool_info json");
        assert_eq!(
            value.get("ok").and_then(|v| v.as_bool()),
            Some(true),
            "tool_info failed for {tool_name}"
        );
    }

    let pm_skill = call_tool_text(
        &client,
        "project_management_skill",
        serde_json::json!({"root": temp.path().display().to_string(), "format": "text"}),
    )
    .await;
    assert!(pm_skill.contains("WorkMesh MCP Skill"));

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
async fn cli_and_mcp_focus_parity() {
    let temp = TempDir::new().expect("tempdir");
    let repo_root = temp.path();
    std::fs::create_dir_all(repo_root.join("workmesh").join("tasks")).expect("tasks dir");
    std::fs::create_dir_all(
        repo_root
            .join("docs")
            .join("projects")
            .join("alpha")
            .join("updates"),
    )
    .expect("docs dir");

    // MCP: set focus
    let client = start_client(repo_root).await;
    let set = call_tool_text(
        &client,
        "focus_set",
        serde_json::json!({
            "root": repo_root.display().to_string(),
            "project_id": "alpha",
            "epic_id": "task-039",
            "objective": "Ship focus",
            "tasks": ["task-001","task-002"],
            "format": "json"
        }),
    )
    .await;
    let set_json: serde_json::Value = serde_json::from_str(&set).expect("json");
    assert!(set_json
        .get("ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));

    // CLI: show focus should reflect set values
    let show = cli()
        .arg("--root")
        .arg(repo_root)
        .arg("focus")
        .arg("show")
        .arg("--json")
        .output()
        .expect("cli focus show");
    assert_output_ok!(show);
    let parsed: serde_json::Value = serde_json::from_slice(&show.stdout).expect("json");
    let focus = parsed
        .get("focus")
        .expect("focus")
        .as_object()
        .expect("obj");
    assert_eq!(
        focus.get("project_id").and_then(|v| v.as_str()).unwrap(),
        "alpha"
    );
    assert_eq!(
        focus.get("epic_id").and_then(|v| v.as_str()).unwrap(),
        "task-039"
    );
    assert_eq!(
        focus.get("objective").and_then(|v| v.as_str()).unwrap(),
        "Ship focus"
    );

    // CLI: clear focus
    let cleared = cli()
        .arg("--root")
        .arg(repo_root)
        .arg("focus")
        .arg("clear")
        .arg("--json")
        .output()
        .expect("cli focus clear");
    assert_output_ok!(cleared);
    let cleared_json: serde_json::Value = serde_json::from_slice(&cleared.stdout).expect("json");
    assert!(cleared_json
        .get("cleared")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));

    // MCP: show focus should now be null
    let shown = call_tool_text(
        &client,
        "focus_show",
        serde_json::json!({"root": repo_root.display().to_string(), "format": "json"}),
    )
    .await;
    let parsed: serde_json::Value = serde_json::from_str(&shown).expect("json");
    assert!(parsed.get("focus").unwrap().is_null());

    client.shut_down().await.expect("shutdown");
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
    assert_output_ok!(cli_init);

    let cli_quickstart = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("quickstart")
        .arg("beta")
        .arg("--name")
        .arg("Beta Project")
        .output()
        .expect("cli quickstart");
    assert_output_ok!(cli_quickstart);

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
    assert_output_ok!(output);
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(text.contains("Dependencies"));
}

#[tokio::test]
async fn cli_and_mcp_migrate_archive_parity() {
    let temp = TempDir::new().expect("tempdir");
    let legacy_tasks = temp.path().join("backlog").join("tasks");
    std::fs::create_dir_all(&legacy_tasks).expect("legacy tasks");
    let today = Local::now().format("%Y-%m-%d %H:%M").to_string();
    write_task_with_updated(&legacy_tasks, "task-001", "Legacy", "Done", &today);

    let cli_migrate = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("migrate")
        .arg("--yes")
        .output()
        .expect("cli migrate");
    assert_output_ok!(cli_migrate);

    let workmesh_tasks = temp.path().join("workmesh").join("tasks");
    assert!(workmesh_tasks.is_dir());

    let cli_archive = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("archive")
        .arg("--before")
        .arg("2999-01-01")
        .arg("--json")
        .output()
        .expect("cli archive");
    assert_output_ok!(cli_archive);
    let archive_root = temp.path().join("workmesh").join("archive");
    assert!(archive_root.is_dir());

    let temp2 = TempDir::new().expect("tempdir2");
    let legacy_tasks2 = temp2.path().join("backlog").join("tasks");
    std::fs::create_dir_all(&legacy_tasks2).expect("legacy tasks2");
    write_task_with_updated(&legacy_tasks2, "task-002", "Legacy", "Done", &today);

    let client = start_client(temp2.path()).await;
    let _ = call_tool_text(
        &client,
        "migrate_backlog",
        serde_json::json!({
            "root": temp2.path().display().to_string(),
            "to": "workmesh"
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "archive_tasks",
        serde_json::json!({
            "root": temp2.path().display().to_string(),
            "before": "2999-01-01",
            "status": "Done"
        }),
    )
    .await;
    client.shut_down().await.expect("shutdown");

    let archive_root2 = temp2.path().join("workmesh").join("archive");
    assert!(archive_root2.is_dir());
}

#[tokio::test]
async fn cli_and_mcp_bulk_ops_parity() {
    let temp = TempDir::new().expect("tempdir");
    let backlog_dir = temp.path().join("backlog");
    let tasks_dir = backlog_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");

    let task_path1 = write_task(&tasks_dir, "task-001", "Alpha", "To Do", &[]);
    let task_path2 = write_task(&tasks_dir, "task-002", "Beta", "To Do", &[]);
    let task_path3 = write_task(&tasks_dir, "task-003", "Gamma", "To Do", &[]);

    let cli_bulk_status = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("bulk")
        .arg("set-status")
        .arg("--tasks")
        .arg("task-001,task-002")
        .arg("--status")
        .arg("In Progress")
        .arg("--json")
        .output()
        .expect("cli bulk status");
    assert_output_ok!(cli_bulk_status);

    let cli_bulk_field = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("bulk-set-field")
        .arg("--tasks")
        .arg("task-001,task-002")
        .arg("--field")
        .arg("priority")
        .arg("--value")
        .arg("P1")
        .arg("--json")
        .output()
        .expect("cli bulk field");
    assert_output_ok!(cli_bulk_field);

    let cli_bulk_label_add = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("bulk")
        .arg("label-add")
        .arg("--tasks")
        .arg("task-001,task-002")
        .arg("--label")
        .arg("bulk")
        .arg("--json")
        .output()
        .expect("cli bulk label add");
    assert_output_ok!(cli_bulk_label_add);

    let cli_bulk_label_remove = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("bulk-label-remove")
        .arg("--tasks")
        .arg("task-001,task-002")
        .arg("--label")
        .arg("bulk")
        .arg("--json")
        .output()
        .expect("cli bulk label remove");
    assert_output_ok!(cli_bulk_label_remove);

    let cli_bulk_dep_add = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("bulk-dep-add")
        .arg("--tasks")
        .arg("task-001,task-002")
        .arg("--dependency")
        .arg("task-003")
        .arg("--json")
        .output()
        .expect("cli bulk dep add");
    assert_output_ok!(cli_bulk_dep_add);

    let cli_bulk_dep_remove = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("bulk-dep-remove")
        .arg("--tasks")
        .arg("task-001,task-002")
        .arg("--dependency")
        .arg("task-003")
        .arg("--json")
        .output()
        .expect("cli bulk dep remove");
    assert_output_ok!(cli_bulk_dep_remove);

    let cli_bulk_note = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("bulk-note")
        .arg("--tasks")
        .arg("task-001,task-002")
        .arg("--note")
        .arg("cli bulk note")
        .arg("--json")
        .output()
        .expect("cli bulk note");
    assert_output_ok!(cli_bulk_note);

    let client = start_client(temp.path()).await;

    let _ = call_tool_text(
        &client,
        "bulk_set_status",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "tasks": ["task-002", "task-003"],
            "status": "Done"
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "bulk_set_field",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "tasks": ["task-002", "task-003"],
            "field": "phase",
            "value": "Phase2"
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "bulk_add_label",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "tasks": ["task-002", "task-003"],
            "label": "mcp"
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "bulk_remove_label",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "tasks": ["task-002", "task-003"],
            "label": "mcp"
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "bulk_add_dependency",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "tasks": ["task-002", "task-003"],
            "dependency": "task-001"
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "bulk_remove_dependency",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "tasks": ["task-002", "task-003"],
            "dependency": "task-001"
        }),
    )
    .await;

    let _ = call_tool_text(
        &client,
        "bulk_add_note",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "tasks": ["task-002", "task-003"],
            "note": "mcp bulk note"
        }),
    )
    .await;

    client.shut_down().await.expect("shutdown");

    let task1 = parse_task_file(&task_path1).expect("task1");
    let task2 = parse_task_file(&task_path2).expect("task2");
    let task3 = parse_task_file(&task_path3).expect("task3");

    assert_eq!(task1.status, "In Progress");
    assert_eq!(task1.priority, "P1");
    assert!(task1.dependencies.is_empty());
    assert!(task1.body.contains("cli bulk note"));

    assert_eq!(task2.status, "Done");
    assert_eq!(task2.priority, "P1");
    assert_eq!(task2.phase, "Phase2");
    assert!(task2.dependencies.is_empty());
    assert!(task2.body.contains("cli bulk note"));
    assert!(task2.body.contains("mcp bulk note"));

    assert_eq!(task3.status, "Done");
    assert_eq!(task3.phase, "Phase2");
    assert!(task3.dependencies.is_empty());
    assert!(task3.body.contains("mcp bulk note"));
}

#[tokio::test]
async fn cli_and_mcp_truth_workflow_parity() {
    let temp = TempDir::new().expect("tempdir");
    let tasks_dir = temp.path().join("workmesh").join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
    write_task(&tasks_dir, "task-main-001", "Epic", "In Progress", &[]);
    std::fs::write(
        temp.path().join("workmesh").join("context.json"),
        r#"{"version":1,"project_id":"workmesh","objective":"Ship truth","scope":{"mode":"epic","epic_id":"task-main-001","task_ids":[]},"updated_at":"2026-02-13T00:00:00Z"}"#,
    )
    .expect("context");

    let cli_propose = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("truth")
        .arg("propose")
        .arg("--title")
        .arg("Use append-only truth events")
        .arg("--statement")
        .arg("Truth records are immutable")
        .arg("--project")
        .arg("workmesh")
        .arg("--epic")
        .arg("task-main-001")
        .arg("--json")
        .output()
        .expect("cli propose");
    assert_output_ok!(cli_propose);
    let cli_proposed: serde_json::Value =
        serde_json::from_slice(&cli_propose.stdout).expect("cli propose json");
    let truth_a = cli_proposed["id"].as_str().expect("truth_a").to_string();

    let client = start_client(temp.path()).await;

    let accepted_text = call_tool_text(
        &client,
        "truth_accept",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "truth_id": truth_a,
            "note": "approved",
            "format": "json"
        }),
    )
    .await;
    let accepted: serde_json::Value = serde_json::from_str(&accepted_text).expect("accept json");
    assert_eq!(accepted["state"], "accepted");

    let proposed_b_text = call_tool_text(
        &client,
        "truth_propose",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "title": "Use current projection",
            "statement": "Current truth view is rebuilt from events",
            "project_id": "workmesh",
            "epic_id": "task-main-001",
            "format": "json"
        }),
    )
    .await;
    let proposed_b: serde_json::Value =
        serde_json::from_str(&proposed_b_text).expect("propose b json");
    let truth_b = proposed_b["id"].as_str().expect("truth_b").to_string();

    let _ = call_tool_text(
        &client,
        "truth_accept",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "truth_id": truth_b,
            "format": "json"
        }),
    )
    .await;

    let cli_supersede = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("truth")
        .arg("supersede")
        .arg(&accepted["id"].as_str().unwrap())
        .arg("--by")
        .arg(&truth_b)
        .arg("--reason")
        .arg("replacement adopted")
        .arg("--json")
        .output()
        .expect("cli supersede");
    assert_output_ok!(cli_supersede);

    let cli_list = cli()
        .arg("--root")
        .arg(temp.path())
        .arg("truth")
        .arg("list")
        .arg("--state")
        .arg("accepted")
        .arg("--project")
        .arg("workmesh")
        .arg("--epic")
        .arg("task-main-001")
        .arg("--json")
        .output()
        .expect("cli list");
    assert_output_ok!(cli_list);
    let cli_listed: serde_json::Value = serde_json::from_slice(&cli_list.stdout).expect("json");

    let mcp_list_text = call_tool_text(
        &client,
        "truth_list",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "states": ["accepted"],
            "project_id": "workmesh",
            "epic_id": "task-main-001",
            "format": "json"
        }),
    )
    .await;
    let mcp_listed: serde_json::Value = serde_json::from_str(&mcp_list_text).expect("mcp list");

    let cli_ids = cli_listed
        .as_array()
        .expect("cli array")
        .iter()
        .filter_map(|entry| entry["id"].as_str().map(|v| v.to_string()))
        .collect::<BTreeSet<_>>();
    let mcp_ids = mcp_listed
        .as_array()
        .expect("mcp array")
        .iter()
        .filter_map(|entry| entry["id"].as_str().map(|v| v.to_string()))
        .collect::<BTreeSet<_>>();

    assert_eq!(cli_ids, mcp_ids);
    assert!(cli_ids.contains(&truth_b));

    let validate_text = call_tool_text(
        &client,
        "truth_validate",
        serde_json::json!({
            "root": temp.path().display().to_string(),
            "format": "json"
        }),
    )
    .await;
    let validated: serde_json::Value = serde_json::from_str(&validate_text).expect("validate");
    assert_eq!(validated["ok"], true);

    client.shut_down().await.expect("shutdown");
}
