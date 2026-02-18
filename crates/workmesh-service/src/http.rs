use axum::extract::{Form, Path as AxumPath, Query, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::auth::{is_authorized, AUTH_COOKIE_NAME};
use crate::model::{RepoView, SessionView, WorkstreamView, WorktreeView};
use crate::state::AppState;
use crate::templates::{card, layout, login_page, section, table};
use crate::ws::ws_handler;

const APP_CSS: &str = include_str!("assets/app.css");
const APP_JS: &str = include_str!("assets/app.js");

#[derive(Debug, Default, Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct NextQuery {
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    token: String,
    next: Option<String>,
}

pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/", get(dashboard_page))
        .route("/sessions", get(sessions_page))
        .route("/workstreams", get(workstreams_page))
        .route("/worktrees", get(worktrees_page))
        .route("/repos", get(repos_page))
        .route("/ws", get(ws_handler))
        .route("/api/v1/summary", get(api_summary))
        .route("/api/v1/sessions", get(api_sessions))
        .route("/api/v1/sessions/:id", get(api_session_by_id))
        .route("/api/v1/workstreams", get(api_workstreams))
        .route("/api/v1/workstreams/:id", get(api_workstream_by_id))
        .route("/api/v1/worktrees", get(api_worktrees))
        .route("/api/v1/repos", get(api_repos))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .route("/healthz", get(healthz))
        .route("/login", get(login_view))
        .route("/auth/login", post(login_action))
        .route("/auth/logout", post(logout_action))
        .route("/assets/app.css", get(app_css))
        .route("/assets/app.js", get(app_js))
        .merge(protected)
        .with_state(state)
}

async fn auth_middleware(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if is_authorized(request.headers(), &state.auth) {
        return next.run(request).await;
    }

    let path = request.uri().path().to_string();
    if path.starts_with("/api/") || path == "/ws" {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    Redirect::to(&format!("/login?next={}", path)).into_response()
}

async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    let snapshot = state.snapshot().await;
    Json(serde_json::json!({
        "ok": true,
        "generated_at": snapshot.generated_at,
        "repos_tracked": snapshot.summary.repos_tracked,
        "warnings": snapshot.warnings,
    }))
}

async fn login_view(Query(query): Query<NextQuery>) -> Html<String> {
    let next = sanitize_next(query.next.as_deref());
    Html(login_page(None, &next))
}

async fn login_action(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    let next = sanitize_next(form.next.as_deref());

    if !state.auth.is_required() {
        return Redirect::to(&next).into_response();
    }

    if !state.auth.verify_plain_token(&form.token) {
        return Html(login_page(Some("Invalid token"), &next)).into_response();
    }

    let cookie_value = state.auth.cookie_value().unwrap_or_default();
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400",
        AUTH_COOKIE_NAME, cookie_value
    );

    let mut response = Redirect::to(&next).into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

async fn logout_action() -> Response {
    let cookie = format!(
        "{}=deleted; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        AUTH_COOKIE_NAME
    );
    let mut response = Redirect::to("/login").into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

async fn app_css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], APP_CSS)
}

async fn app_js() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        APP_JS,
    )
}

async fn dashboard_page(State(state): State<AppState>) -> Html<String> {
    let snapshot = state.snapshot().await;
    let mut cards = String::new();
    cards.push_str("<div class=\"cards\">");
    cards.push_str(&card("Active Sessions", snapshot.summary.active_sessions));
    cards.push_str(&card(
        "Active Workstreams",
        snapshot.summary.active_workstreams,
    ));
    cards.push_str(&card("Open Worktrees", snapshot.summary.open_worktrees));
    cards.push_str(&card("Repos Tracked", snapshot.summary.repos_tracked));
    cards.push_str("</div>");

    let warning_rows: Vec<Vec<String>> = if snapshot.warnings.is_empty() {
        vec![vec!["No warnings".to_string()]]
    } else {
        snapshot
            .warnings
            .iter()
            .map(|warning| vec![warning.clone()])
            .collect()
    };

    let recent_sessions = snapshot
        .sessions
        .iter()
        .take(10)
        .map(|session| {
            vec![
                session.id.clone(),
                session.updated_at.clone(),
                session.objective.clone().unwrap_or_else(|| "-".to_string()),
                session
                    .workstream_id
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();

    let mut body = String::new();
    body.push_str(&cards);
    body.push_str(&section("Warnings", &table(&["Warning"], &warning_rows)));
    body.push_str(&section(
        "Recent Sessions",
        &table(
            &["Session", "Updated", "Objective", "Workstream"],
            &recent_sessions,
        ),
    ));

    Html(layout("Dashboard", &body, state.config.refresh_ms))
}

async fn sessions_page(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Html<String> {
    let snapshot = state.snapshot().await;
    let rows = filter_sessions(&snapshot.sessions, query.q.as_deref())
        .iter()
        .map(|session| {
            vec![
                session.id.clone(),
                session.updated_at.clone(),
                session.objective.clone().unwrap_or_else(|| "-".to_string()),
                session.repo_root.clone().unwrap_or_else(|| "-".to_string()),
                session
                    .worktree_path
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                session
                    .workstream_id
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();

    let content = table(
        &[
            "Session",
            "Updated",
            "Objective",
            "Repo",
            "Worktree",
            "Workstream",
        ],
        &rows,
    );
    Html(layout("Sessions", &content, state.config.refresh_ms))
}

async fn workstreams_page(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Html<String> {
    let snapshot = state.snapshot().await;
    let rows = filter_workstreams(&snapshot.workstreams, query.q.as_deref())
        .iter()
        .map(|stream| {
            vec![
                stream.id.clone(),
                stream.key.clone().unwrap_or_else(|| "-".to_string()),
                stream.name.clone(),
                stream.status.clone(),
                stream.repo_root.clone(),
                stream
                    .worktree_path
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                stream.session_id.clone().unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();

    let content = table(
        &["ID", "Key", "Name", "Status", "Repo", "Worktree", "Session"],
        &rows,
    );
    Html(layout("Workstreams", &content, state.config.refresh_ms))
}

async fn worktrees_page(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Html<String> {
    let snapshot = state.snapshot().await;
    let rows = filter_worktrees(&snapshot.worktrees, query.q.as_deref())
        .iter()
        .map(|worktree| {
            vec![
                worktree.id.clone().unwrap_or_else(|| "-".to_string()),
                worktree.path.clone(),
                worktree
                    .repo_root
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                worktree.branch.clone().unwrap_or_else(|| "-".to_string()),
                worktree
                    .attached_session_id
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                worktree.in_git.to_string(),
                worktree.exists.to_string(),
                if worktree.issues.is_empty() {
                    "-".to_string()
                } else {
                    worktree.issues.join(",")
                },
            ]
        })
        .collect::<Vec<_>>();

    let content = table(
        &[
            "ID", "Path", "Repo", "Branch", "Session", "In Git", "Exists", "Issues",
        ],
        &rows,
    );
    Html(layout("Worktrees", &content, state.config.refresh_ms))
}

async fn repos_page(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Html<String> {
    let snapshot = state.snapshot().await;
    let rows = filter_repos(&snapshot.repos, query.q.as_deref())
        .iter()
        .map(|repo| {
            vec![
                repo.repo_root.clone(),
                repo.workstream_count.to_string(),
                repo.active_workstream_count.to_string(),
                repo.worktree_count.to_string(),
                repo.session_count.to_string(),
                repo.last_activity_at
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();

    let content = table(
        &[
            "Repo",
            "Workstreams",
            "Active",
            "Worktrees",
            "Sessions",
            "Last Activity",
        ],
        &rows,
    );
    Html(layout("Repos", &content, state.config.refresh_ms))
}

async fn api_summary(State(state): State<AppState>) -> Json<serde_json::Value> {
    let snapshot = state.snapshot().await;
    Json(serde_json::json!({
        "summary": snapshot.summary,
        "generated_at": snapshot.generated_at,
        "current_session_id": snapshot.current_session_id,
        "warnings": snapshot.warnings,
    }))
}

async fn api_sessions(State(state): State<AppState>) -> Json<serde_json::Value> {
    let snapshot = state.snapshot().await;
    Json(serde_json::json!({
        "sessions": snapshot.sessions,
        "generated_at": snapshot.generated_at,
        "warnings": snapshot.warnings,
    }))
}

async fn api_session_by_id(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let snapshot = state.snapshot().await;
    if let Some(session) = snapshot.sessions.into_iter().find(|item| item.id == id) {
        return Json(serde_json::json!({"session": session})).into_response();
    }
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "session not found"})),
    )
        .into_response()
}

async fn api_workstreams(State(state): State<AppState>) -> Json<serde_json::Value> {
    let snapshot = state.snapshot().await;
    Json(serde_json::json!({
        "workstreams": snapshot.workstreams,
        "generated_at": snapshot.generated_at,
        "warnings": snapshot.warnings,
    }))
}

async fn api_workstream_by_id(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let snapshot = state.snapshot().await;
    if let Some(workstream) = snapshot.workstreams.into_iter().find(|item| item.id == id) {
        return Json(serde_json::json!({"workstream": workstream})).into_response();
    }
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "workstream not found"})),
    )
        .into_response()
}

async fn api_worktrees(State(state): State<AppState>) -> Json<serde_json::Value> {
    let snapshot = state.snapshot().await;
    Json(serde_json::json!({
        "worktrees": snapshot.worktrees,
        "generated_at": snapshot.generated_at,
        "warnings": snapshot.warnings,
    }))
}

async fn api_repos(State(state): State<AppState>) -> Json<serde_json::Value> {
    let snapshot = state.snapshot().await;
    Json(serde_json::json!({
        "repos": snapshot.repos,
        "generated_at": snapshot.generated_at,
        "warnings": snapshot.warnings,
    }))
}

fn sanitize_next(input: Option<&str>) -> String {
    let Some(value) = input else {
        return "/".to_string();
    };
    if value.starts_with('/') && !value.starts_with("//") {
        value.to_string()
    } else {
        "/".to_string()
    }
}

fn query_matches(fields: &[&str], needle: Option<&str>) -> bool {
    let Some(needle) = needle.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };
    let needle = needle.to_ascii_lowercase();
    fields
        .iter()
        .any(|field| field.to_ascii_lowercase().contains(&needle))
}

fn filter_sessions<'a>(sessions: &'a [SessionView], query: Option<&str>) -> Vec<&'a SessionView> {
    sessions
        .iter()
        .filter(|item| {
            query_matches(
                &[
                    &item.id,
                    &item.updated_at,
                    item.objective.as_deref().unwrap_or_default(),
                    item.repo_root.as_deref().unwrap_or_default(),
                    item.worktree_path.as_deref().unwrap_or_default(),
                    item.workstream_id.as_deref().unwrap_or_default(),
                ],
                query,
            )
        })
        .collect()
}

fn filter_workstreams<'a>(
    workstreams: &'a [WorkstreamView],
    query: Option<&str>,
) -> Vec<&'a WorkstreamView> {
    workstreams
        .iter()
        .filter(|item| {
            query_matches(
                &[
                    &item.id,
                    item.key.as_deref().unwrap_or_default(),
                    &item.name,
                    &item.status,
                    &item.repo_root,
                    item.worktree_path.as_deref().unwrap_or_default(),
                    item.session_id.as_deref().unwrap_or_default(),
                ],
                query,
            )
        })
        .collect()
}

fn filter_worktrees<'a>(
    worktrees: &'a [WorktreeView],
    query: Option<&str>,
) -> Vec<&'a WorktreeView> {
    worktrees
        .iter()
        .filter(|item| {
            query_matches(
                &[
                    item.id.as_deref().unwrap_or_default(),
                    &item.path,
                    item.repo_root.as_deref().unwrap_or_default(),
                    item.branch.as_deref().unwrap_or_default(),
                    item.attached_session_id.as_deref().unwrap_or_default(),
                    &item.in_git.to_string(),
                    &item.exists.to_string(),
                    &item.issues.join(","),
                ],
                query,
            )
        })
        .collect()
}

fn filter_repos<'a>(repos: &'a [RepoView], query: Option<&str>) -> Vec<&'a RepoView> {
    repos
        .iter()
        .filter(|item| {
            query_matches(
                &[
                    &item.repo_root,
                    &item.workstream_count.to_string(),
                    &item.active_workstream_count.to_string(),
                    &item.worktree_count.to_string(),
                    &item.session_count.to_string(),
                    item.last_activity_at.as_deref().unwrap_or_default(),
                ],
                query,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::http::Request;
    use tower::util::ServiceExt;

    use crate::auth::AuthConfig;
    use crate::state::{AppState, ServiceConfig};

    #[tokio::test]
    async fn healthz_is_public() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let state = AppState::new(
            ServiceConfig {
                workmesh_home: temp.path().to_path_buf(),
                scan_roots: Vec::new(),
                refresh_ms: 5000,
            },
            AuthConfig::from_plain_token(Some("secret".to_string())),
        );

        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }
}
