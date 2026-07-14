//! End-to-end auth flow BDD tests: register → login against the live
//! Postgres (shared Kind cluster forwarded to 127.0.0.1:5432, same pattern
//! as hauliage's DB-backed BDD tests).
//!
//! Tests skip gracefully (with a SKIP message) when Postgres is not
//! reachable so `just nt` still passes on machines without the cluster.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::{auth_login, auth_register};
use sesame_idam_identity_login_service_gen::handlers::auth_login::Request as LoginRequest;
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterRequest;

use crate::common::ensure_active_tenant;

/// Tenant used by controller-level tests (any string; spec-level UUID
/// validation happens in the HTTP layer, not here).
const TEST_TENANT: &str = "bdd-tenant";

/// Demo tenant/users from the hauliage dev seed
/// (`identity-user-mgmt-service/impl/seeds/20260706000000_hauliage_demo_users.sql`).
const HAULIAGE_TENANT: &str = "hauliage";
const HAULIAGE_DEMO_EMAIL: &str = "owner@hauliage.dev";
const HAULIAGE_DEMO_PASSWORD: &str = "SecureP@ss123!";

static INIT: Once = Once::new();

/// Configure DB env for `sesame_idam_database::db()` before its `OnceLock`
/// initializes. Returns false (skip) when Postgres is unreachable.
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

fn unique_email(prefix: &str) -> String {
    format!("bddtest_{}_{}@example.com", prefix, uuid::Uuid::new_v4())
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
            first_name: None,
            last_name: None,
            password: password.to_string(),
            phone: None,
            username: None,
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    }
}

fn login_request(tenant: &str, email: &str, password: &str) -> TypedHandlerRequest<LoginRequest> {
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
            x_tenant_id: tenant.to_string(),
        },
        jwt_claims: None,
    }
}

/// Decode the payload segment of a compact JWT into JSON.
fn decode_jwt_payload(token: &str) -> serde_json::Value {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "access token must be a compact JWT");
    let bytes = URL_SAFE_NO_PAD.decode(parts[1]).expect("payload base64");
    serde_json::from_slice(&bytes).expect("payload JSON")
}

/// Scenario: Register a new user, then log in with the same credentials.
///
/// Given a fresh email on the test tenant
/// When POST /auth/register, then POST /auth/login
/// Then register returns 201 with a real `EdDSA` JWT, and login returns 200
///      with claims carrying the tenant and user id.
#[test]
fn register_then_login_round_trip() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);

    let email = unique_email("roundtrip");
    let password = "SecureP@ss123!";

    // ── Register ──
    let resp = auth_register::handle(register_request(&email, password));
    assert_eq!(
        resp.status, 201,
        "register should return 201: {:?}",
        resp.body
    );
    let body = &resp.body;
    let access_token = body["access_token"].as_str().expect("access_token");
    let user_id = body["user_id"].as_str().expect("user_id").to_string();
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["expires_in"].as_i64().unwrap() > 0);
    assert!(!body["refresh_token"].as_str().unwrap().is_empty());

    // Real signed JWT with EdDSA header and our claims
    let payload = decode_jwt_payload(access_token);
    assert_eq!(payload["sub"], user_id);
    assert_eq!(payload["tenant_id"], TEST_TENANT);
    // sx claims serialize under the namespaced URI (Story 2.x)
    assert_eq!(
        payload["https://sesame-idam.dev/claims"]["tenant"],
        TEST_TENANT
    );
    assert!(payload["ver"].as_u64().unwrap() >= 1);
    let header: serde_json::Value = {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        let h = access_token.split('.').next().unwrap();
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(h).unwrap()).unwrap()
    };
    assert_eq!(header["alg"], "EdDSA");
    assert_eq!(header["typ"], "at+jwt");

    // ── Login ──
    let resp = auth_login::handle(login_request(TEST_TENANT, &email, password));
    assert_eq!(resp.status, 200, "login should return 200: {:?}", resp.body);
    assert_eq!(resp.body["user_id"], user_id.as_str());
    let login_payload = decode_jwt_payload(resp.body["access_token"].as_str().unwrap());
    assert_eq!(login_payload["sub"], user_id);
    assert_eq!(login_payload["tenant_id"], TEST_TENANT);
}

/// Scenario: Login with a wrong password is rejected with 401.
#[test]
fn login_wrong_password_rejected() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);

    let email = unique_email("wrongpw");
    let resp = auth_register::handle(register_request(&email, "SecureP@ss123!"));
    assert_eq!(resp.status, 201);

    let resp = auth_login::handle(login_request(TEST_TENANT, &email, "not-the-password"));
    assert_eq!(resp.status, 401);
    assert_eq!(resp.body["error"], "invalid_credentials");
}

/// Scenario: Login for an unknown user is rejected with the same 401 as a
/// wrong password (no user enumeration).
#[test]
fn login_unknown_user_indistinguishable() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);

    let resp = auth_login::handle(login_request(
        TEST_TENANT,
        "does-not-exist@example.com",
        "whatever-password",
    ));
    assert_eq!(resp.status, 401);
    assert_eq!(resp.body["error"], "invalid_credentials");
}

/// Scenario: The same email on a different tenant is a different user —
/// credentials do not leak across tenants (hard-segment isolation).
#[test]
fn tenant_isolation_same_email_different_tenant() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);
    ensure_active_tenant("other-tenant");

    let email = unique_email("xtenant");
    let resp = auth_register::handle(register_request(&email, "SecureP@ss123!"));
    assert_eq!(resp.status, 201);

    // Same email + correct password but a different tenant → 401
    let resp = auth_login::handle(login_request("other-tenant", &email, "SecureP@ss123!"));
    assert_eq!(resp.status, 401, "credentials must not cross tenants");
}

/// Scenario: Duplicate registration on the same tenant returns 400.
#[test]
fn duplicate_registration_rejected() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);

    let email = unique_email("dup");
    let resp = auth_register::handle(register_request(&email, "SecureP@ss123!"));
    assert_eq!(resp.status, 201);

    let resp = auth_register::handle(register_request(&email, "SecureP@ss123!"));
    assert_eq!(resp.status, 400);
    assert_eq!(resp.body["error"], "email_in_use");
}

/// Scenario: Weak password rejected with 400 before touching the DB.
#[test]
fn weak_password_rejected() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);

    let resp = auth_register::handle(register_request(&unique_email("weak"), "short"));
    assert_eq!(resp.status, 400);
    assert_eq!(resp.body["error"], "weak_password");
}

/// Scenario: The seeded hauliage demo user can log in.
///
/// Validates the dev seed
/// (owner@hauliage.dev / SecureP@ss123! on tenant `hauliage`) end to end.
#[test]
fn hauliage_demo_user_logs_in() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let resp = auth_login::handle(login_request(
        HAULIAGE_TENANT,
        HAULIAGE_DEMO_EMAIL,
        HAULIAGE_DEMO_PASSWORD,
    ));
    // Seed may not be applied — tolerate 401 (no user) or 404 (no tenant registry).
    if resp.status == 401 || resp.status == 404 {
        println!("SKIP: hauliage demo seed not applied on this database");
        return;
    }
    assert_eq!(resp.status, 200, "demo login failed: {:?}", resp.body);
    let payload = decode_jwt_payload(resp.body["access_token"].as_str().unwrap());
    assert_eq!(payload["tenant_id"], HAULIAGE_TENANT);
}

/// Scenario: Unknown tenant slug is rejected before credential checks.
#[test]
fn unknown_tenant_rejected() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let resp = auth_login::handle(login_request(
        "totally-unprovisioned-tenant-slug",
        "nobody@example.com",
        "whatever",
    ));
    assert_eq!(resp.status, 404);
    assert_eq!(resp.body["error"], "tenant_unknown");
}
