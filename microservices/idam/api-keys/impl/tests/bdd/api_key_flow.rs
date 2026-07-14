//! Live-database BDD tests for API key create → validate.
//!
//! Runs against the shared Kind postgres; skips gracefully when Postgres is
//! unreachable so `just nt` passes without the cluster.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use uuid::Uuid;

use sesame_idam_api_keys::controllers::{create_api_key, validate_api_key};
use sesame_idam_api_keys_gen::handlers::create_api_key::Request as CreateRequest;
use sesame_idam_api_keys_gen::handlers::validate_api_key::Request as ValidateRequest;

const TEST_TENANT: &str = "bdd-apikey-tenant";

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
        // Process-per-test pools — keep them tiny (see wiki log 2026-07-06).
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

/// Fixture user (`api_keys.user_id` FK targets users).
fn insert_user(client: &may_postgres::Client, user_id: Uuid, tenant: &str) {
    let email = format!("bddtest_apikey_{user_id}@example.com");
    client
        .batch_execute(&format!(
            "INSERT INTO sesame_idam.users \
             (id, email, password_hash, tenant_id, status, email_verified, phone, phone_verified, created_at, updated_at) \
             VALUES ('{user_id}', '{email}', 'x', '{tenant}', 'active', true, NULL, false, NOW(), NOW());"
        ))
        .expect("insert user");
}

fn cleanup_user(client: &may_postgres::Client, user_id: Uuid) {
    // api_keys rows cascade from the user delete.
    client
        .batch_execute(&format!(
            "DELETE FROM sesame_idam.users WHERE id = '{user_id}';"
        ))
        .expect("cleanup");
}

fn create_request(
    tenant: &str,
    user_id: Uuid,
    expires_in_days: Option<i64>,
) -> TypedHandlerRequest<CreateRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/api-keys".to_string(),
        handler_name: "create_api_key".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: CreateRequest {
            expires_in_days: expires_in_days.map(serde_json::Value::from),
            metadata: None,
            name: "bdd test key".to_string(),
            org_id: None,
            permissions: Some(vec!["jobs:read".to_string()]),
            user_id: Some(serde_json::Value::String(user_id.to_string())),
            x_tenant_id: tenant.to_string(),
        },
        jwt_claims: None,
    }
}

fn validate_request(
    tenant: &str,
    api_key: &str,
    key_type: Option<&str>,
) -> TypedHandlerRequest<ValidateRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/api-keys/validate".to_string(),
        handler_name: "validate_api_key".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: ValidateRequest {
            api_key: api_key.to_string(),
            key_type: key_type.map(String::from),
            x_tenant_id: tenant.to_string(),
        },
        jwt_claims: None,
    }
}

/// Scenario: Create a key, validate it, and confirm the plaintext is never
/// stored.
#[test]
fn create_then_validate_round_trip() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let client = raw_client();
    let user_id = Uuid::new_v4();
    insert_user(&client, user_id, TEST_TENANT);

    // Create
    let resp = create_api_key::handle(create_request(TEST_TENANT, user_id, None));
    assert_eq!(resp.status, 201, "create: {}", resp.body);
    let plaintext = resp.body["api_key"].as_str().expect("api_key").to_string();
    assert!(plaintext.starts_with("sk_"), "key must be sk_-prefixed");
    let key_id = resp.body["api_key_id"].as_str().expect("api_key_id");

    // Plaintext never stored — only the hash
    let rows = client
        .query(
            &client
                .prepare("SELECT key_hash, key_prefix FROM sesame_idam.api_keys WHERE id = $1")
                .expect("prepare"),
            &[&key_id.parse::<Uuid>().unwrap()],
        )
        .expect("query");
    let key_hash: String = rows[0].get(0);
    let key_prefix: String = rows[0].get(1);
    assert_ne!(key_hash, plaintext);
    assert_eq!(key_hash.len(), 64, "sha256 hex");
    assert!(plaintext.starts_with(&key_prefix));

    // Validate — valid
    let resp = validate_api_key::handle(validate_request(TEST_TENANT, &plaintext, None));
    assert_eq!(resp.status, 200, "validate: {}", resp.body);
    assert_eq!(resp.body["valid"], true);
    assert_eq!(resp.body["user_id"], user_id.to_string());
    assert_eq!(resp.body["scope_type"], "personal");
    assert_eq!(resp.body["permissions"], serde_json::json!(["jobs:read"]));

    // key_type=personal matches; key_type=org does not
    let resp =
        validate_api_key::handle(validate_request(TEST_TENANT, &plaintext, Some("personal")));
    assert_eq!(resp.status, 200);
    let resp = validate_api_key::handle(validate_request(TEST_TENANT, &plaintext, Some("org")));
    assert_eq!(
        resp.status, 401,
        "org-type filter must reject a personal key"
    );

    cleanup_user(&client, user_id);
}

/// Scenario: Unknown keys and cross-tenant lookups are 401.
#[test]
fn invalid_and_cross_tenant_keys_rejected() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let client = raw_client();
    let user_id = Uuid::new_v4();
    insert_user(&client, user_id, TEST_TENANT);

    let resp = create_api_key::handle(create_request(TEST_TENANT, user_id, None));
    assert_eq!(resp.status, 201);
    let plaintext = resp.body["api_key"].as_str().unwrap().to_string();

    // Unknown key
    let resp = validate_api_key::handle(validate_request(TEST_TENANT, "sk_deadbeef", None));
    assert_eq!(resp.status, 401);
    assert_eq!(resp.body["valid"], false);

    // Right key, wrong tenant — hard-segment isolation
    let resp = validate_api_key::handle(validate_request("other-tenant", &plaintext, None));
    assert_eq!(resp.status, 401, "keys must not validate across tenants");

    cleanup_user(&client, user_id);
}

/// Scenario: Expired keys report valid=false, `is_expired=true` (200).
#[test]
fn expired_key_reports_expired() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let client = raw_client();
    let user_id = Uuid::new_v4();
    insert_user(&client, user_id, TEST_TENANT);

    let resp = create_api_key::handle(create_request(TEST_TENANT, user_id, Some(1)));
    assert_eq!(resp.status, 201);
    let plaintext = resp.body["api_key"].as_str().unwrap().to_string();
    let key_id: Uuid = resp.body["api_key_id"].as_str().unwrap().parse().unwrap();

    // Force expiry in the past
    client
        .batch_execute(&format!(
            "UPDATE sesame_idam.api_keys SET expires_at = NOW() - INTERVAL '1 day' WHERE id = '{key_id}';"
        ))
        .expect("expire key");

    let resp = validate_api_key::handle(validate_request(TEST_TENANT, &plaintext, None));
    assert_eq!(resp.status, 200, "expired is 200 with flags: {}", resp.body);
    assert_eq!(resp.body["valid"], false);
    assert_eq!(resp.body["is_expired"], true);

    cleanup_user(&client, user_id);
}

/// Scenario: Key creation requires a scope (user or org).
#[test]
fn create_without_scope_rejected() {
    // No DB needed — validation happens before any query.
    let mut req = create_request(TEST_TENANT, Uuid::new_v4(), None);
    req.data.user_id = None;
    req.data.org_id = None;
    let resp = create_api_key::handle(req);
    assert_eq!(resp.status, 400);
}
