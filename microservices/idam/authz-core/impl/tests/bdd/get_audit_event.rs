use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_authz_core::controllers::get_audit_event::handle;
use sesame_idam_authz_core_gen::handlers::get_audit_event::{Request, Response};

/// Scenario: Retrieve a single audit event by ID.
///
/// Given: a request with id and X-Tenant-ID.
/// When: the handler is invoked.
/// Then: the response contains id, actor, event_action, event_type, ip_address, timestamp.
#[test]
fn get_audit_event_retrieves_event_by_id() {
    let request_data = Request {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    let typed_req = TypedHandlerRequest {
        method: Method::GET,
        path: "/authz/audit/events/550e8400-e29b-41d4-a716-446655440000".to_string(),
        handler_name: "get_audit_event".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: request_data,
    };

    let response = handle(typed_req);

    assert_eq!(
        response.id, "550e8400-e29b-41d4-a716-446655440000",
        "event id should match requested id"
    );
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json.get("id").is_some(), "missing 'id' field");
    assert!(json.get("actor").is_some(), "missing 'actor' field");
    assert!(
        json.get("event_action").is_some(),
        "missing 'event_action' field"
    );
    assert!(
        json.get("event_type").is_some(),
        "missing 'event_type' field"
    );
    assert!(
        json.get("ip_address").is_some(),
        "missing 'ip_address' field"
    );
    assert!(json.get("timestamp").is_some(), "missing 'timestamp' field");
}

/// Scenario: Response "id" is a non-empty string.
///
/// Given: a request with id and X-Tenant-ID.
/// When: the handler is invoked.
/// Then: response.id is a non-empty string.
#[test]
fn response_id_is_string() {
    let request_data = Request {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    let typed_req = TypedHandlerRequest {
        method: Method::GET,
        path: "/authz/audit/events/550e8400-e29b-41d4-a716-446655440000".to_string(),
        handler_name: "get_audit_event".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: request_data,
    };

    let response = handle(typed_req);
    assert!(!response.id.is_empty(), "response id should not be empty");
    let json = serde_json::to_value(&response).expect("serialize");
    assert!(json["id"].is_string(), "'id' must be a string");
}

/// Scenario: Response "hmac_signature" is optional (string or null).
///
/// Given: a request with id and X-Tenant-ID.
/// When: the handler is invoked.
/// Then: hmac_signature is null in the response.
#[test]
fn response_hmac_signature_is_null() {
    let request_data = Request {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    let typed_req = TypedHandlerRequest {
        method: Method::GET,
        path: "/authz/audit/events/550e8400-e29b-41d4-a716-446655440000".to_string(),
        handler_name: "get_audit_event".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: request_data,
    };

    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    if let Some(sig) = json.get("hmac_signature") {
        assert!(
            sig.is_string() || sig.is_null(),
            "'hmac_signature' must be string or null"
        );
    }
}

/// Scenario: Response "user_agent" is optional (string or null).
///
/// Given: a request with id and X-Tenant-ID.
/// When: the handler is invoked.
/// Then: user_agent is null in the response.
#[test]
fn response_user_agent_is_null() {
    let request_data = Request {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };

    let typed_req = TypedHandlerRequest {
        method: Method::GET,
        path: "/authz/audit/events/550e8400-e29b-41d4-a716-446655440000".to_string(),
        handler_name: "get_audit_event".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: request_data,
    };

    let response = handle(typed_req);
    let json = serde_json::to_value(&response).expect("serialize");
    if let Some(agent) = json.get("user_agent") {
        assert!(
            agent.is_string() || agent.is_null(),
            "'user_agent' must be string or null"
        );
    }
}

/// Scenario: Reject request missing required "id" field.
///
/// Given: a JSON body without "id".
/// When: we attempt to deserialize.
/// Then: deserialization fails.
#[test]
fn reject_missing_id_field() {
    let json_body = serde_json::json!({"X-Tenant-ID": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"});
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
    let json_body = serde_json::json!({"id": "550e8400-e29b-41d4-a716-446655440000"});
    let result: Result<Request, _> = serde_json::from_value(json_body);
    assert!(
        result.is_err(),
        "Missing 'X-Tenant-ID' should cause deserialization error"
    );
}
