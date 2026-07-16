//! Cross-service token lifecycle BDD (H7.1): register → login → JWT verify →
//! userinfo → refresh → logout → refresh rejected.
//!
//! **Run on ms02** (NFS canonical build host) with Kind postgres + redis
//! port-forwarded to localhost — see `just port-forward` on ms02:
//!
//! ```bash
//! ssh ms02 'source ~/.cargo/env && cd ~/Workspace/microscaler/seasame-idam/microservices && \
//!   cargo test -p sesame_idam_identity_login_service --test main_bdd token_lifecycle -- --nocapture'
//! ```
//!
//! Skips gracefully when Postgres/Redis are unreachable (e.g. Mac without
//! port-forwards). On ms02 with forwards active, both tests exercise the
//! full rotation path against live infrastructure.

use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::dispatcher::{HandlerRequest, HeaderVec};
use brrtrouter::ids::RequestId;
use brrtrouter::router::ParamVec;
use brrtrouter::security::JwtTokenStatusChecker;
use brrtrouter::typed::{TypedHandlerFor, TypedHandlerRequest};
use http::Method;
use sesame_common::jwt::{Ed25519Signer, SIGNING_KEY_ENV, SIGNING_KID_ENV};
use uuid::Uuid;

use sesame_idam_identity_login_service::controllers::{auth_login, auth_logout, auth_register};
use sesame_idam_identity_login_service_gen::handlers::auth_login::Request as LoginRequest;
use sesame_idam_identity_login_service_gen::handlers::auth_logout::Request as LogoutRequest;
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterRequest;
use sesame_idam_identity_session_service::controllers::{auth_refresh, oauth_userinfo};
use sesame_idam_identity_session_service_gen::handlers::oauth_userinfo::Request as UserinfoRequest;

use crate::common::ensure_active_tenant;

const TEST_TENANT: &str = "bdd-lifecycle-tenant";
const TEST_KID: &str = "bdd-lifecycle-kid";

static INIT: Once = Once::new();

fn postgres_reachable() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    format!("{host}:{port}")
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some()
}

fn redis_reachable() -> bool {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    redis::Client::open(url.as_str())
        .ok()
        .and_then(|c| c.get_connection().ok())
        .is_some()
}

/// Configure DB, Redis, and a shared Ed25519 signing key before any handler
/// touches the process-wide `SIGNER` lazy statics in login/session services.
pub(crate) fn infra_available() -> bool {
    if !postgres_reachable() || !redis_reachable() {
        return false;
    }

    INIT.call_once(|| {
        let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());

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
        std::env::set_var(
            "REDIS_URL",
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
        );

        let signer = Ed25519Signer::generate(TEST_KID).expect("test signing key");
        std::env::set_var(SIGNING_KEY_ENV, signer.pkcs8_b64());
        std::env::set_var(SIGNING_KID_ENV, TEST_KID);
    });
    true
}

pub(crate) fn unique_email(prefix: &str) -> String {
    format!("bddtest_{}_{}@example.com", prefix, Uuid::new_v4())
}

fn register_request(email: &str, password: &str) -> TypedHandlerRequest<RegisterRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/register".to_string(),
        handler_name: "auth_register".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RegisterRequest {
            email: email.to_string(),
            first_name: Some("Lifecycle".to_string()),
            last_name: Some("Test".to_string()),
            password: password.to_string(),
            phone: None,
            username: None,
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    }
}

fn login_request(email: &str, password: &str) -> TypedHandlerRequest<LoginRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login".to_string(),
        handler_name: "auth_login".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: LoginRequest {
            email: email.to_string(),
            organization_id: None,
            password: password.to_string(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    }
}

fn refresh_request(
    refresh_token: &str,
) -> TypedHandlerRequest<sesame_idam_identity_session_service_gen::handlers::auth_refresh::Request>
{
    TypedHandlerRequest {
        method: Method::POST,
        path: "/session/refresh".to_string(),
        handler_name: "auth_refresh".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: sesame_idam_identity_session_service_gen::handlers::auth_refresh::Request {
            refresh_token: refresh_token.to_string(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    }
}

fn logout_request(refresh_token: &str) -> TypedHandlerRequest<LogoutRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/logout".to_string(),
        handler_name: "auth_logout".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: LogoutRequest {
            refresh_token: Some(refresh_token.to_string()),
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    }
}

fn decode_jwt_payload(token: &str) -> serde_json::Value {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "expected compact JWT");
    let bytes = URL_SAFE_NO_PAD.decode(parts[1]).expect("payload base64");
    serde_json::from_slice(&bytes).expect("payload JSON")
}

fn claims_from_access_token(token: &str) -> serde_json::Value {
    decode_jwt_payload(token)
}

fn userinfo_request(access_token: &str) -> HandlerRequest {
    let payload = decode_jwt_payload(access_token);
    let tenant = payload["tenant_id"].as_str().expect("tenant_id claim");
    let (tx, _rx) = may::sync::mpsc::channel();
    let mut headers = HeaderVec::new();
    headers.push((Arc::from("x-tenant-id"), tenant.to_string()));
    HandlerRequest {
        request_id: RequestId::new(),
        method: Method::GET,
        path: "/identity/userinfo".to_string(),
        handler_name: "oauth_userinfo".to_string(),
        path_params: ParamVec::new(),
        query_params: ParamVec::new(),
        headers,
        cookies: HeaderVec::new(),
        body: None,
        jwt_claims: Some(claims_from_access_token(access_token)),
        reply_tx: tx,
        queue_guard: None,
    }
}

/// Assert the JSON body matches the hauliage E2E `TokenResponse` contract (H2.5).
pub(crate) fn assert_token_response_shape(
    body: &serde_json::Value,
    expected_user_id: Option<&str>,
) {
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["expires_in"].as_i64().unwrap_or(0) > 0);
    assert!(!body["access_token"].as_str().unwrap_or("").is_empty());
    assert!(!body["refresh_token"].as_str().unwrap_or("").is_empty());
    assert!(
        body["refresh_token_expires_in"].as_i64().unwrap_or(0) > 0,
        "refresh_token_expires_in must be present and positive"
    );
    let user_id = body["user_id"].as_str().expect("user_id");
    if let Some(expected) = expected_user_id {
        assert_eq!(user_id, expected);
    }
    assert!(Uuid::parse_str(user_id).is_ok(), "user_id must be a UUID");
    assert!(
        body["scope"].as_str().is_some(),
        "scope should be returned for hauliage clients"
    );

    let header: serde_json::Value = {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        let h = body["access_token"]
            .as_str()
            .unwrap()
            .split('.')
            .next()
            .unwrap();
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(h).unwrap()).unwrap()
    };
    assert_eq!(header["alg"], "EdDSA");
    assert_eq!(header["typ"], "at+jwt");
}

fn test_signer() -> Ed25519Signer {
    Ed25519Signer::from_env()
        .expect("signer from env")
        .expect("signing key env must be set by infra_available")
}

/// Scenario: Full token lifecycle across login + session services.
///
/// Given a new user on the test tenant
/// When they register, fetch userinfo, refresh, and logout
/// Then each step returns valid tokens and post-logout refresh is rejected
#[test]
fn full_token_lifecycle_register_userinfo_refresh_logout() {
    if !infra_available() {
        println!("SKIP: Postgres and/or Redis not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);

    let email = unique_email("lifecycle");
    let password = "SecureP@ss123!";
    let signer = test_signer();

    // ── Register ──
    let reg = auth_register::handle(register_request(&email, password));
    assert_eq!(reg.status, 201, "register: {:?}", reg.body);
    assert_token_response_shape(&reg.body, None);
    let user_id = reg.body["user_id"].as_str().unwrap().to_string();
    let access_token = reg.body["access_token"].as_str().unwrap();
    signer
        .verify(access_token)
        .expect("register access token signature");

    // ── Userinfo (session service, DB-backed profile) ──
    let userinfo_req = userinfo_request(access_token);
    let userinfo_typed = TypedHandlerRequest::<UserinfoRequest>::from_handler(userinfo_req)
        .expect("typed userinfo request");
    let userinfo = oauth_userinfo::handle(userinfo_typed);
    assert_eq!(userinfo.status, 200, "userinfo: {}", userinfo.body);
    assert_eq!(userinfo.body["sub"], user_id);
    assert_eq!(userinfo.body["user_id"], user_id);
    assert_eq!(userinfo.body["email"], email);

    // ── Login (same user, fresh tokens) ──
    let login = auth_login::handle(login_request(&email, password));
    assert_eq!(login.status, 200, "login: {:?}", login.body);
    assert_token_response_shape(&login.body, Some(&user_id));
    let refresh_token = login.body["refresh_token"].as_str().unwrap().to_string();
    signer
        .verify(login.body["access_token"].as_str().unwrap())
        .expect("login access token signature");

    // ── Refresh (session service, Redis rotation) ──
    let refreshed = auth_refresh::handle(refresh_request(&refresh_token));
    assert_eq!(refreshed.status, 200, "refresh: {:?}", refreshed.body);
    assert_token_response_shape(&refreshed.body, Some(&user_id));
    let refresh_body = &refreshed.body;
    signer
        .verify(refresh_body["access_token"].as_str().unwrap())
        .expect("refreshed access token signature");
    assert!(
        refresh_body["refresh_token"]
            .as_str()
            .unwrap_or("")
            .contains('.'),
        "rotated refresh token must be a signed JWT"
    );

    let rotated_refresh = refresh_body["refresh_token"].as_str().unwrap().to_string();

    // Old refresh token must not rotate again (denylisted after rotation)
    let reuse = auth_refresh::handle(refresh_request(&refresh_token));
    assert_eq!(
        reuse.status, 401,
        "pre-rotation refresh must return 401: {:?}",
        reuse.body
    );
    assert!(
        reuse.body["error"].as_str().is_some(),
        "401 body must include OAuth error code"
    );

    // ── Logout (revoke family in Redis) ──
    let access_payload = decode_jwt_payload(login.body["access_token"].as_str().unwrap());
    let access_jti = access_payload["jti"]
        .as_str()
        .expect("access jti")
        .to_string();
    let access_exp = access_payload["exp"].as_u64().expect("access exp");

    let mut logout_req = logout_request(&rotated_refresh);
    logout_req.jwt_claims = Some(serde_json::json!({
        "sub": user_id,
        "tenant_id": TEST_TENANT,
        "jti": access_jti,
        "exp": access_exp,
    }));
    let logout = auth_logout::handle(logout_req);
    assert!(logout.error.is_empty(), "logout error: {:?}", logout.error);

    // ── Denylisted access token rejected by token-status read side ──
    let checker = sesame_common::token_status::SesameTokenStatusChecker::from_redis_url(
        &std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
    )
    .expect("token status checker");
    assert_eq!(
        checker.check(&access_payload),
        brrtrouter::security::JwtTokenStatus::Revoked,
        "logged-out access token must be denylisted"
    );

    // ── Post-logout refresh rejected ──
    let after_logout = auth_refresh::handle(refresh_request(&rotated_refresh));
    assert_eq!(
        after_logout.status, 401,
        "refresh after logout must return 401: {:?}",
        after_logout.body
    );
}

/// Scenario: Register response alone satisfies the hauliage `TokenResponse` fixture.
#[test]
fn register_token_response_matches_hauliage_contract() {
    if !infra_available() {
        println!("SKIP: Postgres and/or Redis not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);

    let resp = auth_register::handle(register_request(&unique_email("fixture"), "SecureP@ss123!"));
    assert_eq!(resp.status, 201);
    assert_token_response_shape(&resp.body, None);
}
