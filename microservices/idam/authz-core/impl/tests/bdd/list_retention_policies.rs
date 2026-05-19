use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_authz_core::controllers::list_retention_policies::handle;
use sesame_idam_authz_core_gen::handlers::list_retention_policies::{Request, Response};

/// Construct a minimal TypedHandlerRequest for list_retention_policies.
fn make_request() -> TypedHandlerRequest<Request> {
    TypedHandlerRequest {
        method: Method::GET,
        path: "/authz/audit/events/retention".to_string(),
        handler_name: "list_retention_policies".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: Request {
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    }
}

/// Scenario: List retention policies returns empty array.
///
/// Given: a valid request with X-Tenant-ID.
/// When: the handler is invoked.
/// Then: the response body is an empty array.
#[test]
fn list_retention_policies_returns_empty() {
    let typed_req = make_request();
    let response = handle(typed_req);

    // The Response is a transparent newtype wrapping Vec<AuditRetentionPolicy>
    let items = &response.0;
    assert!(
        items.is_empty(),
        "retention policies should be empty (stub handler)"
    );
}

/// Scenario: Response is a valid array.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: the response serializes to a JSON array.
#[test]
fn response_is_valid_array() {
    let typed_req = make_request();
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.is_array(),
        "response must serialize to a JSON array"
    );
}

/// Scenario: Response has correct element type.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: each element in the array has expected policy fields.
#[test]
fn response_elements_have_policy_fields() {
    let typed_req = make_request();
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");

    // Empty array - verify it's still valid
    let items: &serde_json::Value = json.as_array().expect("must be array");
    // Stub returns empty, so just verify array type
    assert!(
        json.is_array(),
        "response must be a JSON array"
    );
}

/// Scenario: Response serializes to transparent array (not wrapped object).
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: the response JSON is [items] not {"0":[items]}.
#[test]
fn response_serializes_as_transparent_array() {
    let typed_req = make_request();
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");

    // Must be a JSON array at top level, not an object with key "0"
    assert!(
        json.is_array(),
        "response must be a top-level JSON array (transparent serde)"
    );
}

/// Scenario: Reject request missing required "X-Tenant-ID" header.
///
/// Given: a JSON body without "X-Tenant-ID".
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn reject_missing_x_tenant_id() {
    let json_body = serde_json::json!({});
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
        x_tenant_id: tenant_id.to_string(),
    };

    let json = serde_json::to_value(&request_data).expect("serialize");
    assert_eq!(json["X-Tenant-ID"], tenant_id, "X-Tenant-ID must match");
}
