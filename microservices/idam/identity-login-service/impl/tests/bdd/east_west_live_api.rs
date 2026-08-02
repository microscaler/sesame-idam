//! Pre-OIDC live east–west BDD against ClusterIP login (via port-forward).
//!
//! Set `SESAME_EW_LOGIN_BASE` (default `http://127.0.0.1:18101`) to a
//! port-forward of `svc/identity-login-service:8080`. Skips when unreachable.
//!
//! ```bash
//! kubectl -n sesame-idam port-forward svc/identity-login-service 18101:8080 &
//! SESAME_EW_LOGIN_BASE=http://127.0.0.1:18101 cargo test -p sesame_idam_identity_login_service \
//!   --test main_bdd east_west_ -- --nocapture
//! ```

use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use crate::common::{HAULIAGE_TENANT, HAULIAGE_WEB_CLIENT};

const DEMO_EMAIL: &str = "owner@hauliage.dev";
const DEMO_PASSWORD: &str = "SecureP@ss123!";
const PLATFORM_KEY: &str = "dev-platform-admin";

fn ew_base() -> String {
    std::env::var("SESAME_EW_LOGIN_BASE")
        .unwrap_or_else(|_| "http://127.0.0.1:18101".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn ew_available() -> bool {
    let base = ew_base();
    let Some(hostport) = base
        .strip_prefix("http://")
        .or_else(|| base.strip_prefix("https://"))
    else {
        return false;
    };
    let hostport = hostport.split('/').next().unwrap_or(hostport);
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(8080)),
        None => (hostport, 80u16),
    };
    let Ok(addr) = format!("{host}:{port}").parse::<SocketAddr>() else {
        // hostname — try localhost parse only
        if host == "127.0.0.1" || host == "localhost" {
            return TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], port)),
                Duration::from_millis(400),
            )
            .is_ok();
        }
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(400)).is_ok()
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
fn live_ew_password_login_issues_tokens() {
    if !ew_available() {
        eprintln!(
            "SKIP live_ew_password_login: {} unreachable (port-forward login svc)",
            ew_base()
        );
        return;
    }
    let client = http_client();
    let resp = client
        .post(format!("{}/idam/v1/auth/login", ew_base()))
        .header("Content-Type", "application/json")
        .header("X-Tenant-ID", HAULIAGE_TENANT)
        .json(&serde_json::json!({
            "email": DEMO_EMAIL,
            "password": DEMO_PASSWORD,
            "client_id": HAULIAGE_WEB_CLIENT,
        }))
        .send()
        .expect("ew login");
    let status = resp.status();
    let body: serde_json::Value = resp.json().unwrap_or_else(|_| serde_json::json!({}));
    assert_eq!(status, 200, "ew login failed: {body}");
    assert!(body["access_token"].as_str().unwrap_or("").len() > 20);
}

#[test]
fn live_ew_platform_get_requires_platform_key() {
    if !ew_available() {
        eprintln!("SKIP live_ew_platform_auth: {} unreachable", ew_base());
        return;
    }
    let client = http_client();
    let url = format!("{}/idam/v1/platform/tenants/{HAULIAGE_TENANT}", ew_base());

    let missing = client.get(&url).send().expect("platform no key");
    assert!(
        missing.status().as_u16() == 401 || missing.status().as_u16() == 403,
        "missing platform key must be unauthorized, got {}",
        missing.status()
    );

    let wrong = client
        .get(&url)
        .header("X-Platform-Admin-Key", "wrong-key")
        .send()
        .expect("platform wrong key");
    assert!(
        wrong.status().as_u16() == 401 || wrong.status().as_u16() == 403,
        "wrong platform key must be unauthorized, got {}",
        wrong.status()
    );

    // Valid key reaches the handler (auth layer passed). Response-schema drift
    // may still yield 500 — assert we are past 401/403.
    let ok = client
        .get(&url)
        .header("X-Platform-Admin-Key", PLATFORM_KEY)
        .send()
        .expect("platform valid key");
    assert!(
        ok.status().as_u16() != 401 && ok.status().as_u16() != 403,
        "valid platform key must not be unauthorized, got {}",
        ok.status()
    );
}

#[test]
fn live_ew_token_unsupported_grant_fails_closed() {
    if !ew_available() {
        eprintln!("SKIP live_ew_token_grant: {} unreachable", ew_base());
        return;
    }
    let client = http_client();
    let resp = client
        .post(format!("{}/idam/v1/auth/token", ew_base()))
        .header("Content-Type", "application/json")
        .header("X-Tenant-ID", HAULIAGE_TENANT)
        .json(&serde_json::json!({
            "grant_type": "password",
            "client_id": HAULIAGE_WEB_CLIENT,
        }))
        .send()
        .expect("token request");
    assert!(
        resp.status().as_u16() == 400 || resp.status().as_u16() == 401,
        "unsupported grant must fail closed, got {}",
        resp.status()
    );
}

#[test]
fn live_ew_client_credentials_without_secret_fails_closed() {
    if !ew_available() {
        eprintln!("SKIP live_ew_cc: {} unreachable", ew_base());
        return;
    }
    let client = http_client();
    let resp = client
        .post(format!("{}/idam/v1/auth/token", ew_base()))
        .header("Content-Type", "application/json")
        .header("X-Tenant-ID", HAULIAGE_TENANT)
        .json(&serde_json::json!({
            "grant_type": "client_credentials",
            "client_id": "missing-m2m-client",
        }))
        .send()
        .expect("cc request");
    let status = resp.status().as_u16();
    let body: serde_json::Value = resp.json().unwrap_or_else(|_| serde_json::json!({}));
    // Fail-closed: either HTTP error or an error object without an access token.
    let has_access = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    assert!(
        (status >= 400 && status < 500) || !has_access,
        "CC without secret must not mint a token: status={status} body={body}"
    );
}
