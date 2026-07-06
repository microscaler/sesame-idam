//! Live-database BDD tests for `GET/PATCH /identity/me` (raw handlers).
//!
//! Builds `HandlerRequest`s with validated-JWT claims attached (as the
//! security provider would) and exercises the DB-backed profile flow against
//! the shared Kind postgres. Skips gracefully when Postgres is unreachable.

use std::net::TcpStream;
use std::sync::{Arc, Once};
use std::time::Duration;

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse, HeaderVec};
use brrtrouter::ids::RequestId;
use brrtrouter::router::ParamVec;
use http::Method;
use uuid::Uuid;

use sesame_idam_identity_session_service::controllers::{users_me_get, users_me_patch};

const TEST_TENANT: &str = "bdd-me-tenant";

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
        // Each nextest test runs in its own process with its own pool —
        // keep pools tiny so parallel DB tests don't exhaust Postgres
        // max_connections.
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

fn raw_client() -> may_postgres::Client {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = std::env::var("TEST_DB_USER").unwrap_or_else(|_| "sesame_idam".to_string());
    let pass =
        std::env::var("TEST_DB_PASS").unwrap_or_else(|_| "dev_password_change_in_prod".to_string());
    let db = std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "sesame_idam".to_string());
    may_postgres::connect(&format!("postgres://{user}:{pass}@{host}:{port}/{db}"))
        .expect("connect test DB")
}

fn insert_user(client: &may_postgres::Client, user_id: Uuid, tenant: &str) {
    let email = format!("bddtest_me_{user_id}@example.com");
    client
        .batch_execute(&format!(
            "INSERT INTO sesame_idam.users \
             (id, email, password_hash, tenant_id, status, email_verified, phone, phone_verified, created_at, updated_at) \
             VALUES ('{user_id}', '{email}', 'x', '{tenant}', 'active', true, NULL, false, NOW(), NOW());"
        ))
        .expect("insert user");
}

fn cleanup_user(client: &may_postgres::Client, user_id: Uuid) {
    client
        .batch_execute(&format!(
            "DELETE FROM sesame_idam.users WHERE id = '{user_id}';"
        ))
        .expect("cleanup");
}

/// Build a `HandlerRequest` as it arrives after security validation: claims
/// attached, X-Tenant-ID header set.
fn me_request(
    method: Method,
    claims: Option<serde_json::Value>,
    header_tenant: Option<&str>,
    body: Option<serde_json::Value>,
) -> (HandlerRequest, may::sync::mpsc::Receiver<HandlerResponse>) {
    let (tx, rx) = may::sync::mpsc::channel();
    let mut headers = HeaderVec::new();
    if let Some(tenant) = header_tenant {
        headers.push((Arc::from("x-tenant-id"), tenant.to_string()));
    }
    let req = HandlerRequest {
        request_id: RequestId::new(),
        method,
        path: "/identity/me".to_string(),
        handler_name: "users_me_get".to_string(),
        path_params: ParamVec::new(),
        query_params: ParamVec::new(),
        headers,
        cookies: HeaderVec::new(),
        body,
        jwt_claims: claims,
        reply_tx: tx,
        queue_guard: None,
    };
    (req, rx)
}

fn claims_for(user_id: Uuid, tenant: &str) -> serde_json::Value {
    serde_json::json!({
        "sub": user_id.to_string(),
        "tenant_id": tenant,
        "iss": "https://idam.example.com",
        "aud": ["sesame-idam"],
    })
}

/// Scenario: Authenticated user fetches their profile (no profile row yet).
#[test]
fn get_me_returns_user_with_null_profile_fields() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let client = raw_client();
    let user_id = Uuid::new_v4();
    insert_user(&client, user_id, TEST_TENANT);

    let (req, _rx) = me_request(
        Method::GET,
        Some(claims_for(user_id, TEST_TENANT)),
        Some(TEST_TENANT),
        None,
    );
    let resp = users_me_get::handle_raw(&req);

    assert_eq!(resp.status, 200, "body: {}", resp.body);
    assert_eq!(resp.body["user_id"], user_id.to_string());
    assert_eq!(resp.body["sub"], user_id.to_string());
    assert_eq!(
        resp.body["email"],
        format!("bddtest_me_{user_id}@example.com")
    );
    assert_eq!(resp.body["email_verified"], true);
    assert_eq!(resp.body["is_active"], true);
    assert!(resp.body["first_name"].is_null());
    assert!(resp.body["name"].is_null());

    cleanup_user(&client, user_id);
}

/// Scenario: PATCH creates the profile row and GET reflects it.
#[test]
fn patch_me_upserts_profile_and_get_reflects_it() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let client = raw_client();
    let user_id = Uuid::new_v4();
    insert_user(&client, user_id, TEST_TENANT);

    // PATCH with first/last name
    let (req, _rx) = me_request(
        Method::PATCH,
        Some(claims_for(user_id, TEST_TENANT)),
        Some(TEST_TENANT),
        Some(serde_json::json!({"first_name": "Alice", "last_name": "Smith"})),
    );
    let resp = users_me_patch::handle_raw(&req);
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    assert_eq!(resp.body["first_name"], "Alice");
    assert_eq!(resp.body["name"], "Alice Smith");

    // Second PATCH updates only avatar_url — names must be preserved
    let (req, _rx) = me_request(
        Method::PATCH,
        Some(claims_for(user_id, TEST_TENANT)),
        Some(TEST_TENANT),
        Some(serde_json::json!({"avatar_url": "https://example.com/a.png"})),
    );
    let resp = users_me_patch::handle_raw(&req);
    assert_eq!(resp.status, 200);
    assert_eq!(
        resp.body["first_name"], "Alice",
        "partial update must not clear names"
    );
    assert_eq!(resp.body["avatar_url"], "https://example.com/a.png");

    // GET reflects the stored profile
    let (req, _rx) = me_request(
        Method::GET,
        Some(claims_for(user_id, TEST_TENANT)),
        Some(TEST_TENANT),
        None,
    );
    let resp = users_me_get::handle_raw(&req);
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body["last_name"], "Smith");
    assert_eq!(resp.body["avatar_url"], "https://example.com/a.png");

    cleanup_user(&client, user_id);
}

/// Scenario: Missing claims → 401 (never leaks another user's data).
#[test]
fn get_me_without_claims_is_unauthorized() {
    // No DB needed: the request is rejected before any query.
    let (req, _rx) = me_request(Method::GET, None, Some(TEST_TENANT), None);
    let resp = users_me_get::handle_raw(&req);
    assert_eq!(resp.status, 401);
}

/// Scenario: X-Tenant-ID header disagreeing with the token tenant → 401.
#[test]
fn get_me_tenant_mismatch_is_unauthorized() {
    let (req, _rx) = me_request(
        Method::GET,
        Some(claims_for(Uuid::new_v4(), TEST_TENANT)),
        Some("some-other-tenant"),
        None,
    );
    let resp = users_me_get::handle_raw(&req);
    assert_eq!(resp.status, 401);
}

/// Scenario: Token for a user on another tenant cannot read this tenant's
/// data (defence in depth: query is tenant-scoped).
#[test]
fn get_me_cross_tenant_user_is_unauthorized() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let client = raw_client();
    let user_id = Uuid::new_v4();
    insert_user(&client, user_id, "bdd-me-other-tenant");

    // Claims + header both say TEST_TENANT, but the user lives on another
    // tenant — the tenant-scoped lookup must not find them.
    let (req, _rx) = me_request(
        Method::GET,
        Some(claims_for(user_id, TEST_TENANT)),
        Some(TEST_TENANT),
        None,
    );
    let resp = users_me_get::handle_raw(&req);
    assert_eq!(resp.status, 401);

    cleanup_user(&client, user_id);
}

/// Scenario: PATCH exceeding maxLength is rejected with 400.
#[test]
fn patch_me_rejects_oversized_names() {
    let (req, _rx) = me_request(
        Method::PATCH,
        Some(claims_for(Uuid::new_v4(), TEST_TENANT)),
        Some(TEST_TENANT),
        Some(serde_json::json!({"first_name": "x".repeat(101)})),
    );
    let resp = users_me_patch::handle_raw(&req);
    assert_eq!(resp.status, 400);
    assert_eq!(resp.body["error"], "validation_error");
}
