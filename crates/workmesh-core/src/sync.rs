use std::collections::HashMap;
use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("No adapter registered for provider: {0}")]
    MissingAdapter(String),
    #[error("Adapter error: {0}")]
    Adapter(String),
}

#[derive(Debug, Clone)]
pub enum SyncDirection {
    Pull,
    Push,
}

#[derive(Debug, Clone)]
pub struct SyncReport {
    pub provider: String,
    pub direction: SyncDirection,
    pub pulled: usize,
    pub pushed: usize,
    pub conflicts: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SyncStatus {
    pub provider: String,
    pub connected: bool,
    pub last_sync: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SyncConflict {
    pub conflict_id: String,
    pub provider: String,
    pub task_id: Option<String>,
    pub external_id: Option<String>,
    pub reason: String,
}

pub trait SyncAdapter {
    fn provider(&self) -> &str;
    fn pull(&self, backlog_dir: &Path) -> Result<SyncReport, SyncError>;
    fn push(&self, backlog_dir: &Path) -> Result<SyncReport, SyncError>;
    fn status(&self, backlog_dir: &Path) -> Result<SyncStatus, SyncError>;
    fn list_conflicts(&self, backlog_dir: &Path) -> Result<Vec<SyncConflict>, SyncError>;
    fn resolve_conflict(
        &self,
        backlog_dir: &Path,
        conflict_id: &str,
    ) -> Result<SyncReport, SyncError>;
}

pub struct SyncEngine {
    adapters: HashMap<String, Box<dyn SyncAdapter>>,
}

impl SyncEngine {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn with_stub_adapters() -> Self {
        let mut engine = Self::new();
        engine.register(StubAdapter::new("github"));
        engine.register(StubAdapter::new("jira"));
        engine.register(StubAdapter::new("trello"));
        engine
    }

    pub fn register<A: SyncAdapter + 'static>(&mut self, adapter: A) {
        let key = adapter.provider().to_string();
        self.adapters.insert(key, Box::new(adapter));
    }

    pub fn providers(&self) -> Vec<String> {
        let mut providers: Vec<String> = self.adapters.keys().cloned().collect();
        providers.sort();
        providers
    }

    pub fn pull(&self, provider: &str, backlog_dir: &Path) -> Result<SyncReport, SyncError> {
        self.adapter(provider)?.pull(backlog_dir)
    }

    pub fn push(&self, provider: &str, backlog_dir: &Path) -> Result<SyncReport, SyncError> {
        self.adapter(provider)?.push(backlog_dir)
    }

    pub fn status(&self, provider: &str, backlog_dir: &Path) -> Result<SyncStatus, SyncError> {
        self.adapter(provider)?.status(backlog_dir)
    }

    pub fn list_conflicts(
        &self,
        provider: &str,
        backlog_dir: &Path,
    ) -> Result<Vec<SyncConflict>, SyncError> {
        self.adapter(provider)?.list_conflicts(backlog_dir)
    }

    pub fn resolve_conflict(
        &self,
        provider: &str,
        backlog_dir: &Path,
        conflict_id: &str,
    ) -> Result<SyncReport, SyncError> {
        self.adapter(provider)?
            .resolve_conflict(backlog_dir, conflict_id)
    }

    fn adapter(&self, provider: &str) -> Result<&dyn SyncAdapter, SyncError> {
        self.adapters
            .get(provider)
            .map(|adapter| adapter.as_ref())
            .ok_or_else(|| SyncError::MissingAdapter(provider.to_string()))
    }
}

#[derive(Debug, Default)]
pub struct StubAdapter {
    provider: String,
}

impl StubAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
        }
    }
}

impl SyncAdapter for StubAdapter {
    fn provider(&self) -> &str {
        &self.provider
    }

    fn pull(&self, _backlog_dir: &Path) -> Result<SyncReport, SyncError> {
        Ok(SyncReport {
            provider: self.provider.clone(),
            direction: SyncDirection::Pull,
            pulled: 0,
            pushed: 0,
            conflicts: 0,
            notes: vec!["stub adapter: no external sync configured".to_string()],
        })
    }

    fn push(&self, _backlog_dir: &Path) -> Result<SyncReport, SyncError> {
        Ok(SyncReport {
            provider: self.provider.clone(),
            direction: SyncDirection::Push,
            pulled: 0,
            pushed: 0,
            conflicts: 0,
            notes: vec!["stub adapter: no external sync configured".to_string()],
        })
    }

    fn status(&self, _backlog_dir: &Path) -> Result<SyncStatus, SyncError> {
        Ok(SyncStatus {
            provider: self.provider.clone(),
            connected: false,
            last_sync: None,
            notes: vec!["stub adapter: no external sync configured".to_string()],
        })
    }

    fn list_conflicts(&self, _backlog_dir: &Path) -> Result<Vec<SyncConflict>, SyncError> {
        Ok(Vec::new())
    }

    fn resolve_conflict(
        &self,
        _backlog_dir: &Path,
        conflict_id: &str,
    ) -> Result<SyncReport, SyncError> {
        Ok(SyncReport {
            provider: self.provider.clone(),
            direction: SyncDirection::Pull,
            pulled: 0,
            pushed: 0,
            conflicts: 0,
            notes: vec![format!(
                "stub adapter: conflict {} not resolved (no-op)",
                conflict_id
            )],
        })
    }
}
