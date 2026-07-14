//! Signup validation BDD (D3): `GET /auth/signup/validate` availability pre-check.
//!
//! Pure-validation cases (empty/invalid email) run without infra; the
//! email-taken case needs Postgres and skips otherwise.
//!
//! ```bash
//! ssh ms02 'source ~/.cargo/env && cd ~/Workspace/microscaler/seasame-idam/microservices && \
//!   cargo test -p sesame_idam_identity_login_service --test main_bdd signup_validate -- --nocapture'
//! ```

use http::Method;

use brrtrouter::typed::TypedHandlerRequest;
use sesame_idam_identity_login_service::controllers::{auth_register, signup_validate};
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterRequest;
use sesame_idam_identity_login_service_gen::handlers::signup_validate::{
    Request as ValidateRequest, Response as ValidateResponse,
};

use super::token_lifecycle::{infra_available, unique_email};

use crate::common::ensure_active_tenant;

const TEST_TENANT: &str = "bdd-signup-validate-tenant";

fn validate_request(email: Option<&str>) -> TypedHandlerRequest<ValidateRequest> {
    TypedHandlerRequest {
        method: Method::GET,
        path: "/auth/signup/validate".to_string(),
        handler_name: "signup_validate".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: ValidateRequest {
            x_tenant_id: TEST_TENANT.to_string(),
            email: email.map(str::to_string),
            phone: None,
        },
        jwt_claims: None,
    }
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
            first_name: Some("Sign".to_string()),
            last_name: Some("Up".to_string()),
            password: password.to_string(),
            phone: None,
            username: None,
            x_tenant_id: TEST_TENANT.to_string(),
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
    let resp = signup_validate::handle(validate_request(None));
    assert!(!resp.allowed);
    assert!(reasons(&resp).contains(&"email_required".to_string()));
}

/// Scenario: a malformed email is rejected without touching the database.
#[test]
fn signup_validate_rejects_malformed_email() {
    for bad in ["not-an-email", "no@domain", "@example.com", "a@b."] {
        let resp = signup_validate::handle(validate_request(Some(bad)));
        assert!(!resp.allowed, "{bad} should be rejected");
        assert!(
            reasons(&resp).contains(&"email_invalid".to_string()),
            "{bad}: {:?}",
            reasons(&resp)
        );
    }
}

/// Scenario: a fresh, well-formed email is allowed.
#[test]
fn signup_validate_allows_fresh_email() {
    if !infra_available() {
        println!("SKIP: Postgres and/or Redis not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);
    let resp = signup_validate::handle(validate_request(Some(&unique_email("fresh"))));
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
    let email = unique_email("taken");
    let reg = auth_register::handle(register_request(&email, "SecureP@ss123!"));
    assert_eq!(reg.status, 201, "register: {:?}", reg.body);

    let resp = signup_validate::handle(validate_request(Some(&email)));
    assert!(!resp.allowed, "taken email must not be allowed");
    assert!(reasons(&resp).contains(&"email_taken".to_string()));
}
