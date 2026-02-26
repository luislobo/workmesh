use std::collections::BTreeMap;

use axum::http::StatusCode;
use serde::Serialize;
use serde_json::Value;

pub type DispatchFn = fn(&str, &Value) -> Result<Value, ToolError>;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderTool {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub namespace: &'static str,
    pub version: &'static str,
    pub tools: Vec<ProviderTool>,
}

#[derive(Debug)]
pub struct ProviderEntry {
    info: ProviderInfo,
    dispatch: DispatchFn,
}

impl ProviderEntry {
    pub fn new(info: ProviderInfo, dispatch: DispatchFn) -> Self {
        Self { info, dispatch }
    }
}

#[derive(Debug)]
pub struct ToolError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub details: Value,
}

impl ToolError {
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_ARGUMENT",
            message: message.into(),
            details: Value::Object(Default::default()),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "NOT_FOUND",
            message: message.into(),
            details: Value::Object(Default::default()),
        }
    }
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL",
            message: message.into(),
            details: Value::Object(Default::default()),
        }
    }
}

#[derive(Debug, Default)]
pub struct ToolHost {
    providers: BTreeMap<&'static str, ProviderEntry>,
}

impl ToolHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, entry: ProviderEntry) {
        self.providers.insert(entry.info.namespace, entry);
    }

    pub fn providers(&self) -> Vec<ProviderInfo> {
        self.providers
            .values()
            .map(|entry| entry.info.clone())
            .collect()
    }

    pub fn invoke(
        &self,
        namespace: &str,
        tool: &str,
        arguments: &Value,
    ) -> Result<Value, ToolError> {
        let Some(provider) = self.providers.get(namespace) else {
            return Err(ToolError::not_found(format!(
                "Provider namespace not found: {}",
                namespace
            )));
        };
        (provider.dispatch)(tool, arguments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dispatch(tool: &str, _arguments: &Value) -> Result<Value, ToolError> {
        match tool {
            "ok" => Ok(json!({"ok": true})),
            _ => Err(ToolError::not_found("tool missing")),
        }
    }

    #[test]
    fn invoke_dispatches_to_registered_provider() {
        let mut host = ToolHost::new();
        host.register(ProviderEntry::new(
            ProviderInfo {
                namespace: "example",
                version: "1",
                tools: vec![ProviderTool {
                    name: "ok",
                    description: "ok",
                }],
            },
            dispatch,
        ));
        let result = host
            .invoke("example", "ok", &json!({}))
            .expect("dispatch works");
        assert_eq!(result["ok"], true);
    }

    #[test]
    fn invoke_returns_not_found_for_unknown_provider() {
        let host = ToolHost::new();
        let error = host
            .invoke("missing", "ok", &json!({}))
            .expect_err("expected error");
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.code, "NOT_FOUND");
    }
}
