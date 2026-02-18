use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SummaryResponse {
    pub generated_at: String,
    pub active_sessions: usize,
    pub active_workstreams: usize,
    pub open_worktrees: usize,
    pub repos_tracked: usize,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionView {
    pub id: String,
    pub updated_at: String,
    #[serde(default)]
    pub objective: Option<String>,
    pub cwd: String,
    #[serde(default)]
    pub repo_root: Option<String>,
    #[serde(default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub workstream_id: Option<String>,
    #[serde(default)]
    pub working_set: Vec<String>,
    #[serde(default)]
    pub truth_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkstreamContextView {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub scope_mode: Option<String>,
    #[serde(default)]
    pub epic_id: Option<String>,
    #[serde(default)]
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkstreamView {
    pub id: String,
    #[serde(default)]
    pub key: Option<String>,
    pub name: String,
    pub status: String,
    pub repo_root: String,
    #[serde(default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub context: Option<WorkstreamContextView>,
    #[serde(default)]
    pub truth_refs: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorktreeView {
    #[serde(default)]
    pub id: Option<String>,
    pub path: String,
    #[serde(default)]
    pub repo_root: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub attached_session_id: Option<String>,
    pub in_git: bool,
    pub exists: bool,
    #[serde(default)]
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoView {
    pub repo_root: String,
    pub workstream_count: usize,
    pub active_workstream_count: usize,
    pub worktree_count: usize,
    pub session_count: usize,
    #[serde(default)]
    pub last_activity_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceSnapshot {
    pub generated_at: String,
    pub summary: SummaryResponse,
    #[serde(default)]
    pub current_session_id: Option<String>,
    #[serde(default)]
    pub sessions: Vec<SessionView>,
    #[serde(default)]
    pub workstreams: Vec<WorkstreamView>,
    #[serde(default)]
    pub worktrees: Vec<WorktreeView>,
    #[serde(default)]
    pub repos: Vec<RepoView>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WsEventType {
    Snapshot,
    Delta,
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEvent {
    #[serde(rename = "type")]
    pub event_type: WsEventType,
    pub generated_at: String,
    #[serde(default)]
    pub summary: Option<SummaryResponse>,
    #[serde(default)]
    pub changed_workstream_ids: Vec<String>,
    #[serde(default)]
    pub changed_session_ids: Vec<String>,
    #[serde(default)]
    pub changed_worktree_paths: Vec<String>,
}
