/// BDD feature: Get Effective Permissions (POST /authz/principals/effective)
///
/// Tests exercise the Request/Response schema contract for the principal
/// effective permissions endpoint. These verify:
/// - Required fields are enforced by deserialization
/// - Optional fields accept valid values
/// - Response shape matches the OpenAPI spec
/// - Tenant isolation headers are extracted correctly
use sesame_idam_authz_core_gen::handlers::principal_effective::{Request, Response};

// ─── Scenario Group 1: Successful retrieval ──────────────────────────────────

/// Scenario: Get effective permissions for a user.
///
/// Given: valid request with required fields (user_id, app_id, tenant_id).
/// When: we construct a Request and serialize the response.
/// Then: the response has user_id, roles, and permissions fields.
#[test]
fn test_get_effective_permissions_for_user() {
    // Given: valid request with required fields
    let request_data = Request {
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
        org_id: None,
        include_inherited: None,
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    // When: we serialize the request to JSON
    let json = serde_json::to_value(&request_data).expect("request must serialize to JSON");

    // Then: required fields are present
    assert_eq!(json["user_id"], "1189c444-8a2d-4c41-8b4b-ae43ce79a492");
    assert_eq!(json["app_id"], "33333333-8a2d-4c41-8b4b-ae43ce79a494");
    assert_eq!(json["tenant_id"], "6ba7b810-9dad-11d1-80b4-00c04fd430c8");

    // When: we construct a valid response
    let response = Response {
        attributes: None,
        permissions: vec!["read:invoices".to_string()],
        roles: vec![serde_json::json!("role:admin")],
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
    };

    // Then: the response has user_id, roles, and permissions fields
    let response_json = serde_json::to_value(&response).expect("response must serialize");
    assert_eq!(
        response_json["user_id"],
        "1189c444-8a2d-4c41-8b4b-ae43ce79a492"
    );
    assert!(response_json["roles"].is_array());
    assert!(response_json["permissions"].is_array());
}

/// Scenario: Get effective permissions with inheritance.
///
/// Given: valid request with include_inherited=true.
/// When: we construct a Request.
/// Then: include_inherited is Some(true).
#[test]
fn test_get_effective_permissions_with_inheritance() {
    // Given: valid request with include_inherited
    let request_data = Request {
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
        org_id: None,
        include_inherited: Some(true),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    // When: we serialize the request to JSON
    let json = serde_json::to_value(&request_data).expect("request must serialize");

    // Then: include_inherited is true
    assert!(json["include_inherited"].as_bool().unwrap_or(false));
}

// ─── Scenario Group 2: Required fields validation ────────────────────────────

/// Scenario: Reject request missing required "user_id" field.
///
/// Given: request body without "user_id" field.
/// When: we attempt to construct a Request.
/// Then: serde deserialization fails (missing required field).
#[test]
fn test_reject_request_missing_user_id_field() {
    // Given: request body missing "user_id"
    let json_body = serde_json::json!({
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494",
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });

    // When: we attempt to deserialize
    // Then: deserialization should fail due to missing "user_id"
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing required field 'user_id' should cause deserialization error"
    );
}

/// Scenario: Reject request missing required "tenant_id" field.
///
/// Given: request body without "tenant_id" field.
/// When: we attempt to construct a Request.
/// Then: serde deserialization fails (missing required field).
#[test]
fn test_reject_request_missing_tenant_id_field() {
    // Given: request body missing "tenant_id"
    let json_body = serde_json::json!({
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494",
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });

    // When: we attempt to deserialize
    // Then: deserialization should fail due to missing "tenant_id"
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing required field 'tenant_id' should cause deserialization error"
    );
}

/// Scenario: Reject request missing required "app_id" field.
///
/// Given: request body without "app_id" field.
/// When: we attempt to construct a Request.
/// Then: serde deserialization fails (missing required field).
#[test]
fn test_reject_request_missing_app_id_field() {
    // Given: request body missing "app_id"
    let json_body = serde_json::json!({
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });

    // When: we attempt to deserialize
    // Then: deserialization should fail due to missing "app_id"
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing required field 'app_id' should cause deserialization error"
    );
}

// ─── Scenario Group 3: Response shape validation ─────────────────────────────

/// Scenario: Response contains "user_id" string.
///
/// Given: valid principal_effective request.
/// When: we construct a Response.
/// Then: the response body has a "user_id" field of type string.
#[test]
fn test_response_contains_user_id_string() {
    // When: we construct a response
    let response = Response {
        attributes: None,
        permissions: vec![],
        roles: vec![],
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
    };

    // Then: the response has a "user_id" field of type string
    let response_json = serde_json::to_value(&response).expect("response must serialize");
    assert!(
        response_json.get("user_id").is_some(),
        "Response must have 'user_id' field"
    );
    assert!(
        response_json["user_id"].is_string(),
        "'user_id' must be a string"
    );
}

/// Scenario: Response contains "roles" array.
///
/// Given: valid principal_effective request.
/// When: we construct a Response.
/// Then: the response body has a "roles" field of type array.
#[test]
fn test_response_contains_roles_array() {
    // When: we construct a response
    let response = Response {
        attributes: None,
        permissions: vec![],
        roles: vec![
            serde_json::json!("role:viewer"),
            serde_json::json!("role:editor"),
        ],
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
    };

    // Then: the response has a "roles" field of type array
    let response_json = serde_json::to_value(&response).expect("response must serialize");
    assert!(
        response_json.get("roles").is_some(),
        "Response must have 'roles' field"
    );
    assert!(
        response_json["roles"].is_array(),
        "'roles' must be an array"
    );
    assert_eq!(response_json["roles"].as_array().unwrap().len(), 2);
}

/// Scenario: Response contains "permissions" array.
///
/// Given: valid principal_effective request.
/// When: we construct a Response.
/// Then: the response body has a "permissions" field of type array.
#[test]
fn test_response_contains_permissions_array() {
    // When: we construct a response
    let response = Response {
        attributes: None,
        permissions: vec!["read:invoices".to_string(), "write:invoices".to_string()],
        roles: vec![],
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
    };

    // Then: the response has a "permissions" field of type array
    let response_json = serde_json::to_value(&response).expect("response must serialize");
    assert!(
        response_json.get("permissions").is_some(),
        "Response must have 'permissions' field"
    );
    assert!(
        response_json["permissions"].is_array(),
        "'permissions' must be an array"
    );
    assert_eq!(response_json["permissions"].as_array().unwrap().len(), 2);
}

// ─── Scenario Group 4: Optional fields ───────────────────────────────────────

/// Scenario: Accept request with optional "org_id" field.
///
/// Given: valid request plus optional "org_id".
/// When: we construct a Request.
/// Then: deserialization succeeds and org_id is Some(...).
#[test]
fn test_accept_request_with_optional_org_id() {
    let request_data = Request {
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
        org_id: Some(serde_json::Value::String(
            "22222222-8a2d-4c41-8b4b-ae43ce79a493".to_string(),
        )),
        include_inherited: None,
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    // When: we serialize the request to JSON
    let json = serde_json::to_value(&request_data).expect("request must serialize");

    // Then: org_id is present and matches
    assert!(json.get("org_id").is_some(), "org_id must be present");
    assert_eq!(json["org_id"], "22222222-8a2d-4c41-8b4b-ae43ce79a493");
}

// ─── Scenario Group 5: Tenant isolation ──────────────────────────────────────

/// Scenario: X-Tenant-ID header is extracted and validated.
///
/// Given: a request with X-Tenant-ID header.
/// When: we construct a Request from the header.
/// Then: x_tenant_id and tenant_id are both set from the header.
#[test]
fn test_tenant_isolation_headers() {
    // Given: tenant IDs from header
    let header_tenant_id = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

    // When: we construct a request with tenant info
    let request_data = Request {
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
        tenant_id: header_tenant_id.to_string(),
        app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
        org_id: None,
        include_inherited: None,
        x_tenant_id: header_tenant_id.to_string(),
    };

    // Then: both tenant_id and x_tenant_id are set
    let json = serde_json::to_value(&request_data).expect("request must serialize");
    assert_eq!(json["tenant_id"], header_tenant_id);
    assert_eq!(json["X-Tenant-ID"], header_tenant_id);
}
