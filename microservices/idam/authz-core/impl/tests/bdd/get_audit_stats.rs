use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_authz_core::controllers::get_audit_stats::handle;
use sesame_idam_authz_core_gen::handlers::get_audit_stats::Request;

/// Scenario: Get audit statistics for a tenant.
///
/// Given: a valid request with `tenant_id` and X-Tenant-ID.
/// When: the handler is invoked.
/// Then: the response contains total (0), `by_type`, `by_severity`.
#[test]
fn get_audit_stats_returns_valid_response() {
    let request_data = Request {
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        filters: None,
        sort_by: None,
        sort_order: None,
    };

    let typed_req = TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/audit/events/stats".to_string(),
        handler_name: "get_audit_stats".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: request_data,
    };

    let response = handle(typed_req);

    assert_eq!(response.total, 0, "Stub returns total 0");
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("total").is_some(), "missing 'total' field");
    assert!(json.get("by_type").is_some(), "missing 'by_type' field");
    assert!(
        json.get("by_severity").is_some(),
        "missing 'by_severity' field"
    );
}

/// Scenario: Response "total" is an integer.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: total is an integer.
#[test]
fn response_total_is_integer() {
    let request_data = Request {
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        filters: None,
        sort_by: None,
        sort_order: None,
    };

    let typed_req = TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/audit/events/stats".to_string(),
        handler_name: "get_audit_stats".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: request_data,
    };

    let response = handle(typed_req);
    assert!(
        response.total >= 0,
        "total should be >= 0, got {}",
        response.total
    );
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("total").is_some(),
        "Response must have 'total' field"
    );
    assert!(json["total"].is_number(), "'total' must be an integer");
}

/// Scenario: Response "`by_type`" is an object.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: `by_type` is an object.
#[test]
fn response_by_type_is_object() {
    let request_data = Request {
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        filters: None,
        sort_by: None,
        sort_order: None,
    };

    let typed_req = TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/audit/events/stats".to_string(),
        handler_name: "get_audit_stats".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: request_data,
    };

    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("by_type").is_some(),
        "Response must have 'by_type' field"
    );
    assert!(json["by_type"].is_object(), "'by_type' must be an object");
}

/// Scenario: Reject request missing required fields.
///
/// Given: an empty JSON body.
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn reject_missing_fields() {
    let json_body = serde_json::json!({});
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing fields should cause deserialization error"
    );
}
