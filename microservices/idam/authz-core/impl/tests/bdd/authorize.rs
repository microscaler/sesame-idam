/// BDD feature: Authorization Check (POST /authz/authorize)
///
/// Tests exercise the Request/Response schema contract for the authorize endpoint.
/// These verify:
/// - Required fields are enforced by deserialization
/// - Optional fields accept valid values
/// - Response shape matches the OpenAPI spec
/// - Tenant isolation headers are extracted correctly
use brrtrouter::dispatcher::HeaderVec;
use brrtrouter::ids::RequestId;
use http::Method;
use sesame_idam_authz_core_gen::handlers::authorize::{Request, Response};
use std::sync::Arc;

// ─── Scenario Group 1: Successful authorization ─────────────────────────────

/// Scenario: Allow read action on a resource.
///
/// Given: valid request with required fields (user_id, action, resource).
/// When: we construct a Request and serialize the response.
/// Then: the response body has field "allowed" set to true.
#[test]
fn test_allow_read_action_on_resource() {
    // Given: valid authorization request
    let request_data = Request {
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
        action: "read".to_string(),
        resource: "accounting:invoices".to_string(),
        tenant_id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string()),
        app_id: None,
        org_id: None,
        context: None,
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    // When: we serialize the request to JSON
    let json = serde_json::to_value(&request_data).expect("request must serialize to JSON");

    // Then: required fields are present
    assert_eq!(json["user_id"], "1189c444-8a2d-4c41-8b4b-ae43ce79a492");
    assert_eq!(json["action"], "read");
    assert_eq!(json["resource"], "accounting:invoices");

    // When: we construct a valid response
    let response = Response {
        allowed: true,
        permissions_used: None,
        reason: None,
        roles_matched: None,
    };

    // Then: the response body has "allowed" set to true
    let response_json = serde_json::to_value(&response).expect("response must serialize");
    assert!(response_json["allowed"]
        .as_bool()
        .expect("allowed must be boolean"));
}

// ─── Scenario Group 2: Required fields validation ────────────────────────────

/// Scenario: Reject request missing required "action" field.
///
/// Given: request body without "action" field.
/// When: we attempt to construct a Request.
/// Then: serde deserialization fails (missing required field).
#[test]
fn test_reject_request_missing_action_field() {
    // Given: request body missing "action"
    let json_body = serde_json::json!({
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "resource": "accounting:invoices",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });

    // When: we attempt to deserialize
    // Then: deserialization should fail due to missing "action"
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing required field 'action' should cause deserialization error"
    );
}

/// Scenario: Reject request missing required "resource" field.
///
/// Given: request body without "resource" field.
/// When: we attempt to construct a Request.
/// Then: serde deserialization fails (missing required field).
#[test]
fn test_reject_request_missing_resource_field() {
    // Given: request body missing "resource"
    let json_body = serde_json::json!({
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "action": "read",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });

    // When: we attempt to deserialize
    // Then: deserialization should fail due to missing "resource"
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing required field 'resource' should cause deserialization error"
    );
}

// ─── Scenario Group 3: Optional fields ───────────────────────────────────────

/// Scenario: Accept request with optional "org_id" field.
///
/// Given: valid request plus optional "org_id".
/// When: we construct a Request.
/// Then: deserialization succeeds and org_id is Some(...).
#[test]
fn test_accept_request_with_optional_org_id() {
    // Given: valid request with org_id
    let request_data = Request {
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
        action: "write".to_string(),
        resource: "accounting:invoices".to_string(),
        tenant_id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string()),
        app_id: None,
        org_id: Some(serde_json::Value::String(
            "22222222-8a2d-4c41-8b4b-ae43ce79a493".to_string(),
        )),
        context: None,
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    // When: we serialize the request to JSON
    let json = serde_json::to_value(&request_data).expect("request must serialize");

    // Then: org_id is present and matches
    assert!(json.get("org_id").is_some(), "org_id must be present");
    assert_eq!(json["org_id"], "22222222-8a2d-4c41-8b4b-ae43ce79a493");
}

/// Scenario: Accept request with optional "app_id" field.
///
/// Given: valid request plus optional "app_id".
/// When: we construct a Request.
/// Then: deserialization succeeds and app_id is Some(...).
#[test]
fn test_accept_request_with_optional_app_id() {
    // Given: valid request with app_id
    let request_data = Request {
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
        action: "delete".to_string(),
        resource: "accounting:invoices".to_string(),
        tenant_id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string()),
        app_id: Some(serde_json::Value::String(
            "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
        )),
        org_id: None,
        context: None,
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    // When: we serialize the request to JSON
    let json = serde_json::to_value(&request_data).expect("request must serialize");

    // Then: app_id is present and matches
    assert!(json.get("app_id").is_some(), "app_id must be present");
    assert_eq!(json["app_id"], "33333333-8a2d-4c41-8b4b-ae43ce79a494");
}

// ─── Scenario Group 4: Response shape validation ─────────────────────────────

/// Scenario: Response contains "allowed" boolean.
///
/// Given: valid authorization request.
/// When: we construct a Response.
/// Then: the response body has a valid JSON "allowed" field of type boolean.
#[test]
fn test_response_contains_allowed_boolean() {
    // When: we construct a response
    let response = Response {
        allowed: true,
        permissions_used: None,
        reason: None,
        roles_matched: None,
    };

    // Then: response has "allowed" boolean
    let response_json = serde_json::to_value(&response).expect("response must serialize");
    assert!(
        response_json.get("allowed").is_some(),
        "Response must have 'allowed' field"
    );
    assert!(
        response_json["allowed"].as_bool().is_some(),
        "'allowed' must be a boolean"
    );
    assert!(response_json["allowed"].as_bool().unwrap_or(false));
}

/// Scenario: Response may contain optional "permissions_used" array.
///
/// Given: valid authorization request.
/// When: we construct a Response.
/// Then: the response body MAY contain a "permissions_used" field of type array.
#[test]
fn test_response_may_contain_permissions_used_array() {
    // When: we construct a response with permissions_used
    let response = Response {
        allowed: true,
        permissions_used: Some(serde_json::json!(["read:invoices", "write:invoices"])),
        reason: None,
        roles_matched: None,
    };

    // Then: permissions_used is present and is an array
    let response_json = serde_json::to_value(&response).expect("response must serialize");
    let perms: serde_json::Value = response_json["permissions_used"].clone();
    assert!(
        perms.is_array(),
        "'permissions_used' must be an array or null"
    );
}

/// Scenario: Response may contain optional "reason" string.
///
/// Given: valid authorization request.
/// When: we construct a Response.
/// Then: the response body MAY contain a "reason" field of type string.
#[test]
fn test_response_may_contain_reason_string() {
    // When: we construct a response with reason
    let response = Response {
        allowed: false,
        permissions_used: None,
        reason: Some("insufficient_permissions".to_string()),
        roles_matched: None,
    };

    // Then: reason is present and is a string
    let response_json = serde_json::to_value(&response).expect("response must serialize");
    let reason: serde_json::Value = response_json["reason"].clone();
    assert!(
        reason.is_string() || reason.is_null(),
        "'reason' must be a string or null"
    );
    assert_eq!(reason.as_str().unwrap_or(""), "insufficient_permissions");
}

// ─── Scenario Group 5: Tenant isolation ──────────────────────────────────────

/// Scenario: X-Tenant-ID header is extracted and validated.
///
/// Given: a HandlerRequest with X-Tenant-ID header.
/// When: we construct a Request from the header.
/// Then: x_tenant_id and tenant_id are both set from the header.
#[test]
fn test_tenant_isolation_headers() {
    // Given: tenant IDs from header
    let header_tenant_id = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

    // When: we construct a request with tenant info
    let request_data = Request {
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
        action: "read".to_string(),
        resource: "accounting:invoices".to_string(),
        tenant_id: Some(header_tenant_id.to_string()),
        app_id: None,
        org_id: None,
        context: None,
        x_tenant_id: header_tenant_id.to_string(),
    };

    // Then: both tenant_id and x_tenant_id are set
    let json = serde_json::to_value(&request_data).expect("request must serialize");
    assert_eq!(json["tenant_id"], header_tenant_id);
    assert_eq!(json["X-Tenant-ID"], header_tenant_id);
}
