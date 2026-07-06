//! Live-database BDD tests for the admin user lifecycle: create (idempotent),
//! fetch by email, disable, enable.
//!
//! Skips gracefully when Postgres is unreachable so `just nt` passes without
//! the cluster.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use uuid::Uuid;

use sesame_idam_identity_user_mgmt_service::controllers::{
    create_user, disable_user, enable_user, fetch_user_by_email,
};
use sesame_idam_identity_user_mgmt_service_gen::handlers::create_user::Request as CreateRequest;
use sesame_idam_identity_user_mgmt_service_gen::handlers::disable_user::Request as DisableRequest;
use sesame_idam_identity_user_mgmt_service_gen::handlers::enable_user::Request as EnableRequest;
use sesame_idam_identity_user_mgmt_service_gen::handlers::fetch_user_by_email::Request as FetchRequest;

const TEST_TENANT: &str = "bdd-usermgmt-tenant";

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
    let pass = std::env::var("TEST_DB_PASS")
        .unwrap_or_else(|_| "dev_password_change_in_prod".to_string());
    let db = std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "sesame_idam".to_string());
    may_postgres::connect(&format!("postgres://{user}:{pass}@{host}:{port}/{db}"))
        .expect("connect test DB")
}

fn typed<T>(handler_name: &str, data: T) -> TypedHandlerRequest<T> {
    TypedHandlerRequest {
        method: Method::POST,
        path: format!("/admin/users/{handler_name}"),
        handler_name: handler_name.to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data,
    }
}

fn create_request(email: &str) -> TypedHandlerRequest<CreateRequest> {
    typed(
        "create_user",
        CreateRequest {
            email: email.to_string(),
            email_confirmed: Some(true),
            extra_properties: None,
            first_name: None,
            last_name: None,
            org_id: None,
            picture_url: None,
            send_email_confirmation: None,
            send_welcome_email: None,
            username: None,
            x_tenant_id: TEST_TENANT.to_string(),
        },
    )
}

fn cleanup_email(client: &may_postgres::Client, email: &str) {
    client
        .batch_execute(&format!(
            "DELETE FROM sesame_idam.users WHERE email = '{email}' AND tenant_id = '{TEST_TENANT}';"
        ))
        .expect("cleanup");
}

/// Scenario: Create → fetch by email → disable → enable lifecycle.
#[test]
fn admin_user_lifecycle() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let client = raw_client();
    let email = format!("bddtest_admin_{}@example.com", Uuid::new_v4());

    // Create — 201, no password yet
    let resp = create_user::handle(create_request(&email));
    assert_eq!(resp.status, 201, "create: {}", resp.body);
    assert_eq!(resp.body["email"], email);
    assert_eq!(resp.body["enabled"], true);
    assert_eq!(resp.body["has_password"], false);
    assert_eq!(resp.body["email_confirmed"], true);
    let user_id = resp.body["user_id"].as_str().expect("user_id").to_string();

    // Idempotent second create — 200, same user
    let resp = create_user::handle(create_request(&email));
    assert_eq!(resp.status, 200, "idempotent create must be 200");
    assert_eq!(resp.body["user_id"], user_id.as_str());

    // Fetch by email
    let resp = fetch_user_by_email::handle(typed(
        "fetch_user_by_email",
        FetchRequest {
            email: email.clone(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
    ));
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body["user_id"], user_id.as_str());

    // Disable
    let resp = disable_user::handle(typed(
        "disable_user",
        DisableRequest {
            user_id: user_id.clone(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
    ));
    assert_eq!(resp.status, 200, "disable: {}", resp.body);
    assert_eq!(resp.body["enabled"], false);
    assert_eq!(resp.body["locked"], true);

    // Enable again
    let resp = enable_user::handle(typed(
        "enable_user",
        EnableRequest {
            user_id: user_id.clone(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
    ));
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body["enabled"], true);

    cleanup_email(&client, &email);
}

/// Scenario: Fetch by email is tenant-scoped — other tenants get 404.
#[test]
fn fetch_by_email_is_tenant_scoped() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let client = raw_client();
    let email = format!("bddtest_admin_{}@example.com", Uuid::new_v4());

    let resp = create_user::handle(create_request(&email));
    assert_eq!(resp.status, 201);

    let resp = fetch_user_by_email::handle(typed(
        "fetch_user_by_email",
        FetchRequest {
            email: email.clone(),
            x_tenant_id: "some-other-tenant".to_string(),
        },
    ));
    assert_eq!(resp.status, 404, "users must not resolve across tenants");

    cleanup_email(&client, &email);
}

/// Scenario: Disabling an unknown user is a 404; bad uuid is a 400.
#[test]
fn disable_unknown_and_invalid_ids() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let resp = disable_user::handle(typed(
        "disable_user",
        DisableRequest {
            user_id: Uuid::new_v4().to_string(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
    ));
    assert_eq!(resp.status, 404);

    let resp = disable_user::handle(typed(
        "disable_user",
        DisableRequest {
            user_id: "not-a-uuid".to_string(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
    ));
    assert_eq!(resp.status, 400);
}

/// Scenario: Create with an invalid email is rejected.
#[test]
fn create_invalid_email_rejected() {
    let resp = create_user::handle(create_request("not-an-email"));
    assert_eq!(resp.status, 400);
}
