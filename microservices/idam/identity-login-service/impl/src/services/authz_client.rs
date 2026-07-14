//! Client for authz-core `POST /authz/principals/effective` — the single
//! sanctioned cross-service dependency (login-time JWT claim enrichment).
//!
//! Uses `brrtrouter::http` via [`sesame_common::http`] (see
//! `docs/llmwiki/topics/topic-http-client-policy.md`). Failures degrade
//! gracefully to empty roles: login must not hard-fail when authz-core is
//! briefly unavailable — the token is simply issued without role claims and
//! the client can refresh once authz-core is back.

use std::time::Duration;

use sesame_common::{fetch_post, HttpFetchOptions};

/// Env var for the authz-core base URL.
pub const AUTHZ_CORE_URL_ENV: &str = "AUTHZ_CORE_URL";

/// Default authz-core URL (Kubernetes service DNS, port from repo topology).
const DEFAULT_AUTHZ_CORE_URL: &str = "http://authz-core:8080";

/// Versioned base path all authz-core routes are served under (spec `servers`).
const AUTHZ_BASE_PATH: &str = "/idam/v1";

/// Request timeout — login sits on the hot path, keep enrichment bounded.
const TIMEOUT_MS: u64 = 500;

/// Maximum response body size we are willing to read (64 KB).
const MAX_BODY_BYTES: usize = 64 * 1024;

fn authz_core_url() -> String {
    std::env::var(AUTHZ_CORE_URL_ENV).unwrap_or_else(|_| DEFAULT_AUTHZ_CORE_URL.to_string())
}

/// Full URL of the effective-roles endpoint under `base`.
///
/// authz-core serves every route under the versioned `/idam/v1` base path
/// (spec `servers`); omitting it 404s (regression 2026-07-09: role enrichment
/// silently degraded to empty roles).
fn effective_roles_url(base: &str) -> String {
    format!("{base}{AUTHZ_BASE_PATH}/authz/principals/effective")
}

/// Effective roles + permissions from authz-core.
pub struct EffectiveAuthz {
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// Fetch effective role names for a user from authz-core.
///
/// Returns `Ok(roles)` on success; on any transport/parse failure returns
/// `Err` with a description (callers log and fall back to empty roles).
///
/// # Errors
///
/// Returns an error string when authz-core is unreachable, times out, or
/// returns an unparseable body.
pub fn fetch_effective_roles(
    user_id: &str,
    tenant_id: &str,
    app_id: &str,
) -> Result<Vec<String>, String> {
    fetch_effective_authz(user_id, tenant_id, app_id).map(|authz| authz.roles)
}

/// Fetch effective roles and permissions for JWT enrichment.
///
/// # Errors
///
/// Returns an error string when authz-core is unreachable, times out, or
/// returns an unparseable body.
pub fn fetch_effective_authz(
    user_id: &str,
    tenant_id: &str,
    app_id: &str,
) -> Result<EffectiveAuthz, String> {
    let url = effective_roles_url(&authz_core_url());

    let body = serde_json::json!({
        "user_id": user_id,
        "tenant_id": tenant_id,
        "app_id": app_id,
        "include_inherited": true,
    })
    .to_string();

    let options = HttpFetchOptions {
        timeout: Duration::from_millis(TIMEOUT_MS),
        max_body_bytes: MAX_BODY_BYTES,
        extra_headers: vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("x-tenant-id".to_string(), tenant_id.to_string()),
        ],
    };

    let (_status, response_body) = fetch_post(&url, body.as_bytes(), &options)
        .map_err(|e| format!("authz-core POST {url}: {e}"))?;

    parse_effective_authz(&response_body)
}

fn parse_effective_authz(body: &[u8]) -> Result<EffectiveAuthz, String> {
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("parse EffectiveResponse: {e}"))?;

    Ok(EffectiveAuthz {
        roles: parse_roles_from_value(&value)?,
        permissions: parse_permissions_from_value(&value)?,
    })
}

/// Extract role names from an `EffectiveResponse` body.
///
/// Roles arrive as objects (`{"role": "OWNER", "app_id": ..., ...}`) per the
/// spec; bare strings are tolerated for forward compatibility.
fn parse_roles(body: &[u8]) -> Result<Vec<String>, String> {
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("parse EffectiveResponse: {e}"))?;
    parse_roles_from_value(&value)
}

fn parse_roles_from_value(value: &serde_json::Value) -> Result<Vec<String>, String> {
    let Some(roles) = value.get("roles").and_then(|r| r.as_array()) else {
        return Err(format!(
            "EffectiveResponse missing roles array: {}",
            value
        ));
    };

    Ok(roles
        .iter()
        .filter_map(|r| {
            r.get("role")
                .and_then(|v| v.as_str())
                .or_else(|| r.as_str())
                .map(String::from)
        })
        .collect())
}

fn parse_permissions_from_value(value: &serde_json::Value) -> Result<Vec<String>, String> {
    let Some(permissions) = value.get("permissions").and_then(|p| p.as_array()) else {
        return Ok(vec![]);
    };

    Ok(permissions
        .iter()
        .filter_map(|p| p.as_str().map(String::from))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roles_from_role_objects() {
        let body = br#"{"user_id":"u1","permissions":[],"roles":[
            {"role":"OWNER","app_id":"a1","org_id":null,"inherited":false},
            {"role":"DISPATCHER"}
        ]}"#;
        assert_eq!(parse_roles(body).unwrap(), vec!["OWNER", "DISPATCHER"]);
    }

    #[test]
    fn parse_roles_tolerates_bare_strings() {
        let body = br#"{"roles":["VIEWER"]}"#;
        assert_eq!(parse_roles(body).unwrap(), vec!["VIEWER"]);
    }

    #[test]
    fn parse_roles_rejects_missing_array() {
        assert!(parse_roles(br#"{"user_id":"u1"}"#).is_err());
        assert!(parse_roles(b"not json").is_err());
    }

    #[test]
    fn parse_effective_authz_includes_permissions() {
        let body = br#"{"user_id":"u1","roles":[{"role":"OWNER"}],"permissions":["organization:read","organization:write"]}"#;
        let authz = parse_effective_authz(body).unwrap();
        assert_eq!(authz.roles, vec!["OWNER"]);
        assert_eq!(
            authz.permissions,
            vec!["organization:read", "organization:write"]
        );
    }

    #[test]
    fn effective_roles_url_includes_versioned_base_path() {
        assert_eq!(
            effective_roles_url(DEFAULT_AUTHZ_CORE_URL),
            "http://authz-core:8080/idam/v1/authz/principals/effective"
        );
    }

    #[test]
    fn effective_roles_url_respects_custom_base() {
        assert_eq!(
            effective_roles_url("http://127.0.0.1:8102"),
            "http://127.0.0.1:8102/idam/v1/authz/principals/effective"
        );
    }
}
