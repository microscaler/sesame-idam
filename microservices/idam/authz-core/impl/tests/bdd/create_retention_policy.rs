use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_authz_core::controllers::create_retention_policy::handle;
use sesame_idam_authz_core_gen::handlers::create_retention_policy::Request;

/// Construct a minimal `TypedHandlerRequest` for `create_retention_policy`.
fn make_request(
    event_type: &str,
    retention_days: i32,
    archive_after_days: Option<i32>,
    delete_after_days: Option<i32>,
) -> TypedHandlerRequest<Request> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/audit/events/retention".to_string(),
        handler_name: "create_retention_policy".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            event_type: event_type.to_string(),
            retention_days,
            archive_after_days,
            delete_after_days,
        },
    }
}

/// Scenario: Create retention policy with all fields.
///
/// Given: a valid request with `event_type`, `retention_days`, `archive/delete_after_days`.
/// When: the handler is invoked.
/// Then: the response has id, `event_type`, `retention_days`, `archive/delete_after_days`, `created_at`, `tenant_id`.
#[test]
fn create_retention_policy_all_fields() {
    let typed_req = make_request("authentication", 365, Some(180), Some(365));

    let response = handle(typed_req);

    assert_eq!(
        response.event_type, "authentication",
        "event_type should match"
    );
    assert_eq!(response.retention_days, 365, "retention_days should match");
    assert_eq!(
        response.archive_after_days,
        Some(180),
        "archive_after_days should match"
    );
    assert_eq!(
        response.delete_after_days,
        Some(365),
        "delete_after_days should match"
    );
    assert!(
        response.id.as_deref().is_some_and(|s| !s.is_empty()),
        "id should be a generated UUID"
    );
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("id").is_some(), "missing 'id' field");
    assert!(
        json.get("event_type").is_some(),
        "missing 'event_type' field"
    );
    assert!(json.get("tenant_id").is_some(), "missing 'tenant_id' field");
    // created_at is optional (skip_serializing_if)
    if let Some(created_at) = json.get("created_at") {
        assert!(
            created_at.is_string() || created_at.is_null(),
            "'created_at' must be string or null"
        );
    }
}

/// Scenario: Create retention policy with only required fields.
///
/// Given: a valid request with only `event_type` and `retention_days`.
/// When: the handler is invoked.
/// Then: the response has defaults for optional fields.
#[test]
fn create_retention_policy_required_only() {
    let typed_req = make_request("audit", 90, None, None);

    let response = handle(typed_req);

    assert_eq!(response.event_type, "audit");
    assert_eq!(response.retention_days, 90);
    assert_eq!(response.archive_after_days, None);
    assert_eq!(response.delete_after_days, None);
    assert!(
        response.id.as_deref().is_some_and(|s| !s.is_empty()),
        "id should be a generated UUID"
    );
}

/// Scenario: Response "id" is a non-empty string (UUID).
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: response.id is a non-empty string.
#[test]
fn response_id_is_nonempty_string() {
    let typed_req = make_request("authentication", 365, None, None);
    let response = handle(typed_req);
    assert!(
        response.id.as_deref().is_some_and(|s| !s.is_empty()),
        "id should be a generated UUID, got '{:?}'",
        response.id
    );
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("id").is_some(), "missing 'id' field");
    assert!(json["id"].is_string(), "'id' must be a string");
}

/// Scenario: Response "`event_type`" is a string.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: `response.event_type` is a non-empty string.
#[test]
fn response_event_type_is_string() {
    let typed_req = make_request("security", 180, None, None);
    let response = handle(typed_req);
    assert_eq!(response.event_type, "security");
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("event_type").is_some(),
        "missing 'event_type' field"
    );
    assert!(
        json["event_type"].is_string(),
        "'event_type' must be a string"
    );
}

/// Scenario: Response "`retention_days`" is an integer.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: `response.retention_days` is an integer.
#[test]
fn response_retention_days_is_integer() {
    let typed_req = make_request("authentication", 365, None, None);
    let response = handle(typed_req);
    assert!(
        response.retention_days >= 0,
        "retention_days should be >= 0"
    );
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

/// Scenario: Response "`created_at`" is a string or null.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: `created_at` is present (as string) or absent (null).
#[test]
fn response_created_at_is_string_or_null() {
    let typed_req = make_request("authentication", 365, None, None);
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    if let Some(created_at) = json.get("created_at") {
        assert!(
            created_at.is_string() || created_at.is_null(),
            "'created_at' must be string or null"
        );
    }
}

/// Scenario: Response "`tenant_id`" is present.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: the response body has "`tenant_id`" field.
#[test]
fn response_has_tenant_id() {
    let typed_req = make_request("authentication", 365, None, None);
    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("tenant_id").is_some(), "missing 'tenant_id' field");
    assert!(
        json["tenant_id"].is_string(),
        "'tenant_id' must be a string"
    );
}

/// Scenario: Request with `archive_after_days` only.
///
/// Given: a valid request with `archive_after_days`.
/// When: the handler is invoked.
/// Then: the response reflects the archive value.
#[test]
fn create_with_archive_only() {
    let typed_req = make_request("audit", 90, Some(45), None);
    let response = handle(typed_req);
    assert_eq!(
        response.archive_after_days,
        Some(45),
        "archive_after_days should match"
    );
}

/// Scenario: Request with `delete_after_days` only.
///
/// Given: a valid request with `delete_after_days`.
/// When: the handler is invoked.
/// Then: the response reflects the delete value.
#[test]
fn create_with_delete_only() {
    let typed_req = make_request("audit", 90, None, Some(30));
    let response = handle(typed_req);
    assert_eq!(
        response.delete_after_days,
        Some(30),
        "delete_after_days should match"
    );
}

/// Scenario: Reject request missing required "`event_type`" field.
///
/// Given: a JSON body without "`event_type`".
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn reject_missing_event_type() {
    let json_body = serde_json::json!({
        "retention_days": 90,
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing 'event_type' should cause deserialization error"
    );
}

/// Scenario: Reject request missing required "`retention_days`" field.
///
/// Given: a JSON body without "`retention_days`".
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn reject_missing_retention_days() {
    let json_body = serde_json::json!({
        "event_type": "authentication",
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing 'retention_days' should cause deserialization error"
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
        "event_type": "authentication",
        "retention_days": 90
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
/// Then: `x_tenant_id` is set from the header.
#[test]
fn tenant_isolation_headers() {
    let tenant_id = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
    let request_data = Request {
        x_tenant_id: tenant_id.to_string(),
        event_type: "authentication".to_string(),
        retention_days: 365,
        archive_after_days: None,
        delete_after_days: None,
    };

    let json = serde_json::to_value(&request_data).expect("serialize");
    assert_eq!(json["X-Tenant-ID"], tenant_id, "X-Tenant-ID must match");
}
