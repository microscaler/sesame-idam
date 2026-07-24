//! Password reset journey: forgot → email → reset → sign in with the new
//! password.
//!
//! End-to-end through the real mailbox (Mailpit), like the other email flows.
//! Also asserts the security properties: single-use tokens, generic responses
//! for unknown accounts (no enumeration), weak passwords rejected WITHOUT
//! burning the link, and the old password stopping working.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::{
    auth_forgot_password, auth_login, auth_register, auth_reset_password,
};
use sesame_idam_identity_login_service_gen::handlers::auth_forgot_password::Request as ForgotReq;
use sesame_idam_identity_login_service_gen::handlers::auth_login::Request as LoginReq;
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterReq;
use sesame_idam_identity_login_service_gen::handlers::auth_reset_password::Request as ResetReq;

use crate::common::ensure_active_tenant;

const TENANT: &str = "pwreset-bdd-tenant";
const OLD_PASSWORD: &str = "SecureP@ss123!";
const NEW_PASSWORD: &str = "BrandNewP@ss456!";

static INIT: Once = Once::new();

fn db_available() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let reachable = format!("{host}:{port}")
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some();
    if !reachable {
        return false;
    }
    INIT.call_once(|| {
        std::env::set_var("DB_POOL_MAX", "2");
        std::env::set_var("DB_HOST", &host);
        std::env::set_var("DB_PORT", &port);
        std::env::set_var(
            "DB_USER",
            std::env::var("TEST_DB_USER").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
        std::env::set_var(
            "DB_PASS",
            std::env::var("TEST_DB_PASS").unwrap_or_else(|_| "dev_password_change_in_prod".to_string()),
        );
        std::env::set_var(
            "DB_NAME",
            std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
    });
    true
}

fn tcp_up(host: &str, port: &str) -> bool {
    use std::net::ToSocketAddrs;
    let Ok(mut addrs) = format!("{host}:{port}").to_socket_addrs() else {
        return false;
    };
    addrs.any(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(1500)).is_ok())
}

fn redis_available() -> bool {
    tcp_up(
        &std::env::var("TEST_REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        &std::env::var("TEST_REDIS_PORT").unwrap_or_else(|_| "6379".to_string()),
    )
}

fn mailpit_available() -> Option<String> {
    let smtp_host =
        std::env::var("TEST_SMTP_HOST").unwrap_or_else(|_| "mailpit.data.svc.cluster.local".to_string());
    let smtp_port = std::env::var("TEST_SMTP_PORT").unwrap_or_else(|_| "1025".to_string());
    if !tcp_up(&smtp_host, &smtp_port) {
        return None;
    }
    std::env::set_var("SMTP_HOST", &smtp_host);
    std::env::set_var("SMTP_PORT", &smtp_port);
    if std::env::var("SMTP_TIMEOUT_MS").is_err() {
        std::env::set_var("SMTP_TIMEOUT_MS", "15000");
    }
    let api = std::env::var("TEST_MAILPIT_API")
        .unwrap_or_else(|_| "http://mailpit.data.svc.cluster.local:8025".to_string());
    let options = sesame_common::HttpFetchOptions {
        timeout: Duration::from_millis(1500),
        max_body_bytes: 1024 * 1024,
        extra_headers: vec![],
    };
    match sesame_common::fetch_get(&format!("{api}/api/v1/info"), &options) {
        Ok((200, _)) => Some(api),
        _ => None,
    }
}

fn mailpit_get(api: &str, path: &str) -> Option<serde_json::Value> {
    let options = sesame_common::HttpFetchOptions {
        timeout: Duration::from_millis(2000),
        max_body_bytes: 1024 * 1024,
        extra_headers: vec![],
    };
    match sesame_common::fetch_get(&format!("{api}{path}"), &options) {
        Ok((200, body)) => serde_json::from_slice(&body).ok(),
        _ => None,
    }
}

fn wait_for_message_text(api: &str, recipient: &str) -> Option<String> {
    for _ in 0..60 {
        if let Some(list) = mailpit_get(api, &format!("/api/v1/search?query=to:{recipient}")) {
            if let Some(id) = list["messages"][0]["ID"].as_str() {
                if let Some(msg) = mailpit_get(api, &format!("/api/v1/message/{id}")) {
                    if let Some(text) = msg["Text"].as_str() {
                        return Some(text.to_string());
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    None
}

fn message_count(api: &str, recipient: &str) -> u64 {
    mailpit_get(api, &format!("/api/v1/search?query=to:{recipient}"))
        .and_then(|v| v["messages_count"].as_u64().or_else(|| v["total"].as_u64()))
        .unwrap_or(0)
}

fn extract_reset_token(body: &str) -> Option<String> {
    let url = body
        .split_whitespace()
        .find(|w| w.starts_with("http") && w.contains("token="))?;
    url.split("token=").nth(1).map(|t| t.split('&').next().unwrap_or(t).trim().to_string())
}

fn unique_email(prefix: &str) -> String {
    format!("pwreset_{}_{}@example.com", prefix, uuid::Uuid::new_v4().simple())
}

fn register(email: &str) {
    let resp = auth_register::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/register".to_string(),
        handler_name: "auth_register".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RegisterReq {
            email: email.to_string(),
            first_name: None,
            last_name: None,
            password: OLD_PASSWORD.to_string(),
            phone: None,
            username: None,
            x_tenant_id: TENANT.to_string(),
        },
        jwt_claims: None,
    });
    assert_eq!(resp.status, 201, "register: {:?}", resp.body);
}

fn forgot(email: &str) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    auth_forgot_password::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/password/forgot".to_string(),
        handler_name: "auth_forgot_password".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: ForgotReq { email: email.to_string(), x_tenant_id: TENANT.to_string() },
        jwt_claims: None,
    })
}

fn reset(token: &str, new_password: &str) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    auth_reset_password::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/password/reset".to_string(),
        handler_name: "auth_reset_password".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: ResetReq {
            new_password: new_password.to_string(),
            token: token.to_string(),
            x_tenant_id: TENANT.to_string(),
        },
        jwt_claims: None,
    })
}

fn login(email: &str, password: &str) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    auth_login::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login".to_string(),
        handler_name: "auth_login".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: LoginReq {
            email: email.to_string(),
            organization_id: None,
            password: password.to_string(),
            x_tenant_id: TENANT.to_string(),
        },
        jwt_claims: None,
    })
}

/// Scenario: the whole journey — forgot → email → reset → new password works,
/// old password doesn't, and the link cannot be reused.
#[test]
fn full_password_reset_journey() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    let Some(api) = mailpit_available() else {
        println!("SKIP: Mailpit not available");
        return;
    };
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    ensure_active_tenant(TENANT);

    let email = unique_email("journey");
    register(&email);

    let resp = forgot(&email);
    assert_eq!(resp.status, 200, "forgot: {:?}", resp.body);

    let body = wait_for_message_text(&api, &email).expect("reset email must arrive");
    let token = extract_reset_token(&body).expect("email must carry a token= link");

    let resp = reset(&token, NEW_PASSWORD);
    assert_eq!(resp.status, 200, "reset: {:?}", resp.body);

    // New password works…
    let resp = login(&email, NEW_PASSWORD);
    assert_eq!(resp.status, 200, "new password must work: {:?}", resp.body);
    // …old one does not.
    let resp = login(&email, OLD_PASSWORD);
    assert_eq!(resp.status, 401, "old password must stop working");

    // Link is single-use.
    let replay = reset(&token, "YetAnotherP@ss789!");
    assert_eq!(replay.status, 400, "reset token must be single-use");
}

/// Scenario: a weak new password is rejected WITHOUT consuming the token —
/// the user can retry with the same link.
#[test]
fn weak_password_rejected_without_burning_token() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    let Some(api) = mailpit_available() else {
        println!("SKIP: Mailpit not available");
        return;
    };
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    ensure_active_tenant(TENANT);

    let email = unique_email("weak");
    register(&email);
    assert_eq!(forgot(&email).status, 200);

    let body = wait_for_message_text(&api, &email).expect("reset email");
    let token = extract_reset_token(&body).expect("token");

    let weak = reset(&token, "short");
    assert_eq!(weak.status, 400);
    assert_eq!(weak.body["error"], "weak_password");

    // Same token still works with a strong password.
    let ok = reset(&token, NEW_PASSWORD);
    assert_eq!(ok.status, 200, "token must survive a weak-password retry");
}

/// Scenario: unknown accounts get the identical generic response and receive
/// no mail (no enumeration).
#[test]
fn unknown_account_indistinguishable_and_no_mail() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    let Some(api) = mailpit_available() else {
        println!("SKIP: Mailpit not available");
        return;
    };
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    ensure_active_tenant(TENANT);

    let known = unique_email("known");
    register(&known);
    let ghost = unique_email("ghost");

    let known_resp = forgot(&known);
    let ghost_resp = forgot(&ghost);
    assert_eq!(known_resp.status, ghost_resp.status);
    assert_eq!(known_resp.body, ghost_resp.body, "responses must be identical");

    assert!(wait_for_message_text(&api, &known).is_some(), "known account gets mail");
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(message_count(&api, &ghost), 0, "unknown account gets no mail");
}

/// Scenario: a garbage token is rejected the same way an expired one is.
#[test]
fn invalid_token_rejected() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    ensure_active_tenant(TENANT);
    let resp = reset("not-a-real-token", NEW_PASSWORD);
    assert_eq!(resp.status, 400);
    assert_eq!(resp.body["error"], "invalid_token");
}
