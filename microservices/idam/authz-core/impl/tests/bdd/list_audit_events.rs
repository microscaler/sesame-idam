use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_authz_core::controllers::list_audit_events::handle;
use sesame_idam_authz_core_gen::handlers::list_audit_events::Request;

// NOTE: The generated `Response` for list_audit_events is currently an empty
// struct because the OpenAPI spec does not define a response schema for the
// 200 case (expected: items/total/page/limit envelope). Once the spec adds
// the AuditEventList schema and codegen is re-run, these tests must be
// extended to assert items/total/page/limit fields.

/// Construct a minimal `TypedHandlerRequest` for `list_audit_events`.
#[allow(clippy::too_many_arguments)]
fn make_request(
    page: Option<i32>,
    limit: Option<i32>,
    event_type: Option<String>,
    user_id: Option<String>,
    severity: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
) -> TypedHandlerRequest<Request> {
    TypedHandlerRequest {
        method: Method::GET,
        path: "/authz/audit/events".to_string(),
        handler_name: "list_audit_events".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            id: "default-event-id".to_string(),
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            page,
            limit,
            event_type,
            user_id,
            severity,
            start_time,
            end_time,
        },
        jwt_claims: None,
    }
}

/// Scenario: List audit events returns a serializable response.
///
/// Given: a valid request with id and X-Tenant-ID.
/// When: the handler is invoked.
/// Then: the handler completes and the response serializes to a JSON object.
#[test]
fn list_audit_events_returns_serializable_response() {
    let typed_req = make_request(None, None, None, None, None, None, None);

    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.is_object(), "response must serialize to a JSON object");
}

/// Scenario: Request with page and limit filters is accepted.
#[test]
fn list_with_page_and_limit() {
    let typed_req = make_request(Some(1), Some(10), None, None, None, None, None);
    let _response = handle(typed_req);
}

/// Scenario: Request with `event_type` filter is accepted.
#[test]
fn list_with_event_type_filter() {
    let typed_req = make_request(
        None,
        None,
        Some("authentication".to_string()),
        None,
        None,
        None,
        None,
    );
    let _response = handle(typed_req);
}

/// Scenario: Request with severity filter is accepted.
#[test]
fn list_with_severity_filter() {
    let typed_req = make_request(
        None,
        None,
        None,
        None,
        Some("error".to_string()),
        None,
        None,
    );
    let _response = handle(typed_req);
}

/// Scenario: Request with time range filters is accepted.
#[test]
fn list_with_time_range() {
    let typed_req = make_request(
        None,
        None,
        None,
        None,
        None,
        Some("2024-01-01T00:00:00Z".to_string()),
        Some("2024-12-31T23:59:59Z".to_string()),
    );
    let _response = handle(typed_req);
}

/// Scenario: Reject request missing required "id" field.
#[test]
fn reject_missing_id_field() {
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
#[test]
fn reject_missing_x_tenant_id() {
    let json_body = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000"
    });
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing 'X-Tenant-ID' should cause deserialization error"
    );
}

/// Scenario: X-Tenant-ID header is extracted correctly.
#[test]
fn tenant_isolation_headers() {
    let tenant_id = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
    let request_data = Request {
        id: "test-id".to_string(),
        x_tenant_id: tenant_id.to_string(),
        page: None,
        limit: None,
        event_type: None,
        user_id: None,
        severity: None,
        start_time: None,
        end_time: None,
    };

    let json = serde_json::to_value(&request_data).expect("serialize");
    assert_eq!(json["X-Tenant-ID"], tenant_id, "X-Tenant-ID must match");
}
