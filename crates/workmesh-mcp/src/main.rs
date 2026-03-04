mod version;

use std::path::PathBuf;

use clap::Parser;
use rust_mcp_sdk::error::SdkResult;
use rust_mcp_sdk::{
    mcp_server::{server_runtime, McpServerOptions},
    McpServer, StdioTransport, ToMcpServerHandler, TransportOptions,
};

use workmesh_mcp_server::{build_server_details, McpContext, WorkmeshServerHandler};

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

    let server_details = build_server_details(version::FULL);

    let transport = StdioTransport::new(TransportOptions::default())?;
    let handler = WorkmeshServerHandler {
        context: McpContext {
            default_root: args.root,
            version_full: version::FULL.to_string(),
            server_label: "workmesh-mcp".to_string(),
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
