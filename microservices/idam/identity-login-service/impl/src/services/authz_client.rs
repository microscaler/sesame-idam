//! Client for authz-core `POST /authz/principals/effective` — the single
//! sanctioned cross-service dependency (login-time JWT claim enrichment).
//!
//! Uses `may_http` per the single-HTTP-client policy (see
//! `docs/llmwiki/topics/topic-http-client-policy.md`). Failures degrade
//! gracefully to empty roles: login must not hard-fail when authz-core is
//! briefly unavailable — the token is simply issued without role claims and
//! the client can refresh once authz-core is back.

use std::io::Read;
use std::time::Duration;

use http_legacy::{Method, Uri};
use may_http::client::HttpClient;

/// Env var for the authz-core base URL.
pub const AUTHZ_CORE_URL_ENV: &str = "AUTHZ_CORE_URL";

/// Default authz-core URL (Kubernetes service DNS, port from repo topology).
const DEFAULT_AUTHZ_CORE_URL: &str = "http://authz-core:8102";

/// Request timeout — login sits on the hot path, keep enrichment bounded.
const TIMEOUT_MS: u64 = 500;

/// Maximum response body size we are willing to read (64 KB).
const MAX_BODY_BYTES: u64 = 64 * 1024;

fn authz_core_url() -> String {
    std::env::var(AUTHZ_CORE_URL_ENV).unwrap_or_else(|_| DEFAULT_AUTHZ_CORE_URL.to_string())
}

/// Parse `http://host[:port]` into (host, port). No TLS support — authz-core
/// is cluster-internal.
fn parse_host_port(url: &str) -> Option<(String, u16)> {
    let rest = url.strip_prefix("http://")?;
    let host_port = rest.split('/').next()?;
    match host_port.split_once(':') {
        Some((h, p)) => Some((h.to_string(), p.parse().ok()?)),
        None => Some((host_port.to_string(), 80)),
    }
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
    let base = authz_core_url();
    let (host, port) =
        parse_host_port(&base).ok_or_else(|| format!("invalid {AUTHZ_CORE_URL_ENV}: {base}"))?;

    let mut client = HttpClient::connect((host.as_str(), port))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    client.set_timeout(Some(Duration::from_millis(TIMEOUT_MS)));

    let uri: Uri = format!("{base}/authz/principals/effective")
        .parse()
        .map_err(|e| format!("uri: {e}"))?;

    let body = serde_json::json!({
        "user_id": user_id,
        "tenant_id": tenant_id,
        "app_id": app_id,
        "include_inherited": true,
    })
    .to_string();

    let mut req = client.new_request(Method::POST, uri);
    req.headers_mut().insert(
        "content-type",
        http_legacy::HeaderValue::from_static("application/json"),
    );
    if let Ok(tenant_header) = http_legacy::HeaderValue::from_str(tenant_id) {
        req.headers_mut().insert("x-tenant-id", tenant_header);
    }
    req.send(body.as_bytes())
        .map_err(|e| format!("send: {e}"))?;

    let mut response = client
        .send_request(req)
        .map_err(|e| format!("response: {e}"))?;

    let mut buf = Vec::with_capacity(1024);
    response
        .by_ref()
        .take(MAX_BODY_BYTES)
        .read_to_end(&mut buf)
        .map_err(|e| format!("read body: {e}"))?;

    parse_roles(&buf)
}

/// Extract role names from an `EffectiveResponse` body.
///
/// Roles arrive as objects (`{"role": "OWNER", "app_id": ..., ...}`) per the
/// spec; bare strings are tolerated for forward compatibility.
fn parse_roles(body: &[u8]) -> Result<Vec<String>, String> {
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("parse EffectiveResponse: {e}"))?;

    let Some(roles) = value.get("roles").and_then(|r| r.as_array()) else {
        return Err(format!(
            "EffectiveResponse missing roles array: {}",
            String::from_utf8_lossy(&body[..body.len().min(200)])
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
    fn parse_host_port_variants() {
        assert_eq!(
            parse_host_port("http://authz-core:8102"),
            Some(("authz-core".to_string(), 8102))
        );
        assert_eq!(
            parse_host_port("http://127.0.0.1"),
            Some(("127.0.0.1".to_string(), 80))
        );
        assert_eq!(parse_host_port("https://x"), None);
    }
}
