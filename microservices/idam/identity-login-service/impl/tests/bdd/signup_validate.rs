//! Signup validation BDD (D3): `GET /auth/signup/validate` availability pre-check.
//!
//! Tenant is bound via registered `client_id`. Pure-validation cases
//! (empty/invalid email) run without infra; email-taken needs Postgres.

use http::Method;

use brrtrouter::typed::TypedHandlerRequest;
use sesame_idam_identity_login_service::controllers::{auth_register, signup_validate};
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterRequest;
use sesame_idam_identity_login_service_gen::handlers::signup_validate::{
    Request as ValidateRequest, Response as ValidateResponse,
};

use super::token_lifecycle::{infra_available, unique_email};

use crate::common::{ensure_active_tenant, ensure_public_login_client};

const TEST_TENANT: &str = "bdd-signup-validate-tenant";

fn validate_request(client_id: &str, email: Option<&str>) -> TypedHandlerRequest<ValidateRequest> {
    TypedHandlerRequest {
        method: Method::GET,
        path: "/auth/signup/validate".to_string(),
        handler_name: "signup_validate".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: ValidateRequest {
            client_id: client_id.to_string(),
            x_tenant_id: None,
            email: email.map(str::to_string),
            phone: None,
        },
        jwt_claims: None,
    }
}

fn register_request(
    client_id: &str,
    email: &str,
    password: &str,
) -> TypedHandlerRequest<RegisterRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/register".to_string(),
        handler_name: "auth_register".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RegisterRequest {
            client_id: client_id.to_string(),
            email: email.to_string(),
            first_name: Some("Sign".to_string()),
            last_name: Some("Up".to_string()),
            password: password.to_string(),
            phone: None,
            username: None,
            x_tenant_id: None,
        },
        jwt_claims: None,
    }
}

fn reasons(resp: &ValidateResponse) -> Vec<String> {
    resp.reasons.clone().unwrap_or_default()
}

/// Scenario: an empty email is rejected without touching the database.
#[test]
fn signup_validate_requires_email() {
    let resp = signup_validate::handle(validate_request("any-client", None));
    assert!(!resp.allowed);
    assert!(reasons(&resp).contains(&"email_required".to_string()));
}

/// Scenario: a malformed email is rejected without touching the database.
#[test]
fn signup_validate_rejects_malformed_email() {
    for bad in ["not-an-email", "no@domain", "@example.com", "a@b."] {
        let resp = signup_validate::handle(validate_request("any-client", Some(bad)));
        assert!(!resp.allowed, "{bad} should be rejected");
        assert!(
            reasons(&resp).contains(&"email_invalid".to_string()),
            "{bad}: {:?}",
            reasons(&resp)
        );
    }
}

/// Scenario: unknown client_id is rejected before the availability check.
#[test]
fn signup_validate_rejects_unknown_client() {
    if !infra_available() {
        println!("SKIP: Postgres and/or Redis not available");
        return;
    }
    let resp = signup_validate::handle(validate_request(
        "not-a-registered-client",
        Some("fresh@example.com"),
    ));
    assert!(!resp.allowed);
    assert!(reasons(&resp).contains(&"client_invalid".to_string()));
}

/// Scenario: a fresh, well-formed email is allowed.
#[test]
fn signup_validate_allows_fresh_email() {
    if !infra_available() {
        println!("SKIP: Postgres and/or Redis not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);
    let client_id = ensure_public_login_client(TEST_TENANT);
    let resp = signup_validate::handle(validate_request(
        &client_id,
        Some(&unique_email("fresh")),
    ));
    assert!(
        resp.allowed,
        "fresh email should be allowed: {:?}",
        reasons(&resp)
    );
    assert!(reasons(&resp).is_empty());
}

/// Scenario: an already-registered email reports `email_taken`.
#[test]
fn signup_validate_flags_taken_email() {
    if !infra_available() {
        println!("SKIP: Postgres and/or Redis not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);
    let client_id = ensure_public_login_client(TEST_TENANT);
    let email = unique_email("taken");
    let reg = auth_register::handle(register_request(&client_id, &email, "SecureP@ss123!"));
    assert_eq!(reg.status, 201, "register: {:?}", reg.body);

    let resp = signup_validate::handle(validate_request(&client_id, Some(&email)));
    assert!(!resp.allowed, "taken email must not be allowed");
    assert!(reasons(&resp).contains(&"email_taken".to_string()));
}
