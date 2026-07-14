//! Live-database BDD tests for `POST /authz/principals/effective`.
//!
//! Runs against the shared Kind postgres (same pattern as
//! identity-login-service's `auth_flow` tests) and skips gracefully when
//! Postgres is unreachable so `just nt` passes without the cluster.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use chrono::Utc;
use http::Method;
use lifeguard::LifeExecutor;
use uuid::Uuid;

use sesame_idam_authz_core::controllers::principal_effective::handle;
use sesame_idam_authz_core_gen::handlers::principal_effective::Request;

const TEST_TENANT: &str = "bdd-authz-tenant";
const TEST_APP: &str = "33333333-8a2d-4c41-8b4b-ae43ce79a494";
/// Hauliage login uses portal id `frontend` for principal_effective app scope.
const HAULIAGE_PORTAL_APP: &str = "frontend";

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

fn make_request(user_id: &str, tenant_id: &str) -> TypedHandlerRequest<Request> {
    make_request_with_app(user_id, tenant_id, TEST_APP)
}

fn make_request_with_app(
    user_id: &str,
    tenant_id: &str,
    app_id: &str,
) -> TypedHandlerRequest<Request> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/principals/effective".to_string(),
        handler_name: "principal_effective".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            user_id: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            app_id: app_id.to_string(),
            org_id: None,
            include_inherited: Some(true),
            x_tenant_id: tenant_id.to_string(),
        },
        jwt_claims: None,
    }
}

/// Scenario: A seeded role assignment is returned for the principal.
///
/// Given a user with an OWNER role assignment in the test tenant
/// When POST /authz/principals/effective
/// Then the response roles contain OWNER (and cross-tenant queries do not).
#[test]
fn role_assignments_resolved_from_database() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let user_id = Uuid::new_v4();
    let assignment_id = Uuid::new_v4();
    let email = format!("bddtest_authz_{user_id}@example.com");
    let now = Utc::now();

    // Fixture: user (FK target, RLS-scoped) + role assignment
    sesame_idam_database::with_pre_auth_tenant(TEST_TENANT, |exec| {
        exec.execute_values(
            "INSERT INTO sesame_idam.users \
             (id, email, password_hash, tenant_id, status, email_verified, phone, phone_verified, created_at, updated_at) \
             VALUES ($1, $2, 'x', $3, 'active', true, NULL, false, $4, $4)",
            &sea_query::Values(vec![
                user_id.into(),
                email.into(),
                TEST_TENANT.into(),
                now.into(),
            ]),
        )?;
        exec.execute_values(
            "INSERT INTO sesame_idam.role_assignments \
             (id, principal_id, role_name, resource_type, resource_id, tenant_id, created_at, updated_at) \
             VALUES ($1, $2, 'OWNER', 'application', NULL, $3, $4, $4)",
            &sea_query::Values(vec![
                assignment_id.into(),
                user_id.into(),
                TEST_TENANT.into(),
                now.into(),
            ]),
        )
    })
    .expect("insert fixtures");

    // When — same tenant
    let response = handle(make_request(&user_id.to_string(), TEST_TENANT));

    // Then — OWNER is present
    let roles: Vec<String> = response
        .roles
        .iter()
        .filter_map(|r| r.get("role").and_then(|v| v.as_str()).map(String::from))
        .collect();
    assert_eq!(roles, vec!["OWNER".to_string()], "seeded role must resolve");
    assert_eq!(response.user_id, user_id.to_string());

    // Tenant isolation: same principal queried under another tenant → empty
    let cross = handle(make_request(&user_id.to_string(), "other-tenant"));
    assert!(
        cross.roles.is_empty(),
        "role assignments must not leak across tenants"
    );

    // Cleanup (role assignment cascades from user delete)
    sesame_idam_database::with_pre_auth_tenant(TEST_TENANT, |exec| {
        exec.execute_values(
            "DELETE FROM sesame_idam.users WHERE id = $1",
            &sea_query::Values(vec![user_id.into()]),
        )
    })
    .expect("cleanup");
}

/// Scenario: OWNER role resolves seeded permissions for hauliage portal scope.
///
/// Given app_role_permissions rows for tenant `hauliage` and app `frontend`
/// When POST /authz/principals/effective for an OWNER principal
/// Then permissions include organization:read (JWT sx.permissions path).
#[test]
fn owner_role_resolves_seeded_permissions() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    const HAULIAGE_TENANT: &str = "hauliage";
    let user_id = Uuid::parse_str("a1000001-0001-4000-8000-000000000001")
        .expect("demo owner user id");

    let response = handle(make_request_with_app(
        &user_id.to_string(),
        HAULIAGE_TENANT,
        HAULIAGE_PORTAL_APP,
    ));

    let roles: Vec<String> = response
        .roles
        .iter()
        .filter_map(|r| r.get("role").and_then(|v| v.as_str()).map(String::from))
        .collect();
    assert!(
        roles.contains(&"OWNER".to_string()),
        "demo owner must have OWNER role assignment"
    );
    assert!(
        response.permissions.contains(&"organization:read".to_string()),
        "OWNER must resolve organization:read from app_role_permissions seed; got {:?}",
        response.permissions
    );
    assert!(
        response.permissions.contains(&"org:manage".to_string()),
        "OWNER must resolve org:manage; got {:?}",
        response.permissions
    );

    // Cross-app scope: same principal with unrelated app_id → no permissions
    let other_app = handle(make_request_with_app(
        &user_id.to_string(),
        HAULIAGE_TENANT,
        TEST_APP,
    ));
    assert!(
        other_app.permissions.is_empty(),
        "permissions must be app-scoped"
    );
}

/// Scenario: Unknown principal gets empty roles (not an error).
#[test]
fn unknown_principal_returns_empty_roles() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let response = handle(make_request(&Uuid::new_v4().to_string(), TEST_TENANT));
    assert!(response.roles.is_empty());
    assert!(response.permissions.is_empty());
}

/// Scenario: Non-UUID user id is handled without touching the database.
#[test]
fn non_uuid_user_id_returns_empty() {
    // Deliberately no db_available() gate: the controller must return before
    // any database access for a non-UUID principal.
    let response = handle(make_request("not-a-uuid", TEST_TENANT));
    assert!(response.roles.is_empty());
    assert_eq!(response.user_id, "not-a-uuid");
}
