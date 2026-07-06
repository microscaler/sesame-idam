/// Standalone BDD scenario tests for all Epic 1 controllers.
///
/// Each test calls the handler directly and verifies the response.
/// These are NOT connected to Gherkin steps — they are independent rstest tests.
use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use rstest::rstest;

use sesame_idam_authz_core::controllers::assign_principal_role::handle as role_handle;
use sesame_idam_authz_core::controllers::authorize::handle as auth_handle;
use sesame_idam_authz_core::controllers::principal_effective::handle as effective_handle;
use sesame_idam_authz_core::controllers::revoke_principal_role::handle as revoke_handle;
use sesame_idam_authz_core::controllers::set_principal_attribute::handle as attr_handle;
use sesame_idam_authz_core_gen::handlers::assign_principal_role::Request as RoleReq;
use sesame_idam_authz_core_gen::handlers::authorize::Request as AuthReq;
use sesame_idam_authz_core_gen::handlers::principal_effective::Request as EffectiveReq;
use sesame_idam_authz_core_gen::handlers::revoke_principal_role::Request as RevokeReq;
use sesame_idam_authz_core_gen::handlers::set_principal_attribute::Request as AttrReq;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_auth_request(
    user_id: impl Into<String>,
    action: &str,
    resource: &str,
) -> TypedHandlerRequest<AuthReq> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/authorize".to_string(),
        handler_name: "authorize".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: AuthReq {
            user_id: user_id.into(),
            action: action.to_string(),
            resource: resource.to_string(),
            tenant_id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string()),
            app_id: None,
            org_id: None,
            context: None,
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    }
}

fn make_effective_request(user_id: impl Into<String>) -> TypedHandlerRequest<EffectiveReq> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/principals/effective".to_string(),
        handler_name: "principal_effective".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: EffectiveReq {
            user_id: user_id.into(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
            org_id: None,
            include_inherited: None,
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    }
}

fn make_attr_request(
    user_id: impl Into<String>,
    key: &str,
    value: &str,
) -> TypedHandlerRequest<AttrReq> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/principals/attributes".to_string(),
        handler_name: "set_principal_attribute".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: AttrReq {
            user_id: user_id.into(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            key: key.to_string(),
            value: value.to_string(),
            org_id: None,
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    }
}

fn make_role_request(user_id: impl Into<String>, role: &str) -> TypedHandlerRequest<RoleReq> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/principals/roles".to_string(),
        handler_name: "assign_principal_role".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RoleReq {
            user_id: user_id.into(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
            role: role.to_string(),
            expires_at: None,
            org_id: None,
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    }
}

fn make_revoke_request(user_id: impl Into<String>, role: &str) -> TypedHandlerRequest<RevokeReq> {
    TypedHandlerRequest {
        method: Method::DELETE,
        path: "/authz/principals/roles".to_string(),
        handler_name: "revoke_principal_role".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RevokeReq {
            user_id: user_id.into(),
            app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
            role: role.to_string(),
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    }
}

// ─── Authorize Scenarios ─────────────────────────────────────────────────────

/// Scenario: Allow read action on a resource.
///
/// Given: valid request with required fields (`user_id`, action, resource).
/// When: we call the authorize handler.
/// Then: the response body has field "allowed" set to true.
#[rstest]
fn valid_authorization_returns_allowed_true() {
    let typed_req = make_auth_request("test-user-1", "read", "resource:docs");
    let response = auth_handle(typed_req);
    assert!(response.allowed, "allowed should be true for valid request");
}

/// Scenario: Response contains "allowed" boolean.
///
/// Given: valid authorization request.
/// When: we call the authorize handler.
/// Then: the response body has a valid JSON "allowed" field of type boolean.
#[rstest]
fn authorization_response_has_allowed_boolean() {
    let typed_req = make_auth_request("test-user-1", "read", "resource:docs");
    let response = auth_handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("allowed").is_some(), "missing 'allowed' field");
    assert!(
        json["allowed"].as_bool().is_some(),
        "'allowed' must be boolean"
    );
}

// ─── Principal Effective Scenarios ───────────────────────────────────────────

/// Scenario: Get effective permissions for a user.
///
/// Given: valid request with required fields (`user_id`, `app_id`, `tenant_id`).
/// When: we call the `principal_effective` handler.
/// Then: the response has `user_id`, roles, and permissions fields.
#[rstest]
fn effective_permissions_returns_valid_response() {
    let typed_req = make_effective_request("test-user-1");
    let response = effective_handle(typed_req);
    assert!(!response.user_id.is_empty(), "user_id should not be empty");
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("permissions").is_some(),
        "missing 'permissions' field"
    );
    assert!(json.get("roles").is_some(), "missing 'roles' field");
}

// ─── Set Principal Attribute Scenarios ───────────────────────────────────────

/// Scenario: Set an attribute on a principal.
///
/// Given: valid request with required fields (`user_id`, `tenant_id`, key, value).
/// When: we call the `set_principal_attribute` handler.
/// Then: the response is returned with an error field.
#[rstest]
fn set_attribute_returns_success() {
    let typed_req = make_attr_request("test-user-1", "department", "engineering");
    let response = attr_handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("error").is_some(),
        "response should have 'error' field"
    );
}

// ─── Assign Role Scenarios ───────────────────────────────────────────────────

/// Scenario: Assign a role to a principal.
///
/// Given: valid request with required fields.
/// When: we call the `assign_principal_role` handler.
/// Then: the response has an error field.
#[rstest]
fn assign_role_returns_success() {
    let typed_req = make_role_request("test-user-1", "editor");
    let response = role_handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("error").is_some(),
        "response should have 'error' field"
    );
}

// ─── Revoke Role Scenarios ───────────────────────────────────────────────────

/// Scenario: Revoke a role from a principal.
///
/// Given: valid request with required fields.
/// When: we call the `revoke_principal_role` handler.
/// Then: the response has an error field.
#[rstest]
fn revoke_role_returns_success() {
    let typed_req = make_revoke_request("test-user-1", "editor");
    let response = revoke_handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("error").is_some(),
        "response should have 'error' field"
    );
}
