use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_authz_core::controllers::search_audit_events::handle;
use sesame_idam_authz_core_gen::handlers::search_audit_events::{Request, Response};

/// Construct a minimal TypedHandlerRequest for search_audit_events.
fn make_request(
    filters: Option<sesame_idam_authz_core_gen::handlers::types::AuditEventFilter>,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> TypedHandlerRequest<Request> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/audit/events/search".to_string(),
        handler_name: "search_audit_events".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: Request {
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            filters,
            sort_by,
            sort_order,
        },
    }
}

/// Scenario: Search audit events returns empty items and zero total.
///
/// Given: a valid request with tenant_id and X-Tenant-ID.
/// When: the handler is invoked.
/// Then: the response has items=[], total=0.
#[test]
fn search_audit_events_returns_empty_response() {
    let typed_req = make_request(None, None, None);

    let response = handle(typed_req);

    assert!(
        response.items.is_empty(),
        "items should be empty (stub handler)"
    );
    assert_eq!(response.total, 0, "total should be 0 (stub handler)");
}

/// Scenario: Response has "items" array field.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: the response body has "items" field of type array.
#[test]
fn response_has_items_array() {
    let typed_req = make_request(None, None, None);
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("items").is_some(), "missing 'items' field");
    assert!(json["items"].is_array(), "'items' must be an array");
}

/// Scenario: Response has "total" integer field.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: the response body has "total" field of type integer.
#[test]
fn response_has_total_integer() {
    let typed_req = make_request(None, None, None);
    let response = handle(typed_req);
    assert_eq!(response.total, 0, "total should be 0");
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("total").is_some(), "missing 'total' field");
    assert!(json["total"].is_number(), "'total' must be an integer");
}

/// Scenario: Response has "limit" integer field.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: the response body has "limit" field.
#[test]
fn response_has_limit_integer() {
    let typed_req = make_request(None, None, None);
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("limit").is_some(), "missing 'limit' field");
    assert!(json["limit"].is_number(), "'limit' must be an integer");
}

/// Scenario: Response has "offset" integer field.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: the response body has "offset" field.
#[test]
fn response_has_offset_integer() {
    let typed_req = make_request(None, None, None);
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("offset").is_some(), "missing 'offset' field");
    assert!(json["offset"].is_number(), "'offset' must be an integer");
}

/// Scenario: Search with event_type filter.
///
/// Given: a valid request with event_type filter.
/// When: the handler is invoked.
/// Then: the response is returned (audit emission).
#[test]
fn search_with_event_type_filter() {
    let filter = sesame_idam_authz_core_gen::handlers::types::AuditEventFilter {
        event_type: "authentication".to_string(),
        actor: Default::default(),
        event_action: Default::default(),
        tenant_id: Default::default(),
        user_id: Default::default(),
        org_id: Default::default(),
        severity: Default::default(),
        start_time: Default::default(),
        end_time: Default::default(),
        limit: 0,
        offset: 0,
    };
    let typed_req = make_request(Some(filter), None, None);

    let response = handle(typed_req);
    assert!(
        response.items.is_empty(),
        "stub handler returns empty items"
    );
}

/// Scenario: Search with actor filter.
///
/// Given: a valid request with actor filter.
/// When: the handler is invoked.
/// Then: the response is returned (audit emission).
#[test]
fn search_with_actor_filter() {
    let filter = sesame_idam_authz_core_gen::handlers::types::AuditEventFilter {
        actor: "admin".to_string(),
        event_type: Default::default(),
        event_action: Default::default(),
        tenant_id: Default::default(),
        user_id: Default::default(),
        org_id: Default::default(),
        severity: Default::default(),
        start_time: Default::default(),
        end_time: Default::default(),
        limit: 0,
        offset: 0,
    };
    let typed_req = make_request(Some(filter), None, None);

    let response = handle(typed_req);
    assert!(
        response.items.is_empty(),
        "stub handler returns empty items"
    );
}

/// Scenario: Search with combined filters.
///
/// Given: a valid request with multiple filters.
/// When: the handler is invoked.
/// Then: the response is returned.
#[test]
fn search_with_combined_filters() {
    let filter = sesame_idam_authz_core_gen::handlers::types::AuditEventFilter {
        event_type: "authentication".to_string(),
        actor: "admin".to_string(),
        event_action: "login".to_string(),
        severity: "error".to_string(),
        limit: 10,
        offset: 0,
        tenant_id: Default::default(),
        user_id: Default::default(),
        org_id: Default::default(),
        start_time: Default::default(),
        end_time: Default::default(),
    };
    let typed_req = make_request(Some(filter), None, None);

    let response = handle(typed_req);
    assert!(
        response.items.is_empty(),
        "stub handler returns empty items"
    );
}

/// Scenario: Search with sort_by parameter.
///
/// Given: a valid request with sort_by.
/// When: the handler is invoked.
/// Then: the response is returned.
#[test]
fn search_with_sort_by() {
    let typed_req = make_request(None, Some("timestamp".to_string()), None);

    let response = handle(typed_req);
    assert!(
        response.items.is_empty(),
        "stub handler returns empty items"
    );
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

/// Scenario: Reject request missing required "X-Tenant-ID" header.
///
/// Given: a JSON body without "X-Tenant-ID".
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn reject_missing_x_tenant_id() {
    let json_body = serde_json::json!({
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
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
        tenant_id: tenant_id.to_string(),
        x_tenant_id: tenant_id.to_string(),
        filters: None,
        sort_by: None,
        sort_order: None,
    };

    let json = serde_json::to_value(&request_data).expect("serialize");
    assert_eq!(json["X-Tenant-ID"], tenant_id, "X-Tenant-ID must match");
}
