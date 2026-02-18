mod auth;
mod http;
mod model;
mod read_model;
mod state;
mod templates;
mod version;
mod ws;

use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use tracing::{info, warn};

use crate::auth::AuthConfig;
use crate::state::{spawn_refresh_loop, AppState, ServiceConfig};

#[derive(Debug, Parser)]
#[command(name = "workmesh-service", version = version::FULL, about = "WorkMesh monitoring service")]
struct Args {
    /// Host interface to bind (use 0.0.0.0 for LAN access)
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,

    /// Port to bind
    #[arg(long, default_value_t = 4747)]
    port: u16,

    /// Override WORKMESH_HOME location
    #[arg(long)]
    workmesh_home: Option<PathBuf>,

    /// Additional repo roots to include in scans (repeatable)
    #[arg(long = "scan-root")]
    scan_roots: Vec<PathBuf>,

    /// Refresh interval in milliseconds
    #[arg(long, default_value_t = 3000)]
    refresh_ms: u64,

    /// Access token (required for non-loopback binds)
    #[arg(long, env = "WORKMESH_SERVICE_TOKEN")]
    auth_token: Option<String>,

    /// Read access token from file
    #[arg(long)]
    auth_token_file: Option<PathBuf>,

    /// Try opening the dashboard in the default browser on startup
    #[arg(long)]
    open: bool,

    /// Emit logs as compact JSON
    #[arg(long)]
    json_log: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    init_tracing(args.json_log);

    let workmesh_home = resolve_workmesh_home(args.workmesh_home)
        .context("resolve workmesh home path for service")?;

    let auth_token = resolve_auth_token(args.auth_token, args.auth_token_file)
        .context("resolve service auth token")?;

    if !args.host.is_loopback() && auth_token.is_none() {
        return Err(anyhow!(
            "non-loopback bind requires auth token (use --auth-token, --auth-token-file, or WORKMESH_SERVICE_TOKEN)"
        ));
    }

    let auth = AuthConfig::from_plain_token(auth_token);
    let state = AppState::new(
        ServiceConfig {
            workmesh_home: workmesh_home.clone(),
            scan_roots: args.scan_roots,
            refresh_ms: args.refresh_ms.max(500),
        },
        auth,
    );

    spawn_refresh_loop(state.clone());

    let app = http::router(state);
    let addr = std::net::SocketAddr::new(args.host, args.port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {}", addr))?;

    let url = format!("http://{}:{}", args.host, args.port);
    info!("workmesh-service listening on {}", url);
    info!("workmesh_home={}", workmesh_home.display());

    if args.open {
        if let Err(err) = try_open_browser(&url) {
            warn!("unable to open browser automatically: {}", err);
        }
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serve workmesh-service")?;

    Ok(())
}

fn init_tracing(json_log: bool) {
    let default_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let filter = tracing_subscriber::EnvFilter::try_new(default_filter)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    if json_log {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .with_current_span(false)
            .with_target(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .compact()
            .with_target(false)
            .init();
    }
}

fn resolve_workmesh_home(override_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path);
    }
    workmesh_core::global_sessions::resolve_workmesh_home()
}

fn resolve_auth_token(inline: Option<String>, file: Option<PathBuf>) -> Result<Option<String>> {
    if let Some(value) = inline {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }

    if let Some(path) = file {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("read auth token file {}", path.display()))?;
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }

    Ok(None)
}

fn try_open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("spawn xdg-open")?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .context("spawn open")?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .context("spawn start")?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(anyhow!("auto-open is not supported on this platform"))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut stream) = signal(SignalKind::terminate()) {
            let _ = stream.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
