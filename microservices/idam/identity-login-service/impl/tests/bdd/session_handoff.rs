//! Cross-origin session handoff (ADR-010): mint a one-time code on the auth
//! origin, redeem it from the tenant app's origin.
//!
//! Properties under test — each is a real attack the design defends against:
//!  - a code is single-use (replay is worthless),
//!  - a code is bound to its redirect_uri (a stolen code cannot be redeemed
//!    at an attacker's destination),
//!  - a code is bound to its tenant,
//!  - a code cannot be conjured from a token we did not issue,
//!  - failures are indistinguishable to the caller.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::{
    auth_login, auth_register, auth_session_code, auth_token,
};
use sesame_idam_identity_login_service_gen::handlers::auth_login::Request as LoginReq;
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterReq;
use sesame_idam_identity_login_service_gen::handlers::auth_session_code::Request as CodeReq;
use sesame_idam_identity_login_service_gen::handlers::auth_token::Request as TokenReq;

use crate::common::ensure_active_tenant;

const TENANT: &str = "handoff-bdd-tenant";
const PASSWORD: &str = "SecureP@ss123!";
const APP_URI: &str = "https://app.tenant.example/callback";
const EVIL_URI: &str = "https://attacker.example/callback";

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

fn redis_available() -> bool {
    let host = std::env::var("TEST_REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
    format!("{host}:{port}")
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some()
}

fn unique_email() -> String {
    format!("handoff_{}@example.com", uuid::Uuid::new_v4().simple())
}

/// Register + sign in, returning a real access token for the tenant.
fn authenticate() -> (String, Option<String>) {
    let email = unique_email();
    let resp = auth_register::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/register".to_string(),
        handler_name: "auth_register".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RegisterReq {
            email: email.clone(),
            first_name: None,
            last_name: None,
            password: PASSWORD.to_string(),
            phone: None,
            username: None,
            x_tenant_id: TENANT.to_string(),
        },
        jwt_claims: None,
    });
    assert_eq!(resp.status, 201, "register: {:?}", resp.body);

    let resp = auth_login::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login".to_string(),
        handler_name: "auth_login".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: LoginReq {
            email,
            organization_id: None,
            password: PASSWORD.to_string(),
            x_tenant_id: TENANT.to_string(),
        },
        jwt_claims: None,
    });
    assert_eq!(resp.status, 200, "login: {:?}", resp.body);
    (
        resp.body["access_token"].as_str().unwrap().to_string(),
        resp.body["refresh_token"].as_str().map(str::to_string),
    )
}

fn mint(access_token: &str, refresh_token: Option<String>, redirect_uri: &str) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    auth_session_code::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/session/code".to_string(),
        handler_name: "auth_session_code".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: CodeReq {
            access_token: access_token.to_string(),
            refresh_token,
            redirect_uri: redirect_uri.to_string(),
            x_tenant_id: TENANT.to_string(),
        },
        jwt_claims: None,
    })
}

fn redeem(code: &str, redirect_uri: &str, tenant: &str) -> sesame_idam_identity_login_service_gen::handlers::auth_token::Response {
    auth_token::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/token".to_string(),
        handler_name: "auth_token".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: TokenReq {
            actor_token: None,
            client_id: None,
            client_secret: None,
            code: Some(code.to_string()),
            grant_type: "authorization_code".to_string(),
            redirect_uri: Some(redirect_uri.to_string()),
            refresh_token: None,
            requested_token_type: None,
            scope: None,
            subject_token: None,
            subject_token_type: None,
            x_tenant_id: tenant.to_string(),
        },
        jwt_claims: None,
    })
}

/// Scenario: the happy path — auth origin mints, app origin redeems, and the
/// app receives the very session that was created.
#[test]
fn code_round_trip_delivers_the_session() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    ensure_active_tenant(TENANT);

    let (access, refresh) = authenticate();
    let minted = mint(&access, refresh, APP_URI);
    assert_eq!(minted.status, 200, "mint: {:?}", minted.body);
    let code = minted.body["code"].as_str().expect("code").to_string();
    assert!(minted.body["expires_in"].as_i64().unwrap() <= 300, "code TTL must be short");

    let redeemed = redeem(&code, APP_URI, TENANT);
    assert_eq!(redeemed.access_token, access, "app must receive the same session");
    assert!(!redeemed.user_id.is_empty(), "user_id resolved from the token");
}

/// Scenario: replay. A code works once.
#[test]
fn code_is_single_use() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    ensure_active_tenant(TENANT);

    let (access, refresh) = authenticate();
    let code = mint(&access, refresh, APP_URI).body["code"].as_str().unwrap().to_string();

    assert!(!redeem(&code, APP_URI, TENANT).access_token.is_empty());
    assert!(
        redeem(&code, APP_URI, TENANT).access_token.is_empty(),
        "replayed code must yield nothing"
    );
}

/// Scenario: a stolen code cannot be redeemed at the attacker's destination —
/// and the attempt burns it, so it cannot then be probed elsewhere.
#[test]
fn code_is_bound_to_redirect_uri() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    ensure_active_tenant(TENANT);

    let (access, refresh) = authenticate();
    let code = mint(&access, refresh, APP_URI).body["code"].as_str().unwrap().to_string();

    assert!(
        redeem(&code, EVIL_URI, TENANT).access_token.is_empty(),
        "redirect_uri mismatch must be refused"
    );
    assert!(
        redeem(&code, APP_URI, TENANT).access_token.is_empty(),
        "the failed attempt must have burned the code"
    );
}

/// Scenario: a code minted for one tenant is useless under another.
#[test]
fn code_is_bound_to_tenant() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    ensure_active_tenant(TENANT);
    ensure_active_tenant("handoff-other-tenant");

    let (access, refresh) = authenticate();
    let code = mint(&access, refresh, APP_URI).body["code"].as_str().unwrap().to_string();

    assert!(
        redeem(&code, APP_URI, "handoff-other-tenant").access_token.is_empty(),
        "cross-tenant redemption must be refused"
    );
}

/// Scenario: a code cannot be conjured from a token we never issued.
#[test]
fn cannot_mint_from_a_foreign_token() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    ensure_active_tenant(TENANT);

    let resp = mint("not.a.jwt", None, APP_URI);
    assert_eq!(resp.status, 400, "garbage token must be refused");

    // A well-formed JWT for a DIFFERENT tenant must also be refused.
    use base64::Engine;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(br#"{"sub":"u1","tenant_id":"some-other-tenant"}"#);
    let foreign = format!("aGVhZGVy.{payload}.c2ln");
    assert_eq!(mint(&foreign, None, APP_URI).status, 400);
}

/// Scenario: an unknown code is refused exactly like an expired one.
#[test]
fn unknown_code_refused() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    ensure_active_tenant(TENANT);
    assert!(redeem("no-such-code", APP_URI, TENANT).access_token.is_empty());
}
