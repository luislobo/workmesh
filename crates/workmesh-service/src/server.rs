use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::Instant,
};

use anyhow::{bail, Context, Result};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::net::TcpListener;
use tracing::info;

use crate::{
    config::{load_config, CliOverrides, ServiceConfig},
    mcp_http,
    version::FULL,
};

#[derive(Debug)]
pub struct ServiceState {
    started_at: DateTime<Utc>,
    started_instant: Instant,
    config_version: AtomicU64,
    ready: AtomicBool,
    request_count: AtomicU64,
    auth_token: RwLock<Option<String>>,
    max_body_bytes: usize,
    request_timeout_ms: u64,
    bound_host: std::net::IpAddr,
    bound_port: u16,
    config_path: Option<PathBuf>,
    cli_overrides: CliOverrides,
    pending_restart: AtomicBool,
}

impl ServiceState {
    fn new(
        config: &ServiceConfig,
        config_path: Option<PathBuf>,
        cli_overrides: CliOverrides,
    ) -> Self {
        Self {
            started_at: Utc::now(),
            started_instant: Instant::now(),
            config_version: AtomicU64::new(config.config_version),
            ready: AtomicBool::new(true),
            request_count: AtomicU64::new(0),
            auth_token: RwLock::new(config.auth_token.clone()),
            max_body_bytes: config.max_body_bytes,
            request_timeout_ms: config.request_timeout_ms,
            bound_host: config.host,
            bound_port: config.port,
            config_path,
            cli_overrides,
            pending_restart: AtomicBool::new(false),
        }
    }

    fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    fn auth_enabled(&self) -> bool {
        self.auth_token
            .read()
            .ok()
            .and_then(|value| value.clone())
            .is_some()
    }

    fn auth_token_value(&self) -> Option<String> {
        self.auth_token.read().ok().and_then(|value| value.clone())
    }
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    ok: bool,
    service: &'static str,
    version: &'static str,
    started_at: String,
    uptime_seconds: u64,
    config_version: u64,
    ready: bool,
    auth_enabled: bool,
    request_count: u64,
    max_body_bytes: usize,
    request_timeout_ms: u64,
    pending_restart: bool,
}

#[derive(Debug, Serialize)]
struct MetricsResponse {
    service: &'static str,
    request_count: u64,
    uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
struct ReloadResponse {
    ok: bool,
    config_version: u64,
    providers_loaded: usize,
    pending_restart: bool,
    message: String,
}

pub fn build_router(state: Arc<ServiceState>) -> Router {
    Router::new()
        .route("/v1/healthz", get(healthz))
        .route("/v1/readyz", get(readyz))
        .route("/v1/status", get(status))
        .route("/v1/metrics", get(metrics))
        .route("/v1/providers", get(mcp_http::list_providers))
        .route("/v1/mcp/invoke", post(mcp_http::invoke))
        .route("/v1/admin/reload", post(reload))
        .layer(DefaultBodyLimit::max(state.max_body_bytes))
        .layer(from_fn_with_state(state.clone(), request_guard))
        .with_state(state)
}

pub async fn run(
    config: ServiceConfig,
    config_path: Option<PathBuf>,
    cli_overrides: CliOverrides,
) -> Result<()> {
    if !config.host.is_loopback() && config.auth_token.is_none() {
        bail!(
            "non-localhost bind requires --auth-token or auth_token in config for LAN-safe access"
        );
    }

    let addr = SocketAddr::from((config.host, config.port));
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {}", addr))?;

    let state = Arc::new(ServiceState::new(&config, config_path, cli_overrides));
    let app = build_router(state);
    info!(
        "workmesh-service listening on {} (auth_enabled={})",
        addr,
        config.auth_token.is_some()
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("service runtime failed")
}

async fn request_guard(
    State(state): State<Arc<ServiceState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    state.request_count.fetch_add(1, Ordering::Relaxed);
    if state.auth_enabled() && requires_auth(request.uri().path()) {
        let token = state.auth_token_value().unwrap_or_default();
        if !is_authorized(request.headers().get(AUTHORIZATION), &token) {
            return unauthorized_response();
        }
    }
    next.run(request).await
}

fn requires_auth(path: &str) -> bool {
    !matches!(path, "/v1/healthz" | "/v1/readyz")
}

fn is_authorized(header: Option<&axum::http::HeaderValue>, expected_token: &str) -> bool {
    let Some(header) = header.and_then(|value| value.to_str().ok()) else {
        return false;
    };
    let Some(value) = header.strip_prefix("Bearer ") else {
        return false;
    };
    value.trim() == expected_token
}

fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "ok": false,
            "error": {
                "code": "UNAUTHORIZED",
                "message": "missing or invalid bearer token",
                "details": {},
            }
        })),
    )
        .into_response()
}

async fn healthz() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            ok: true,
            service: "workmesh-service",
            version: FULL,
        }),
    )
}

async fn readyz(State(state): State<Arc<ServiceState>>) -> impl IntoResponse {
    if state.is_ready() {
        (
            StatusCode::OK,
            Json(HealthResponse {
                ok: true,
                service: "workmesh-service",
                version: FULL,
            }),
        )
            .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                ok: false,
                service: "workmesh-service",
                version: FULL,
            }),
        )
            .into_response()
    }
}

async fn status(State(state): State<Arc<ServiceState>>) -> impl IntoResponse {
    let response = StatusResponse {
        ok: true,
        service: "workmesh-service",
        version: FULL,
        started_at: state.started_at.to_rfc3339(),
        uptime_seconds: state.started_instant.elapsed().as_secs(),
        config_version: state.config_version.load(Ordering::Relaxed),
        ready: state.is_ready(),
        auth_enabled: state.auth_enabled(),
        request_count: state.request_count.load(Ordering::Relaxed),
        max_body_bytes: state.max_body_bytes,
        request_timeout_ms: state.request_timeout_ms,
        pending_restart: state.pending_restart.load(Ordering::Relaxed),
    };
    (StatusCode::OK, Json(response))
}

async fn metrics(State(state): State<Arc<ServiceState>>) -> impl IntoResponse {
    let response = MetricsResponse {
        service: "workmesh-service",
        request_count: state.request_count.load(Ordering::Relaxed),
        uptime_seconds: state.started_instant.elapsed().as_secs(),
    };
    (StatusCode::OK, Json(response))
}

async fn reload(State(state): State<Arc<ServiceState>>) -> impl IntoResponse {
    let Some(config_path) = state.config_path.clone() else {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "ok": false,
                "error": {
                    "code": "CONFLICT",
                    "message": "reload requires a config file path (--config)",
                    "details": {}
                }
            })),
        )
            .into_response();
    };

    let loaded = match load_config(Some(config_path.as_path()), &state.cli_overrides) {
        Ok(config) => config,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "ok": false,
                    "error": {
                        "code": "INVALID_ARGUMENT",
                        "message": format!("reload failed: {}", err),
                        "details": { "config_path": config_path.display().to_string() }
                    }
                })),
            )
                .into_response();
        }
    };

    let pending_restart = loaded.host != state.bound_host
        || loaded.port != state.bound_port
        || loaded.max_body_bytes != state.max_body_bytes
        || loaded.request_timeout_ms != state.request_timeout_ms;

    if let Ok(mut auth_token) = state.auth_token.write() {
        *auth_token = loaded.auth_token;
    }

    let providers_loaded = match mcp_http::reload_tool_host() {
        Ok(count) => count,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "ok": false,
                    "error": {
                        "code": "INTERNAL",
                        "message": format!("provider reload failed: {}", err),
                        "details": {}
                    }
                })),
            )
                .into_response();
        }
    };

    state
        .pending_restart
        .store(pending_restart, Ordering::Relaxed);
    let config_version = state.config_version.fetch_add(1, Ordering::Relaxed) + 1;

    (
        StatusCode::OK,
        Json(ReloadResponse {
            ok: true,
            config_version,
            providers_loaded,
            pending_restart,
            message: if pending_restart {
                "reload applied; restart required for host/port/body-limit/timeout changes"
                    .to_string()
            } else {
                "reload applied".to_string()
            },
        }),
    )
        .into_response()
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::{body::Body, http::Request};
    use serde_json::Value;
    use tower::ServiceExt;

    fn test_config() -> ServiceConfig {
        ServiceConfig::default()
    }

    fn state_for(config: &ServiceConfig) -> Arc<ServiceState> {
        Arc::new(ServiceState::new(
            config,
            None,
            CliOverrides::new(None, None, None, None, None, None),
        ))
    }

    #[tokio::test]
    async fn healthz_returns_ok_payload() {
        let config = test_config();
        let state = state_for(&config);
        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/healthz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("bytes");
        let json: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["ok"], true);
        assert_eq!(json["service"], "workmesh-service");
    }

    #[tokio::test]
    async fn readyz_returns_ok_when_state_ready() {
        let config = test_config();
        let state = state_for(&config);
        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/readyz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn status_returns_runtime_details() {
        let mut config = test_config();
        config.config_version = 7;
        let state = state_for(&config);
        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/status")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("bytes");
        let json: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["config_version"], 7);
        assert_eq!(json["ready"], true);
    }

    #[tokio::test]
    async fn auth_enabled_routes_require_bearer_token() {
        let mut config = test_config();
        config.auth_token = Some("token123".to_string());
        let state = state_for(&config);
        let app = build_router(state);

        let unauthorized = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/status")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let authorized = app
            .oneshot(
                Request::builder()
                    .uri("/v1/status")
                    .header("Authorization", "Bearer token123")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(authorized.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn reload_without_config_path_returns_conflict() {
        let config = test_config();
        let state = state_for(&config);
        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/admin/reload")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
