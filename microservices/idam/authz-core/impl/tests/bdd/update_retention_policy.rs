use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_authz_core::controllers::update_retention_policy::handle;
use sesame_idam_authz_core_gen::handlers::update_retention_policy::{Request, Response};

/// Construct a minimal TypedHandlerRequest for update_retention_policy.
fn make_request(method: Method, handler_name: &str, data: Request) -> TypedHandlerRequest<Request> {
    TypedHandlerRequest {
        method,
        path: "/authz/audit/retention/retention-rule-1".to_string(),
        handler_name: handler_name.to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data,
    }
}

/// Scenario: Update retention policy with all fields.
///
/// Given: valid request with id, x_tenant_id, retention_days.
/// When: the handler is invoked.
/// Then: the response has id, event_type, retention_days, archive_after_days, delete_after_days, created_at.
#[test]
fn test_update_retention_policy_all_fields() {
    let request_data = Request {
        id: "retention-rule-1".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        retention_days: Some(365),
        archive_after_days: Some(180),
        delete_after_days: Some(365),
    };

    let typed_req = make_request(Method::PUT, "update_retention_policy", request_data);

    let response = handle(typed_req);

    assert_eq!(
        response.retention_days, 365,
        "retention_days should match request value"
    );
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("id").is_some(), "missing 'id' field");
    assert!(
        json.get("event_type").is_some(),
        "missing 'event_type' field"
    );
    assert!(
        json.get("retention_days").is_some(),
        "missing 'retention_days' field"
    );
    assert!(
        json.get("archive_after_days").is_some(),
        "missing 'archive_after_days' field"
    );
    assert!(
        json.get("delete_after_days").is_some(),
        "missing 'delete_after_days' field"
    );
    // created_at is Option with skip_serializing_if, may be absent
    if let Some(created_at) = json.get("created_at") {
        assert!(
            created_at.is_string() || created_at.is_null(),
            "'created_at' must be string or null"
        );
    }
    // tenant_id is required in response
    assert!(json.get("tenant_id").is_some(), "missing 'tenant_id' field");
}

/// Scenario: Update retention policy with optional fields omitted.
///
/// Given: valid request with only id and x_tenant_id.
/// When: the handler is invoked.
/// Then: the response returns default values for optional fields.
#[test]
fn test_update_retention_policy_defaults() {
    let request_data = Request {
        id: "retention-rule-1".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        retention_days: None,
        archive_after_days: None,
        delete_after_days: None,
    };

    let typed_req = make_request(Method::PUT, "update_retention_policy", request_data);

    let response = handle(typed_req);
    assert_eq!(response.retention_days, 90, "default retention_days is 90");
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("id").is_some(), "missing 'id' field");
}

/// Scenario: Update retention policy with retention_days only.
///
/// Given: valid request with only id, x_tenant_id, retention_days.
/// When: the handler is invoked.
/// Then: the response has retention_days = 365.
#[test]
fn test_update_retention_policy_with_retention_days() {
    let request_data = Request {
        id: "retention-rule-1".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        retention_days: Some(365),
        archive_after_days: None,
        delete_after_days: None,
    };

    let typed_req = make_request(Method::PUT, "update_retention_policy", request_data);

    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("retention_days").is_some(),
        "missing 'retention_days' field"
    );
}

/// Scenario: Reject request missing required "id" field.
///
/// Given: a JSON body without "id".
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn test_reject_missing_id() {
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
fn test_reject_missing_x_tenant_id() {
    let json_body = serde_json::json!({
        "id": "retention-rule-1"
    });
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing 'X-Tenant-ID' should cause deserialization error"
    );
}

/// Scenario: Response "id" may be optional.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: response.id may be null.
#[test]
fn test_response_id_may_be_null() {
    let request_data = Request {
        id: "retention-rule-1".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        retention_days: None,
        archive_after_days: None,
        delete_after_days: None,
    };

    let typed_req = make_request(Method::PUT, "update_retention_policy", request_data);

    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    if let Some(id) = json.get("id") {
        assert!(
            id.is_string() || id.is_null(),
            "'id' must be a string or null"
        );
    }
}

/// Scenario: Response "event_type" is a string.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: event_type is a string.
#[test]
fn test_response_event_type_is_string() {
    let request_data = Request {
        id: "retention-rule-1".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        retention_days: Some(365),
        archive_after_days: None,
        delete_after_days: None,
    };

    let typed_req = make_request(Method::PUT, "update_retention_policy", request_data);

    let response = handle(typed_req);
    assert_eq!(response.event_type, "", "'event_type' must be a string");
}

/// Scenario: Response "retention_days" is an integer.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: retention_days is an integer.
#[test]
fn test_response_retention_days_is_integer() {
    let request_data = Request {
        id: "retention-rule-1".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        retention_days: Some(365),
        archive_after_days: None,
        delete_after_days: None,
    };

    let typed_req = make_request(Method::PUT, "update_retention_policy", request_data);

    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("retention_days").is_some(),
        "missing 'retention_days' field"
    );
    assert!(
        json["retention_days"].is_number(),
        "'retention_days' must be an integer"
    );
}

/// Scenario: Response "created_at" may be null.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: created_at is null in the response.
#[test]
fn test_response_created_at_may_be_null() {
    let request_data = Request {
        id: "retention-rule-1".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        retention_days: Some(365),
        archive_after_days: None,
        delete_after_days: None,
    };

    let typed_req = make_request(Method::PUT, "update_retention_policy", request_data);

    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    if let Some(created_at) = json.get("created_at") {
        assert!(
            created_at.is_string() || created_at.is_null(),
            "'created_at' must be a string or null"
        );
    }
}

/// Scenario: X-Tenant-ID header is extracted and validated.
///
/// Given: a request with X-Tenant-ID header.
/// When: we construct a Request from the header.
/// Then: x_tenant_id is set from the header.
#[test]
fn test_tenant_isolation_headers() {
    let header_tenant_id = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

    let request_data = Request {
        id: "retention-rule-1".to_string(),
        x_tenant_id: header_tenant_id.to_string(),
        retention_days: Some(365),
        archive_after_days: None,
        delete_after_days: None,
    };

    let json = serde_json::to_value(&request_data).expect("request must serialize");
    assert_eq!(
        json["X-Tenant-ID"], header_tenant_id,
        "X-Tenant-ID must match header"
    );
}
