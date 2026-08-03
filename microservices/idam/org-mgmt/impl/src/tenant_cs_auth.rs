//! Tenant customer-service / ops credential validation (`X-Tenant-CS-Key`).
//!
//! Keys are tenant-bound via `SESAME_TENANT_CS_KEYS` JSON map:
//! `{"hauliage":"secret-for-hauliage","acme":"other-secret"}`.
//!
//! See `docs/design-org-owner-transfer-and-ops-consoles.md`.

use std::collections::HashMap;

use brrtrouter::typed::HttpJson;

pub const TENANT_CS_KEY_ENV: &str = "SESAME_TENANT_CS_KEYS";
pub const TENANT_CS_HEADER: &str = "X-Tenant-CS-Key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantCsAuthError {
    Unconfigured,
    MissingKey,
    MissingTenant,
    Invalid,
}

impl TenantCsAuthError {
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::Unconfigured => 503,
            Self::MissingKey | Self::MissingTenant | Self::Invalid => 401,
        }
    }

    #[must_use]
    pub fn api_error(&self) -> &'static str {
        match self {
            Self::Unconfigured => "tenant_cs_auth_unconfigured",
            Self::MissingKey | Self::MissingTenant | Self::Invalid => "unauthorized",
        }
    }

    #[must_use]
    pub fn message(&self) -> &'static str {
        match self {
            Self::Unconfigured => "Tenant CS credentials are not configured",
            Self::MissingKey => "X-Tenant-CS-Key header is required",
            Self::MissingTenant => "X-Tenant-ID header is required",
            Self::Invalid => "Invalid tenant CS credentials",
        }
    }
}

/// Parse `SESAME_TENANT_CS_KEYS` JSON object into tenant → secret map.
pub fn load_tenant_cs_keys_from_env() -> Result<HashMap<String, String>, TenantCsAuthError> {
    let raw = std::env::var(TENANT_CS_KEY_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .ok_or(TenantCsAuthError::Unconfigured)?;
    parse_tenant_cs_keys_json(&raw)
}

pub fn parse_tenant_cs_keys_json(raw: &str) -> Result<HashMap<String, String>, TenantCsAuthError> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| TenantCsAuthError::Unconfigured)?;
    let obj = value.as_object().ok_or(TenantCsAuthError::Unconfigured)?;
    let mut map = HashMap::new();
    for (tenant, secret) in obj {
        let tenant = tenant.trim();
        let Some(secret) = secret.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        if !tenant.is_empty() {
            map.insert(tenant.to_string(), secret.to_string());
        }
    }
    if map.is_empty() {
        return Err(TenantCsAuthError::Unconfigured);
    }
    Ok(map)
}

/// Validate that `presented_key` is the configured secret for `tenant_id`.
pub fn validate_tenant_cs_key(
    keys: &HashMap<String, String>,
    tenant_id: Option<&str>,
    presented_key: Option<&str>,
) -> Result<(), TenantCsAuthError> {
    let Some(tenant) = tenant_id.map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(TenantCsAuthError::MissingTenant);
    };
    let Some(presented) = presented_key.map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(TenantCsAuthError::MissingKey);
    };
    let Some(expected) = keys.get(tenant) else {
        return Err(TenantCsAuthError::Invalid);
    };
    if !constant_time_eq(presented.as_bytes(), expected.as_bytes()) {
        return Err(TenantCsAuthError::Invalid);
    }
    Ok(())
}

/// Convenience: load env map and validate in one step.
pub fn require_tenant_cs(
    tenant_id: Option<&str>,
    presented_key: Option<&str>,
) -> Result<(), TenantCsAuthError> {
    let keys = load_tenant_cs_keys_from_env()?;
    validate_tenant_cs_key(&keys, tenant_id, presented_key)
}

#[must_use]
pub fn tenant_cs_http_error(err: &TenantCsAuthError) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        err.http_status(),
        serde_json::json!({
            "error": err.api_error(),
            "message": err.message(),
        }),
    )
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_map() {
        let map = parse_tenant_cs_keys_json(r#"{"hauliage":"s1","acme":"s2"}"#).unwrap();
        assert_eq!(map.get("hauliage").map(String::as_str), Some("s1"));
        assert_eq!(map.get("acme").map(String::as_str), Some("s2"));
    }

    #[test]
    fn validates_tenant_bound_key() {
        let map = parse_tenant_cs_keys_json(r#"{"hauliage":"secret"}"#).unwrap();
        assert!(validate_tenant_cs_key(&map, Some("hauliage"), Some("secret")).is_ok());
        assert_eq!(
            validate_tenant_cs_key(&map, Some("hauliage"), Some("wrong")),
            Err(TenantCsAuthError::Invalid)
        );
        assert_eq!(
            validate_tenant_cs_key(&map, Some("acme"), Some("secret")),
            Err(TenantCsAuthError::Invalid)
        );
        assert_eq!(
            validate_tenant_cs_key(&map, None, Some("secret")),
            Err(TenantCsAuthError::MissingTenant)
        );
    }
}
