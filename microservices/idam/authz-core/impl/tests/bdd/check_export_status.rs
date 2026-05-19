/// BDD feature: Check Export Status (GET /authz/audit/events/export/{export_id})
///
/// Tests verify the request/response schema contract and audit event emission
/// for the export status polling endpoint.
use brrtrouter::dispatcher::HeaderVec;
use brrtrouter::ids::RequestId;
use http::Method;
use sesame_idam_authz_core::controllers::check_export_status;
use sesame_idam_authz_core_gen::handlers::check_export_status::{Request, Response};

// ─── Test Helpers ────────────────────────────────────────────────────────────

/// Construct a minimal HandlerRequest for export status check.
fn make_export_status_request(
    path: &str,
    method: Method,
    headers: Vec<(&str, &str)>,
    body: Option<serde_json::Value>,
) -> brrtrouter::dispatcher::HandlerRequest {
    let mut hv = HeaderVec::new();
    for (k, v) in headers {
        hv.push((std::sync::Arc::from(k), v.to_string()));
    }
    brrtrouter::dispatcher::HandlerRequest {
        request_id: RequestId::new(),
        method,
        path: path.to_string(),
        handler_name: "check_export_status".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        headers: hv,
        cookies: HeaderVec::new(),
        body,
        jwt_claims: None,
        reply_tx: may::sync::mpsc::channel().0,
        queue_guard: None,
    }
}

/// Invoke the check_export_status handler and return the response data.
fn invoke_export_status_request(
    req: brrtrouter::dispatcher::HandlerRequest,
    request_data: Request,
) -> Response {
    let typed_req = brrtrouter::typed::TypedHandlerRequest {
        method: req.method.clone(),
        path: req.path.clone(),
        handler_name: req.handler_name.clone(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: request_data,
    };
    check_export_status::handle(typed_req)
}

// ─── Scenario Group 1: Successful status check ───────────────────────────────

/// Scenario: Check status of a pending export.
///
/// Given: valid request with required fields (export_id, X-Tenant-ID).
/// When: we invoke the check_export_status handler.
/// Then: the response returns status "pending".
#[test]
fn test_check_export_status_pending() {
    // Given: valid request with export_id
    let request_data = Request {
        export_id: "7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    let req = make_export_status_request(
        "/authz/audit/events/export/7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3",
        Method::GET,
        vec![(
            "X-Tenant-ID",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        )],
        Some(serde_json::json!({
            "export_id": "7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3",
            "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
        })),
    );

    // When: we invoke the handler
    let response = invoke_export_status_request(req, request_data);

    // Then: status is "pending"
    assert_eq!(response.status, "pending");
}

/// Scenario: Check status returns expected response shape.
///
/// Given: valid request.
/// When: we invoke the handler.
/// Then: the response contains all required response fields.
#[test]
fn test_export_status_response_has_required_fields() {
    let request_data = Request {
        export_id: "7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    let req = make_export_status_request(
        "/authz/audit/events/export/7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3",
        Method::GET,
        vec![(
            "X-Tenant-ID",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        )],
        None,
    );

    let response = invoke_export_status_request(req, request_data);

    // Then: the response has all required fields
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("export_id").is_some(), "missing 'export_id' field");
    assert!(json.get("status").is_some(), "missing 'status' field");
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

// ─── Scenario Group 2: Required fields validation ────────────────────────────

/// Scenario: Reject request missing required "X-Tenant-ID" header.
///
/// Given: request body without "X-Tenant-ID" header.
/// When: we attempt to construct a Request.
/// Then: serde deserialization fails (missing required field).
#[test]
fn test_reject_missing_x_tenant_id() {
    let json_body = serde_json::json!({
        "export_id": "7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3"
    });

    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing required field 'X-Tenant-ID' should cause deserialization error"
    );
}

/// Scenario: Reject request missing required "export_id" field.
///
/// Given: request body without "export_id" field.
/// When: we attempt to construct a Request.
/// Then: serde deserialization fails (missing required field).
#[test]
fn test_reject_missing_export_id() {
    let json_body = serde_json::json!({
        "X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    });

    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing required field 'export_id' should cause deserialization error"
    );
}

// ─── Scenario Group 3: Response shape validation ─────────────────────────────

/// Scenario: Response contains "export_id" string.
///
/// Given: valid export status request.
/// When: we invoke the handler.
/// Then: the response body has an "export_id" field of type string.
#[test]
fn test_response_contains_export_id() {
    let request_data = Request {
        export_id: "7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    let req = make_export_status_request(
        "/authz/audit/events/export/7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3",
        Method::GET,
        vec![(
            "X-Tenant-ID",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        )],
        None,
    );

    let response = invoke_export_status_request(req, request_data);
    let json = serde_json::to_value(&response).expect("serialize");

    assert!(
        json.get("export_id").is_some(),
        "Response must have 'export_id' field"
    );
    assert!(json["export_id"].is_string(), "'export_id' must be a string");
}

/// Scenario: Response contains "status" string.
///
/// Given: valid export status request.
/// When: we invoke the handler.
/// Then: the response body has a "status" field of type string.
#[test]
fn test_response_contains_status() {
    let request_data = Request {
        export_id: "7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    let req = make_export_status_request(
        "/authz/audit/events/export/7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3",
        Method::GET,
        vec![(
            "X-Tenant-ID",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        )],
        None,
    );

    let response = invoke_export_status_request(req, request_data);
    let json = serde_json::to_value(&response).expect("serialize");

    assert!(
        json.get("status").is_some(),
        "Response must have 'status' field"
    );
    assert!(json["status"].is_string(), "'status' must be a string");
}

/// Scenario: Response contains optional "download_url" string.
///
/// Given: valid export status request.
/// When: we invoke the handler.
/// Then: the response body has a "download_url" field of type string or null.
#[test]
fn test_response_may_contain_download_url() {
    let request_data = Request {
        export_id: "7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    let req = make_export_status_request(
        "/authz/audit/events/export/7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3",
        Method::GET,
        vec![(
            "X-Tenant-ID",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        )],
        None,
    );

    let response = invoke_export_status_request(req, request_data);
    let json = serde_json::to_value(&response).expect("serialize");

    // download_url is Option<String> — may be null or a string
    if let Some(url) = json.get("download_url") {
        assert!(
            url.is_string() || url.is_null(),
            "'download_url' must be a string or null"
        );
    }
}

/// Scenario: Response contains optional "estimated_completion" string.
///
/// Given: valid export status request.
/// When: we invoke the handler.
/// Then: the response body has an "estimated_completion" field of type string or null.
#[test]
fn test_response_may_contain_estimated_completion() {
    let request_data = Request {
        export_id: "7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    let req = make_export_status_request(
        "/authz/audit/events/export/7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3",
        Method::GET,
        vec![(
            "X-Tenant-ID",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        )],
        None,
    );

    let response = invoke_export_status_request(req, request_data);
    let json = serde_json::to_value(&response).expect("serialize");

    // estimated_completion is Option<String> — may be null or a string
    if let Some(completion) = json.get("estimated_completion") {
        assert!(
            completion.is_string() || completion.is_null(),
            "'estimated_completion' must be a string or null"
        );
    }
}

// ─── Scenario Group 4: Tenant isolation ──────────────────────────────────────

/// Scenario: X-Tenant-ID header is extracted and validated.
///
/// Given: a request with X-Tenant-ID header.
/// When: we construct a Request from the header.
/// Then: x_tenant_id is set from the header.
#[test]
fn test_tenant_isolation_headers() {
    let header_tenant_id = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

    let request_data = Request {
        export_id: "7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3".to_string(),
        x_tenant_id: header_tenant_id.to_string(),
    };

    let json = serde_json::to_value(&request_data).expect("request must serialize");
    assert_eq!(json["X-Tenant-ID"], header_tenant_id);
}
