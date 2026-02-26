use std::{fs, net::IpAddr, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub host: IpAddr,
    pub port: u16,
    pub log_filter: String,
    pub auth_token: Option<String>,
    pub max_body_bytes: usize,
    pub request_timeout_ms: u64,
    pub config_version: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".parse().expect("valid localhost ip"),
            port: 4747,
            log_filter: "info".to_string(),
            auth_token: None,
            max_body_bytes: 1_048_576,
            request_timeout_ms: 15_000,
            config_version: 1,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct FileConfig {
    pub host: Option<IpAddr>,
    pub port: Option<u16>,
    pub log_filter: Option<String>,
    pub auth_token: Option<String>,
    pub max_body_bytes: Option<usize>,
    pub request_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CliOverrides {
    pub host: Option<IpAddr>,
    pub port: Option<u16>,
    pub log_filter: Option<String>,
    pub auth_token: Option<String>,
    pub max_body_bytes: Option<usize>,
    pub request_timeout_ms: Option<u64>,
}

impl CliOverrides {
    pub fn new(
        host: Option<IpAddr>,
        port: Option<u16>,
        log_filter: Option<String>,
        auth_token: Option<String>,
        max_body_bytes: Option<usize>,
        request_timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            host,
            port,
            log_filter,
            auth_token,
            max_body_bytes,
            request_timeout_ms,
        }
    }
}

pub fn load_config(path: Option<&Path>, overrides: &CliOverrides) -> Result<ServiceConfig> {
    let mut cfg = ServiceConfig::default();

    if let Some(path) = path {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        let from_file: FileConfig = toml::from_str(&raw)
            .with_context(|| format!("failed to parse TOML config {}", path.display()))?;
        if let Some(host) = from_file.host {
            cfg.host = host;
        }
        if let Some(port) = from_file.port {
            cfg.port = port;
        }
        if let Some(log_filter) = from_file.log_filter {
            cfg.log_filter = log_filter;
        }
        if let Some(auth_token) = from_file.auth_token {
            cfg.auth_token = Some(auth_token);
        }
        if let Some(max_body_bytes) = from_file.max_body_bytes {
            cfg.max_body_bytes = max_body_bytes;
        }
        if let Some(request_timeout_ms) = from_file.request_timeout_ms {
            cfg.request_timeout_ms = request_timeout_ms;
        }
    }

    if let Some(host) = overrides.host {
        cfg.host = host;
    }
    if let Some(port) = overrides.port {
        cfg.port = port;
    }
    if let Some(log_filter) = &overrides.log_filter {
        cfg.log_filter = log_filter.clone();
    }
    if let Some(auth_token) = &overrides.auth_token {
        cfg.auth_token = Some(auth_token.clone());
    }
    if let Some(max_body_bytes) = overrides.max_body_bytes {
        cfg.max_body_bytes = max_body_bytes;
    }
    if let Some(request_timeout_ms) = overrides.request_timeout_ms {
        cfg.request_timeout_ms = request_timeout_ms;
    }

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_overrides_take_precedence() {
        let overrides = CliOverrides::new(
            Some("0.0.0.0".parse().expect("ip")),
            Some(8080),
            Some("debug".to_string()),
            Some("token123".to_string()),
            Some(64 * 1024),
            Some(30_000),
        );
        let cfg = load_config(None, &overrides).expect("load");
        assert_eq!(cfg.host, "0.0.0.0".parse::<IpAddr>().expect("ip"));
        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.log_filter, "debug");
        assert_eq!(cfg.auth_token.as_deref(), Some("token123"));
        assert_eq!(cfg.max_body_bytes, 64 * 1024);
        assert_eq!(cfg.request_timeout_ms, 30_000);
    }
}
