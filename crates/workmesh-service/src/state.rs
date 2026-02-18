use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, RwLock};

use crate::auth::AuthConfig;
use crate::model::{ServiceSnapshot, WsEvent, WsEventType};
use crate::read_model::collect_snapshot;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub workmesh_home: PathBuf,
    pub scan_roots: Vec<PathBuf>,
    pub refresh_ms: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub config: ServiceConfig,
    pub auth: AuthConfig,
    pub snapshot: Arc<RwLock<ServiceSnapshot>>,
    pub tx: broadcast::Sender<WsEvent>,
}

impl AppState {
    pub fn new(config: ServiceConfig, auth: AuthConfig) -> Self {
        let snapshot = collect_snapshot(&config.workmesh_home, &config.scan_roots);
        let (tx, _) = broadcast::channel(256);
        Self {
            config,
            auth,
            snapshot: Arc::new(RwLock::new(snapshot)),
            tx,
        }
    }

    pub async fn snapshot(&self) -> ServiceSnapshot {
        self.snapshot.read().await.clone()
    }

    pub async fn replace_snapshot(&self, snapshot: ServiceSnapshot) {
        let mut guard = self.snapshot.write().await;
        *guard = snapshot;
    }
}

pub fn spawn_refresh_loop(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(state.config.refresh_ms));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let old_snapshot = state.snapshot().await;
            let new_snapshot =
                collect_snapshot(&state.config.workmesh_home, &state.config.scan_roots);
            let diff = diff_event(&old_snapshot, &new_snapshot);

            state.replace_snapshot(new_snapshot.clone()).await;

            let message = if diff.changed_session_ids.is_empty()
                && diff.changed_workstream_ids.is_empty()
                && diff.changed_worktree_paths.is_empty()
            {
                WsEvent {
                    event_type: WsEventType::Heartbeat,
                    generated_at: new_snapshot.generated_at,
                    summary: Some(new_snapshot.summary),
                    changed_workstream_ids: Vec::new(),
                    changed_session_ids: Vec::new(),
                    changed_worktree_paths: Vec::new(),
                }
            } else {
                diff
            };

            let _ = state.tx.send(message);
        }
    });
}

pub fn initial_snapshot_event(snapshot: &ServiceSnapshot) -> WsEvent {
    WsEvent {
        event_type: WsEventType::Snapshot,
        generated_at: snapshot.generated_at.clone(),
        summary: Some(snapshot.summary.clone()),
        changed_workstream_ids: snapshot
            .workstreams
            .iter()
            .map(|item| item.id.clone())
            .collect(),
        changed_session_ids: snapshot
            .sessions
            .iter()
            .map(|item| item.id.clone())
            .collect(),
        changed_worktree_paths: snapshot
            .worktrees
            .iter()
            .map(|item| item.path.clone())
            .collect(),
    }
}

fn diff_event(old: &ServiceSnapshot, new: &ServiceSnapshot) -> WsEvent {
    let old_sessions = index_by_id(
        old.sessions
            .iter()
            .map(|item| (&item.id, item.updated_at.as_str())),
    );
    let new_sessions = index_by_id(
        new.sessions
            .iter()
            .map(|item| (&item.id, item.updated_at.as_str())),
    );
    let changed_sessions = changed_ids(&old_sessions, &new_sessions);

    let old_streams = index_by_id(
        old.workstreams
            .iter()
            .map(|item| (&item.id, item.updated_at.as_str())),
    );
    let new_streams = index_by_id(
        new.workstreams
            .iter()
            .map(|item| (&item.id, item.updated_at.as_str())),
    );
    let changed_streams = changed_ids(&old_streams, &new_streams);

    let old_worktrees = index_by_id(
        old.worktrees
            .iter()
            .map(|item| (&item.path, item.branch.as_deref().unwrap_or(""))),
    );
    let new_worktrees = index_by_id(
        new.worktrees
            .iter()
            .map(|item| (&item.path, item.branch.as_deref().unwrap_or(""))),
    );
    let changed_worktrees = changed_ids(&old_worktrees, &new_worktrees);

    WsEvent {
        event_type: WsEventType::Delta,
        generated_at: new.generated_at.clone(),
        summary: Some(new.summary.clone()),
        changed_workstream_ids: changed_streams,
        changed_session_ids: changed_sessions,
        changed_worktree_paths: changed_worktrees,
    }
}

fn index_by_id<'a>(iter: impl Iterator<Item = (&'a String, &'a str)>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (id, version) in iter {
        out.insert(id.to_string(), version.to_string());
    }
    out
}

fn changed_ids(old: &BTreeMap<String, String>, new: &BTreeMap<String, String>) -> Vec<String> {
    let mut changed = BTreeSet::new();
    for (id, version) in new {
        if old.get(id) != Some(version) {
            changed.insert(id.to_string());
        }
    }
    for id in old.keys() {
        if !new.contains_key(id) {
            changed.insert(id.to_string());
        }
    }
    changed.into_iter().collect()
}
