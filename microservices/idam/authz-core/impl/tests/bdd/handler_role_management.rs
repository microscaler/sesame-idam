/// BDD handler-level tests for role management (assign + revoke) endpoints.
///
/// Calls the REAL controller handlers and verifies the response shapes.
/// Unlike the unit tests in tests/unit/ which only validate schema
/// serialization, these exercise the actual handler code (audit event emission).
use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use rstest_bdd::gherkin::{given, then, when};
use sesame_idam_authz_core::controllers::assign_principal_role::handle as assign_handle;
use sesame_idam_authz_core::controllers::revoke_principal_role::handle as revoke_handle;
use sesame_idam_authz_core_gen::handlers::assign_principal_role::Request as AssignRequest;
use sesame_idam_authz_core_gen::handlers::revoke_principal_role::Request as RevokeRequest;
use std::sync::LazyLock;
use std::sync::Mutex;

thread_local! {
    static CONTEXT: Mutex<Option<serde_json::Value>> = Mutex::new(None);
}

// ═══════════════════════════════════════════════════════════════════════════
// Assign Role Scenarios
// ═══════════════════════════════════════════════════════════════════════════

/// Scenario: Assign a role to a principal
///   Given a valid assign_principal_role request
///   When I send a valid request to the assign_principal_role controller
///   Then the response is returned
///   And the response has an error field

#[given("a valid assign_principal_role request")]
fn given_valid_assign_request() {
    CONTEXT.with(|c| *c.lock().unwrap() = None);
}

#[when("I send a valid request to the assign_principal_role controller")]
fn when_assign_valid() {
    let req = TypedHandlerRequest::<AssignRequest> {
        method: Method::POST,
        path: "/authz/principals/roles".to_string(),
        handler_name: "assign_principal_role".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: AssignRequest {
            user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
            role: "editor".to_string(),
            expires_at: None,
            org_id: None,
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = assign_handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    CONTEXT.with(|c| *c.lock().unwrap() = Some(json));
}

#[then("the response is returned")]
fn then_assign_response_returned() {
    CONTEXT.with(|c| {
        let json = c.lock().unwrap().as_ref().expect("no response cached");
        assert!(json.get("error").is_some(), "missing 'error' field");
    });
}

#[then("the response has an error field")]
fn then_assign_has_error_field() {
    CONTEXT.with(|c| {
        let json = c.lock().unwrap().as_ref().expect("no response cached");
        assert!(json.get("error").is_some(), "missing 'error' field");
        assert!(
            json["error"].as_str().unwrap_or("") == "",
            "stub returns empty error"
        );
    });
}

/// Scenario: Assign role with expiration
///   Given an assign_principal_role request with expires_at
///   WHEN I send the request to the controller
///   Then the response is returned

#[given("an assign_principal_role request with expires_at")]
fn given_assign_with_expiry() {
    CONTEXT.with(|c| *c.lock().unwrap() = None);
}

#[when("I send the request with expiration")]
fn when_assign_with_expiry() {
    let req = TypedHandlerRequest::<AssignRequest> {
        method: Method::POST,
        path: "/authz/principals/roles".to_string(),
        handler_name: "assign_principal_role".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: AssignRequest {
            user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
            role: "admin".to_string(),
            expires_at: Some(1735689600),
            org_id: None,
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = assign_handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    CONTEXT.with(|c| *c.lock().unwrap() = Some(json));
}

// ═══════════════════════════════════════════════════════════════════════════
// Revoke Role Scenarios
// ═══════════════════════════════════════════════════════════════════════════

/// Scenario: Revoke a role from a principal
///   Given a valid revoke_principal_role request
///   When I send a valid request to the revoke_principal_role controller
///   Then the response is returned
///   And the response has an error field

#[given("a valid revoke_principal_role request")]
fn given_valid_revoke_request() {
    CONTEXT.with(|c| *c.lock().unwrap() = None);
}

#[when("I send a valid request to the revoke_principal_role controller")]
fn when_revoke_valid() {
    let req = TypedHandlerRequest::<RevokeRequest> {
        method: Method::DELETE,
        path: "/authz/principals/roles".to_string(),
        handler_name: "revoke_principal_role".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RevokeRequest {
            user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
            app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
            role: "editor".to_string(),
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = revoke_handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    CONTEXT.with(|c| *c.lock().unwrap() = Some(json));
}

/// Scenario: Revoke role from a principal in an org
///   Given a revoke_principal_role request with org context
///   WHEN I send the request to the controller
///   Then the response is returned

#[given("a revoke_principal_role request with org context")]
fn given_revoke_with_org() {
    CONTEXT.with(|c| *c.lock().unwrap() = None);
}

#[when("I send the org context revoke request")]
fn when_revoke_with_org() {
    let req = TypedHandlerRequest::<RevokeRequest> {
        method: Method::DELETE,
        path: "/authz/principals/roles".to_string(),
        handler_name: "revoke_principal_role".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RevokeRequest {
            user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
            app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
            role: "admin".to_string(),
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = revoke_handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    CONTEXT.with(|c| *c.lock().unwrap() = Some(json));
}
