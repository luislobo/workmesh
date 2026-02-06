mod tools;
mod version;

use std::path::PathBuf;

use clap::Parser;
use rust_mcp_sdk::error::SdkResult;
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools,
};
use rust_mcp_sdk::{
    mcp_icon,
    mcp_server::{server_runtime, McpServerOptions},
    McpServer, StdioTransport, ToMcpServerHandler, TransportOptions,
};

use crate::tools::{McpContext, WorkmeshServerHandler};

#[derive(Parser)]
#[command(name = "workmesh-mcp", version = version::FULL)]
struct Args {
    /// Default backlog root for MCP tool calls.
    #[arg(long)]
    root: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> SdkResult<()> {
    let args = Args::parse();

    let server_details = InitializeResult {
        server_info: Implementation {
            name: "workmesh".into(),
            version: version::FULL.into(),
            title: Some("WorkMesh MCP Server".into()),
            description: Some("MCP server for Markdown-backed backlogs".into()),
            icons: vec![mcp_icon!(
                src = "https://raw.githubusercontent.com/rust-mcp-stack/rust-mcp-sdk/main/assets/rust-mcp-icon.png",
                mime_type = "image/png",
                sizes = ["128x128"],
                theme = "dark"
            )],
            website_url: Some("https://github.com/luislobo/workmesh".into()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: Some("WorkMesh MCP server".into()),
        protocol_version: ProtocolVersion::V2025_11_25.into(),
    };

    let transport = StdioTransport::new(TransportOptions::default())?;
    let handler = WorkmeshServerHandler {
        context: McpContext {
            default_root: args.root,
        },
    };

    let server = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
    });

    server.start().await
}
