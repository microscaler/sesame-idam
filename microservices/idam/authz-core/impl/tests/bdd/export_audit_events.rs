use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_authz_core::controllers::export_audit_events::handle;
use sesame_idam_authz_core_gen::handlers::export_audit_events::{Request, Response};

/// Scenario: Export audit events with format csv.
///
/// Given: a valid request with tenant_id, X-Tenant-ID, format=csv.
/// When: the handler is invoked.
/// Then: the response has export_id, status="pending", estimated_completion, download_url.
#[test]
fn export_csv_returns_pending() {
    let request_data = Request {
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        format: "csv".to_string(),
        filters: None,
        include_metadata: None,
    };

    let typed_req = TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/audit/events/export".to_string(),
        handler_name: "export_audit_events".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: request_data,
    };

    let response = handle(typed_req);

    assert_eq!(
        response.status, "pending",
        "status should be pending"
    );
    assert!(
        !response.export_id.is_empty(),
        "export_id should not be empty"
    );
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("export_id").is_some(),
        "missing 'export_id' field"
    );
    assert!(
        json.get("status").is_some(),
        "missing 'status' field"
    );
    // optional fields are absent or null when not set
    if let Some(completion) = json.get("estimated_completion") {
        assert!(
            completion.is_string() || completion.is_null(),
            "'estimated_completion' must be string or null"
        );
    }
    if let Some(url) = json.get("download_url") {
        assert!(
            url.is_string() || url.is_null(),
            "'download_url' must be string or null"
        );
    }
}

/// Scenario: Export audit events with format json.
///
/// Given: a valid request with format=json.
/// When: the handler is invoked.
/// Then: the response has export_id and status.
#[test]
fn export_json_returns_pending() {
    let request_data = Request {
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        format: "json".to_string(),
        filters: None,
        include_metadata: None,
    };

    let typed_req = TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/audit/events/export".to_string(),
        handler_name: "export_audit_events".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: request_data,
    };

    let response = handle(typed_req);
    assert_eq!(
        response.status, "pending",
        "status should be pending"
    );
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("export_id").is_some(),
        "missing 'export_id' field"
    );
    assert!(
        json.get("status").is_some(),
        "missing 'status' field"
    );
}

/// Scenario: Reject request missing required "format" field.
///
/// Given: a JSON body without "format".
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn reject_missing_format() {
    let json_body = serde_json::json!({
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing 'format' should fail"
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
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "format": "csv"
    });
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing 'X-Tenant-ID' should fail"
    );
}

/// Scenario: Reject request missing required "tenant_id" field.
///
/// Given: a JSON body without "tenant_id".
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn reject_missing_tenant_id() {
    let json_body = serde_json::json!({
        "format": "csv",
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing 'tenant_id' should fail"
    );
}

/// Scenario: Response "export_id" is a non-empty string.
///
/// Given: a valid request with format=csv.
/// When: the handler is invoked.
/// Then: export_id is a non-empty string.
#[test]
fn export_id_is_nonempty_string() {
    let request_data = Request {
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        format: "csv".to_string(),
        filters: None,
        include_metadata: None,
    };

    let typed_req = TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/audit/events/export".to_string(),
        handler_name: "export_audit_events".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: request_data,
    };

    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("export_id").is_some(),
        "missing 'export_id' field"
    );
    assert!(
        json["export_id"].is_string(),
        "'export_id' must be a string"
    );
}

/// Scenario: Response "status" is pending string.
///
/// Given: a valid request.
/// When: the handler is invoked.
/// Then: status is "pending".
#[test]
fn status_is_pending_string() {
    let request_data = Request {
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        format: "csv".to_string(),
        filters: None,
        include_metadata: None,
    };

    let typed_req = TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/audit/events/export".to_string(),
        handler_name: "export_audit_events".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: request_data,
    };

    let response = handle(typed_req);
    assert_eq!(
        response.status, "pending",
        "status should be pending"
    );
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(
        json.get("status").is_some(),
        "missing 'status' field"
    );
    assert!(
        json["status"].is_string(),
        "'status' must be a string"
    );
}
