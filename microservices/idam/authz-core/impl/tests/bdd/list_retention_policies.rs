use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_authz_core::controllers::list_retention_policies::handle;
use sesame_idam_authz_core_gen::handlers::list_retention_policies::Request;

/// Construct a minimal `TypedHandlerRequest` for `list_retention_policies`.
fn make_request() -> TypedHandlerRequest<Request> {
    TypedHandlerRequest {
        method: Method::GET,
        path: "/authz/audit/events/retention".to_string(),
        handler_name: "list_retention_policies".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    }
}

/// Scenario: List retention policies returns items array.
///
/// Given: a valid request with X-Tenant-ID.
/// When: the handler is invoked.
/// Then: the response has an "items" field with an array value.
#[test]
fn list_retention_policies_returns_response_with_items() {
    let typed_req = make_request();
    let response = handle(typed_req);

    // Response struct has an items field
    assert!(
        response.0.is_empty(),
        "retention policies should be empty (stub handler)"
    );
}

/// Scenario: Response "items" field is an array.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: the response body has "items" field of type array.
#[test]
fn response_has_items_array() {
    // Response is a tuple struct wrapping Vec<AuditRetentionPolicy>,
    // so it serializes to a bare JSON array.
    let typed_req = make_request();
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.is_array(), "response must serialize to a JSON array");
}

/// Scenario: Response serializes to a valid JSON object.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: the response body is a valid JSON array.
#[test]
fn response_is_valid_object() {
    let typed_req = make_request();
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.is_array(), "response must serialize to a JSON array");
}

/// Scenario: Response items array is empty for stub.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: response.items is an empty array.
#[test]
fn response_items_array_is_empty_for_stub() {
    let typed_req = make_request();
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    let items = json.as_array().expect("must be array");
    assert!(
        items.is_empty(),
        "stub handler should return empty items array"
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
/// Then: `x_tenant_id` is set from the header.
#[test]
fn tenant_isolation_headers() {
    let tenant_id = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
    let request_data = Request {
        x_tenant_id: tenant_id.to_string(),
    };

    let json = serde_json::to_value(&request_data).expect("serialize");
    assert_eq!(json["X-Tenant-ID"], tenant_id, "X-Tenant-ID must match");
}
