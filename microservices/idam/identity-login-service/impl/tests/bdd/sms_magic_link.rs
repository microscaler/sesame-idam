//! SMS magic-link send BDD — abuse gate + generic success (provider not wired).

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::sms_magic_link_send;
use sesame_idam_identity_login_service_gen::handlers::sms_magic_link_send::Request as SmsMagicRequest;

use crate::common::{ensure_active_tenant, FIXTURE_TENANT};

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
            std::env::var("TEST_DB_PASS")
                .unwrap_or_else(|_| "dev_password_change_in_prod".to_string()),
        );
        std::env::set_var(
            "DB_NAME",
            std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
        // Prefer Redis for abuse guard; tests still assert generic success.
        if std::env::var("REDIS_URL").is_err() {
            std::env::set_var("REDIS_URL", "redis://127.0.0.1:6379");
        }
    });
    true
}

fn send_request(tenant: &str, phone: &str) -> TypedHandlerRequest<SmsMagicRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login/phone-magic-link".to_string(),
        handler_name: "sms_magic_link_send".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: SmsMagicRequest {
            phone: phone.to_string(),
            x_tenant_id: Some(tenant.to_string()),
        },
        jwt_claims: None,
    }
}

#[test]
fn sms_magic_link_send_unknown_tenant_rejected() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    let resp = sms_magic_link_send::handle(send_request(
        "totally-unprovisioned-tenant-slug",
        "+15555550100",
    ));
    assert_eq!(resp.status, 404);
    assert_eq!(resp.body["error"], "tenant_unknown");
}

#[test]
fn sms_magic_link_send_returns_generic_success() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    ensure_active_tenant(FIXTURE_TENANT);
    let resp = sms_magic_link_send::handle(send_request(FIXTURE_TENANT, "+15555550123"));
    assert_eq!(resp.status, 200, "sms magic send: {:?}", resp.body);
    assert_eq!(resp.body["success"], true);
}
