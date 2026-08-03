//! Social OAuth start-flow BDD (pre-OIDC north handlers).
//!
//! Covers fail-closed paths without a live IdP: unsupported provider,
//! missing redirect, and tenant without OAuth config.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::social_login::{self, SocialLoginOutcome};
use sesame_idam_identity_login_service_gen::handlers::social_login::Request as SocialRequest;

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
    });
    true
}

fn social_request(
    tenant: &str,
    provider: &str,
    redirect_uri: &str,
) -> TypedHandlerRequest<SocialRequest> {
    TypedHandlerRequest {
        method: Method::GET,
        path: format!("/auth/social/{provider}/login"),
        handler_name: "social_login".to_string(),
        path_params: std::collections::HashMap::from([(
            "provider".to_string(),
            provider.to_string(),
        )]),
        query_params: std::collections::HashMap::new(),
        data: SocialRequest {
            x_tenant_id: tenant.to_string(),
            provider: provider.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scope: None,
        },
        jwt_claims: None,
    }
}

fn error_body(outcome: SocialLoginOutcome) -> (u16, serde_json::Value) {
    match outcome {
        SocialLoginOutcome::Error(json) => (json.status, json.body),
        SocialLoginOutcome::Redirect(_) => panic!("expected error outcome, got redirect"),
    }
}

#[test]
fn social_login_rejects_unsupported_provider() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    ensure_active_tenant(FIXTURE_TENANT);
    let (status, body) = error_body(social_login::handle(social_request(
        FIXTURE_TENANT,
        "not-a-provider",
        "https://app.example/callback",
    )));
    assert_eq!(status, 400);
    assert_eq!(body["error"], "unsupported_provider");
}

#[test]
fn social_login_rejects_empty_redirect() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    ensure_active_tenant(FIXTURE_TENANT);
    let (status, body) = error_body(social_login::handle(social_request(
        FIXTURE_TENANT,
        "google",
        "",
    )));
    assert_eq!(status, 400);
    assert_eq!(body["error"], "redirect_uri_required");
}

#[test]
fn social_login_unconfigured_provider_is_unavailable() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    let tenant = format!("social-{}", uuid::Uuid::new_v4().simple());
    ensure_active_tenant(&tenant);
    let (status, body) = error_body(social_login::handle(social_request(
        &tenant,
        "google",
        "https://app.example/callback",
    )));
    assert_eq!(status, 503);
    assert_eq!(body["error"], "oauth_not_configured");
}
