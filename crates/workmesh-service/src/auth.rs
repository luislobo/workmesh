use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const AUTH_COOKIE_NAME: &str = "wm_auth";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    pub token_hash: Option<String>,
}

impl AuthConfig {
    pub fn from_plain_token(token: Option<String>) -> Self {
        let token_hash = token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(hash_token);
        Self { token_hash }
    }

    pub fn is_required(&self) -> bool {
        self.token_hash.is_some()
    }

    pub fn verify_plain_token(&self, token: &str) -> bool {
        let Some(expected) = self.token_hash.as_deref() else {
            return true;
        };
        hash_token(token.trim()) == expected
    }

    pub fn verify_cookie_value(&self, cookie_value: &str) -> bool {
        let Some(expected) = self.token_hash.as_deref() else {
            return true;
        };
        cookie_value.trim() == expected
    }

    pub fn cookie_value(&self) -> Option<String> {
        self.token_hash.clone()
    }
}

pub fn is_authorized(headers: &HeaderMap, config: &AuthConfig) -> bool {
    if !config.is_required() {
        return true;
    }

    if let Some(token) = bearer_token(headers) {
        if config.verify_plain_token(&token) {
            return true;
        }
    }

    if let Some(cookie) = cookie_value(headers, AUTH_COOKIE_NAME) {
        if config.verify_cookie_value(&cookie) {
            return true;
        }
    }

    false
}

pub fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get("authorization")?.to_str().ok()?.trim();
    let lower = raw.to_ascii_lowercase();
    if !lower.starts_with("bearer ") {
        return None;
    }
    let token = raw.get(7..)?.trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

pub fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get("cookie")?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        let mut pieces = trimmed.splitn(2, '=');
        let key = pieces.next()?.trim();
        let value = pieces.next()?.trim();
        if key == name {
            return Some(value.to_string());
        }
    }
    None
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_config_hashes_and_verifies_tokens() {
        let cfg = AuthConfig::from_plain_token(Some("abc123".to_string()));
        assert!(cfg.is_required());
        assert!(cfg.verify_plain_token("abc123"));
        assert!(!cfg.verify_plain_token("zzz"));
    }
}
