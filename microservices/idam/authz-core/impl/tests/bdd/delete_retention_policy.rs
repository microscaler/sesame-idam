use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_authz_core::controllers::delete_retention_policy::handle;
use sesame_idam_authz_core_gen::handlers::delete_retention_policy::{
    Request, Response,
};

/// Construct a minimal TypedHandlerRequest for delete_retention_policy.
fn make_request(id: &str) -> TypedHandlerRequest<Request> {
    TypedHandlerRequest {
        method: Method::DELETE,
        path: format!("/authz/audit/events/retention/{id}"),
        handler_name: "delete_retention_policy".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: Request {
            id: id.to_string(),
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    }
}

/// Scenario: Delete retention policy returns success response.
///
/// Given: a valid request with policy id and X-Tenant-ID.
/// When: the handler is invoked.
/// Then: the response has an error field.
#[test]
fn delete_retention_policy_returns_response() {
    let typed_req = make_request("policy-123");

    let response = handle(typed_req);

    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("error").is_some(),
        "response should have 'error' field"
    );
}

/// Scenario: Response "error" field is a string.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: response.error is a string.
#[test]
fn response_error_is_string() {
    let typed_req = make_request("policy-123");
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("error").is_some(),
        "missing 'error' field"
    );
    assert!(
        json["error"].is_string(),
        "'error' must be a string"
    );
}

/// Scenario: Response "error_description" is optional string.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: error_description is string or null.
#[test]
fn response_error_description_is_optional_string() {
    let typed_req = make_request("policy-123");
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    if let Some(desc) = json.get("error_description") {
        assert!(
            desc.is_string() || desc.is_null(),
            "'error_description' must be string or null"
        );
    }
}

/// Scenario: Delete with different policy IDs works.
///
/// Given: a valid request with various policy IDs.
/// When: the handler is invoked for each.
/// Then: the response is returned consistently.
#[test]
fn delete_various_policy_ids() {
    for id in ["policy-1", "550e8400-e29b-41d4-a716-446655440000", "rule-abc"] {
        let typed_req = make_request(id);
        let response = handle(typed_req);
        let json = serde_json::to_value(&response).expect("serialize");
        assert!(
            json.get("error").is_some(),
            "response should have 'error' field for id={id}"
        );
    }
}

/// Scenario: Reject request missing required "id" path parameter.
///
/// Given: a JSON body without "id".
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn reject_missing_id() {
    let json_body = serde_json::json!({
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing 'id' should cause deserialization error"
    );
}

/// Scenario: Reject request missing required "X-Tenant-ID" header.
///
/// Given: a JSON body without "X-Tenant-ID".
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn reject_missing_x_tenant_id() {
    let json_body = serde_json::json!({
        "id": "policy-123"
    });
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing 'X-Tenant-ID' should cause deserialization error"
    );
}

/// Scenario: X-Tenant-ID header is extracted correctly.
///
/// Given: a request with X-Tenant-ID header.
/// When: we construct a Request.
/// Then: x_tenant_id is set from the header.
#[test]
fn tenant_isolation_headers() {
    let tenant_id = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
    let request_data = Request {
        id: "policy-123".to_string(),
        x_tenant_id: tenant_id.to_string(),
    };

    let json = serde_json::to_value(&request_data).expect("serialize");
    assert_eq!(json["X-Tenant-ID"], tenant_id, "X-Tenant-ID must match");
}
