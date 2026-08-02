//! Pre-OIDC live North BDD against the public API edge.
//!
//! Hits `https://api.sesameidentity.dev.local/idam/v1/auth/*` (Gateway → login).
//! Skips when the API host is unreachable.

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::common::{HAULIAGE_TENANT, HAULIAGE_WEB_CLIENT};

const API_BASE: &str = "https://api.sesameidentity.dev.local/idam/v1";
const DEMO_EMAIL: &str = "owner@hauliage.dev";
const DEMO_PASSWORD: &str = "SecureP@ss123!";

fn live_available() -> bool {
    let host = std::env::var("SESAME_NORTH_LIVE_HOST")
        .unwrap_or_else(|_| "api.sesameidentity.dev.local".to_string());
    let Ok(mut addrs) = (host.as_str(), 443u16).to_socket_addrs() else {
        return false;
    };
    let Some(addr) = addrs.next() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(800)).is_ok()
        || TcpStream::connect_timeout(
            &SocketAddr::from(([10, 177, 76, 220], 443)),
            Duration::from_millis(400),
        )
        .is_ok()
}

fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .expect("http client")
}

#[test]
fn live_north_password_login_issues_tokens() {
    if !live_available() {
        eprintln!("SKIP live_north_password_login: api host unreachable");
        return;
    }
    let client = http_client();
    let resp = client
        .post(format!("{API_BASE}/auth/login"))
        .header("Content-Type", "application/json")
        .header("X-Tenant-ID", HAULIAGE_TENANT)
        .json(&serde_json::json!({
            "email": DEMO_EMAIL,
            "password": DEMO_PASSWORD,
            "client_id": HAULIAGE_WEB_CLIENT,
        }))
        .send()
        .expect("login request");
    let status = resp.status();
    let body: serde_json::Value = resp.json().unwrap_or_else(|_| serde_json::json!({}));
    assert_eq!(status, 200, "login failed: {body}");
    assert!(body["access_token"].as_str().unwrap_or("").len() > 20);
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["expires_in"].as_i64().unwrap_or(0) > 0);
}

#[test]
fn live_north_wrong_password_is_invalid_credentials() {
    if !live_available() {
        eprintln!("SKIP live_north_wrong_password: api host unreachable");
        return;
    }
    let client = http_client();
    let resp = client
        .post(format!("{API_BASE}/auth/login"))
        .header("Content-Type", "application/json")
        .header("X-Tenant-ID", HAULIAGE_TENANT)
        .json(&serde_json::json!({
            "email": DEMO_EMAIL,
            "password": "definitely-not-the-password",
            "client_id": HAULIAGE_WEB_CLIENT,
        }))
        .send()
        .expect("login request");
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = resp.json().expect("error json");
    assert_eq!(body["error"], "invalid_credentials");
}

#[test]
fn live_north_unknown_client_is_invalid_client() {
    if !live_available() {
        eprintln!("SKIP live_north_unknown_client: api host unreachable");
        return;
    }
    let client = http_client();
    let resp = client
        .post(format!("{API_BASE}/auth/login"))
        .header("Content-Type", "application/json")
        .header("X-Tenant-ID", HAULIAGE_TENANT)
        .json(&serde_json::json!({
            "email": DEMO_EMAIL,
            "password": DEMO_PASSWORD,
            "client_id": "not-a-registered-client",
        }))
        .send()
        .expect("login request");
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = resp.json().expect("error json");
    assert_eq!(body["error"], "invalid_client");
}

#[test]
fn live_north_logout_requires_bearer() {
    if !live_available() {
        eprintln!("SKIP live_north_logout: api host unreachable");
        return;
    }
    let client = http_client();
    let resp = client
        .post(format!("{API_BASE}/auth/logout"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({}))
        .send()
        .expect("logout request");
    assert!(
        resp.status().as_u16() == 401 || resp.status().as_u16() == 403,
        "logout without bearer must be unauthorized, got {}",
        resp.status()
    );
}

#[test]
fn live_north_platform_is_not_routed_on_api_edge() {
    if !live_available() {
        eprintln!("SKIP live_north_platform_airgap: api host unreachable");
        return;
    }
    let client = http_client();
    let resp = client
        .get(format!("{API_BASE}/platform/tenants/{HAULIAGE_TENANT}"))
        .header("X-Platform-Admin-Key", "dev-platform-admin")
        .send()
        .expect("platform probe");
    // Edge must not expose /platform/* (404/405/no-route). Never 200 with tenant body.
    assert_ne!(
        resp.status().as_u16(),
        200,
        "platform surface must not be north-routed: {}",
        resp.text().unwrap_or_default()
    );
}
