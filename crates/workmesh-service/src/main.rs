mod config;
mod mcp_http;
mod server;
mod toolhost;
mod version;

use std::{net::IpAddr, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use config::{load_config, CliOverrides};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "workmesh-service", version = version::FULL)]
struct Args {
    /// Optional path to a service TOML config file.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Host/IP to bind the HTTP service.
    #[arg(long)]
    host: Option<IpAddr>,

    /// Port to bind the HTTP service.
    #[arg(long)]
    port: Option<u16>,

    /// Log filter (for example: info,debug,trace).
    #[arg(long)]
    log_filter: Option<String>,

    /// Bearer token required for authenticated endpoints.
    #[arg(long)]
    auth_token: Option<String>,

    /// Maximum HTTP request body size in bytes.
    #[arg(long)]
    max_body_bytes: Option<usize>,

    /// Request timeout in milliseconds.
    #[arg(long)]
    request_timeout_ms: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let overrides = CliOverrides::new(
        args.host,
        args.port,
        args.log_filter,
        args.auth_token,
        args.max_body_bytes,
        args.request_timeout_ms,
    );
    let config = load_config(args.config.as_deref(), &overrides)?;

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_new(config.log_filter.clone())
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    server::run(config, args.config, overrides).await
}
