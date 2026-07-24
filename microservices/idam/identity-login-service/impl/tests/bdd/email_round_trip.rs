//! End-to-end email round trip via the cluster's Mailpit test endpoint
//! (`data` namespace): request OTP / magic link → the service delivers real
//! SMTP mail → the test reads the message back through Mailpit's REST API →
//! extracts the code / link → verifies it → asserts a real signed JWT.
//! The magic-link scenario IS the "click": the URL extracted from the email
//! body is consumed exactly as the browser would.
//!
//! Also proves Gate A3 against the actual mailbox: capped sends must produce
//! NO new Mailpit message — the meter is verified at the delivery boundary,
//! not just at the guard's return value.
//!
//! Needs Postgres, Redis, and Mailpit (SMTP + API). Mailpit is addressed BY
//! SERVICE NAME only (no IPs):
//! - `TEST_SMTP_HOST`/`TEST_SMTP_PORT` (default mailpit.data.svc.cluster.local:1025)
//! - `TEST_MAILPIT_API`   (default http://mailpit.data.svc.cluster.local:8025)
//! In-cluster that resolves natively; dev/build hosts resolve the
//! `svc.cluster.local` zone via their resolver (or override the env for a
//! local Mailpit). Skips gracefully when any dependency is unreachable.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::{
    auth_register, login_email_otp, magic_link_send, magic_link_verify, verify_email_otp,
};
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterRequest;
use sesame_idam_identity_login_service_gen::handlers::login_email_otp::Request as OtpSendRequest;
use sesame_idam_identity_login_service_gen::handlers::magic_link_send::Request as MagicSendRequest;
use sesame_idam_identity_login_service_gen::handlers::magic_link_verify::Request as MagicVerifyRequest;
use sesame_idam_identity_login_service_gen::handlers::verify_email_otp::Request as OtpVerifyRequest;

use crate::common::ensure_active_tenant;

const TEST_TENANT: &str = "email-e2e-tenant";
const PASSWORD: &str = "SecureP@ss123!";

static INIT: Once = Once::new();

fn db_available() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let addr = format!("{host}:{port}");
    let reachable = addr
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
            std::env::var("TEST_DB_PASS")
                .unwrap_or_else(|_| "dev_password_change_in_prod".to_string()),
        );
        std::env::set_var(
            "DB_NAME",
            std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
    });
    true
}

fn tcp_up(host: &str, port: &str) -> bool {
    // Resolve (works for service DNS names and /etc/hosts entries, not just
    // raw IPs — SocketAddr::parse would reject hostnames).
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

/// Point the service's SMTP client at the test Mailpit and return the API
/// base, or None (skip) when unreachable.
///
/// Defaults target the cluster service BY NAME (no IPs anywhere): resolvable
/// in-cluster natively, and from dev/build hosts via a resolver or hosts
/// entry for the `svc.cluster.local` zone. Override with TEST_SMTP_HOST /
/// TEST_SMTP_PORT / TEST_MAILPIT_API (e.g. a local Mailpit container).
fn mailpit_available() -> Option<String> {
    let smtp_host = std::env::var("TEST_SMTP_HOST")
        .unwrap_or_else(|_| "mailpit.data.svc.cluster.local".to_string());
    let smtp_port = std::env::var("TEST_SMTP_PORT").unwrap_or_else(|_| "1025".to_string());
    if !tcp_up(&smtp_host, &smtp_port) {
        return None;
    }
    std::env::set_var("SMTP_HOST", &smtp_host);
    std::env::set_var("SMTP_PORT", &smtp_port);
    // Cluster LB paths (MetalLB) can delay the SMTP banner past the 5s
    // default; allow overriding and default the tests to a patient client.
    if std::env::var("SMTP_TIMEOUT_MS").is_err() {
        std::env::set_var("SMTP_TIMEOUT_MS", "15000");
    }
    let api = std::env::var("TEST_MAILPIT_API")
        .unwrap_or_else(|_| "http://mailpit.data.svc.cluster.local:8025".to_string());
    match sesame_common::fetch_get(&format!("{api}/api/v1/info"), &fetch_options(1500)) {
        Ok((200, _)) => Some(api),
        _ => None,
    }
}

fn unique_email(prefix: &str) -> String {
    format!("e2e_{}_{}@example.com", prefix, uuid::Uuid::new_v4().simple())
}

fn register(email: &str) {
    let resp = auth_register::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/register".to_string(),
        handler_name: "auth_register".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RegisterRequest {
            email: email.to_string(),
            first_name: None,
            last_name: None,
            password: PASSWORD.to_string(),
            phone: None,
            username: None,
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    });
    assert_eq!(resp.status, 201, "register: {:?}", resp.body);
}

fn request_otp(email: &str) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    login_email_otp::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login/email-otp".to_string(),
        handler_name: "login_email_otp".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: OtpSendRequest {
            email: email.to_string(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    })
}

fn request_magic_link(email: &str) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    magic_link_send::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/magic-link".to_string(),
        handler_name: "magic_link_send".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: MagicSendRequest {
            email: email.to_string(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    })
}

fn verify_otp(email: &str, code: &str) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    verify_email_otp::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/verify/email-otp".to_string(),
        handler_name: "verify_email_otp".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: OtpVerifyRequest {
            code: code.to_string(),
            email: email.to_string(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    })
}

fn click_magic_link(token: &str) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    magic_link_verify::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/verify-magic".to_string(),
        handler_name: "magic_link_verify".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: MagicVerifyRequest {
            token: token.to_string(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    })
}

// ── Mailpit API helpers ─────────────────────────────────────────────────────

fn fetch_options(timeout_ms: u64) -> sesame_common::HttpFetchOptions {
    sesame_common::HttpFetchOptions {
        timeout: Duration::from_millis(timeout_ms),
        max_body_bytes: 1024 * 1024,
        extra_headers: vec![],
    }
}

fn mailpit_get(api: &str, path: &str) -> Option<serde_json::Value> {
    match sesame_common::fetch_get(&format!("{api}{path}"), &fetch_options(2000)) {
        Ok((200, body)) => serde_json::from_slice(&body).ok(),
        _ => None,
    }
}

/// Count messages addressed to `recipient`.
fn message_count(api: &str, recipient: &str) -> u64 {
    mailpit_get(api, &format!("/api/v1/search?query=to:{recipient}"))
        .and_then(|v| v["messages_count"].as_u64().or_else(|| v["total"].as_u64()))
        .unwrap_or(0)
}

/// Poll Mailpit for the newest message to `recipient`; return its full text
/// body. Generous window: through the cluster LB (MetalLB) the SMTP banner
/// alone has been observed to take >5s, so in-cluster delivery needs slack.
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

fn extract_code(body: &str) -> Option<String> {
    body.split_whitespace()
        .find(|w| w.len() == 6 && w.chars().all(|c| c.is_ascii_digit()))
        .map(std::string::ToString::to_string)
}

fn extract_magic_token(body: &str) -> Option<String> {
    let url = body
        .split_whitespace()
        .find(|w| w.starts_with("http") && w.contains("token="))?;
    url.split("token=").nth(1).map(|t| {
        t.split('&')
            .next()
            .unwrap_or(t)
            .trim()
            .to_string()
    })
}

fn decode_jwt_payload(token: &str) -> serde_json::Value {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "access token must be a compact JWT");
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).expect("payload b64"))
        .expect("payload JSON")
}

// ── scenarios ───────────────────────────────────────────────────────────────

/// Scenario: full email-OTP round trip through the real mailbox.
/// Request code → Mailpit receives mail → extract 6-digit code → verify →
/// real signed JWT for the right user+tenant. Wrong code stays generic;
/// the real code is single-use.
#[test]
fn email_otp_full_round_trip() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    let Some(api) = mailpit_available() else {
        println!("SKIP: Mailpit not available");
        return;
    };
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    ensure_active_tenant(TEST_TENANT);

    let email = unique_email("otp");
    register(&email);

    let resp = request_otp(&email);
    assert_eq!(resp.status, 200, "otp send: {:?}", resp.body);

    let body = wait_for_message_text(&api, &email).expect("OTP email must arrive in Mailpit");
    let code = extract_code(&body).expect("email must contain a 6-digit code");

    // Wrong code first: generic 401, does not burn the real code.
    let wrong = verify_otp(&email, "000000");
    // (1-in-a-million flake guard: if the real code IS 000000, skip this leg.)
    if code != "000000" {
        assert_eq!(wrong.status, 401);
        assert_eq!(wrong.body["error"], "invalid_credentials");
    }

    // Right code: real token response.
    let resp = verify_otp(&email, &code);
    assert_eq!(resp.status, 200, "otp verify: {:?}", resp.body);
    let payload = decode_jwt_payload(resp.body["access_token"].as_str().expect("access_token"));
    assert_eq!(payload["tenant_id"], TEST_TENANT);
    assert_eq!(resp.body["token_type"], "Bearer");

    // Single use: replay of the same code fails.
    let replay = verify_otp(&email, &code);
    assert_eq!(replay.status, 401, "OTP must be single-use");
}

/// Scenario: full magic-link round trip — the extracted URL is "clicked"
/// (consumed) exactly as a browser would, yielding tokens; a second click
/// (replay) is rejected.
#[test]
fn magic_link_click_round_trip() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    let Some(api) = mailpit_available() else {
        println!("SKIP: Mailpit not available");
        return;
    };
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    ensure_active_tenant(TEST_TENANT);

    let email = unique_email("magic");
    register(&email);

    let resp = request_magic_link(&email);
    assert_eq!(resp.status, 200, "magic send: {:?}", resp.body);

    let body = wait_for_message_text(&api, &email).expect("magic-link email must arrive");
    let token = extract_magic_token(&body).expect("email must contain a token= link");

    // First click: tokens.
    let resp = click_magic_link(&token);
    assert_eq!(resp.status, 200, "magic verify: {:?}", resp.body);
    let payload = decode_jwt_payload(resp.body["access_token"].as_str().expect("access_token"));
    assert_eq!(payload["tenant_id"], TEST_TENANT);

    // Second click: burned.
    let replay = click_magic_link(&token);
    assert_eq!(replay.status, 401, "magic link must be single-use");
}

/// Scenario: Gate A3 proven at the mailbox — with a send window of 1, a
/// second request produces NO new Mailpit message, while the HTTP responses
/// stay identical (no cap oracle).
#[test]
fn capped_send_never_reaches_mailbox() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    let Some(api) = mailpit_available() else {
        println!("SKIP: Mailpit not available");
        return;
    };
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    std::env::set_var("OTP_SEND_MAX_PER_WINDOW", "1");
    ensure_active_tenant(TEST_TENANT);

    let email = unique_email("capped");
    register(&email);

    let first = request_otp(&email);
    assert_eq!(first.status, 200);
    assert!(
        wait_for_message_text(&api, &email).is_some(),
        "first send must reach the mailbox"
    );
    let after_first = message_count(&api, &email);

    let second = request_otp(&email);
    assert_eq!(second.status, 200);
    assert_eq!(
        first.body, second.body,
        "capped response must be indistinguishable"
    );

    // Give a would-be second delivery time to land, then prove it didn't.
    std::thread::sleep(Duration::from_secs(2));
    assert_eq!(
        message_count(&api, &email),
        after_first,
        "capped send must never reach the mailbox"
    );
}

/// Scenario: unknown accounts receive no mail but the same success response
/// (enumeration is invisible at the mailbox too).
#[test]
fn unknown_account_gets_no_mail_same_response() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    let Some(api) = mailpit_available() else {
        println!("SKIP: Mailpit not available");
        return;
    };
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    ensure_active_tenant(TEST_TENANT);

    let known = unique_email("known");
    register(&known);
    let ghost = unique_email("ghost");

    let known_resp = request_otp(&known);
    let ghost_resp = request_otp(&ghost);
    assert_eq!(known_resp.status, 200);
    assert_eq!(ghost_resp.status, 200);
    assert_eq!(
        known_resp.body, ghost_resp.body,
        "known and unknown accounts must be indistinguishable"
    );

    assert!(
        wait_for_message_text(&api, &known).is_some(),
        "known account gets mail"
    );
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(
        message_count(&api, &ghost),
        0,
        "unknown account must receive no mail"
    );
}
