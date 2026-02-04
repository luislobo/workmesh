use std::path::Path;

use workmesh_core::sync::{StubAdapter, SyncEngine, SyncError};

#[test]
fn register_lists_providers() {
    let mut engine = SyncEngine::new();
    engine.register(StubAdapter::new("stub"));
    assert_eq!(engine.providers(), vec!["stub".to_string()]);
}

#[test]
fn with_stub_adapters_registers_defaults() {
    let engine = SyncEngine::with_stub_adapters();
    assert_eq!(
        engine.providers(),
        vec!["github".to_string(), "jira".to_string(), "trello".to_string()]
    );
}

#[test]
fn pull_missing_adapter_returns_error() {
    let engine = SyncEngine::new();
    let result = engine.pull("jira", Path::new("."));
    assert!(matches!(result, Err(SyncError::MissingAdapter(_))));
}
