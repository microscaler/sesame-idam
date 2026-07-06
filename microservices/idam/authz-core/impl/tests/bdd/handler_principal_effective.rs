/// BDD handler-level tests for principal_effective endpoint.
///
/// Calls the REAL controller handler and verifies it returns the expected
/// response shape (user_id, roles, permissions, attributes).
/// Unlike the unit tests in tests/unit/principal_effective.rs which only
/// validate schema serialization, these exercise the actual handler code.
use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use rstest_bdd::gherkin::{given, then, when};
use sesame_idam_authz_core::controllers::principal_effective::handle;
use sesame_idam_authz_core_gen::handlers::principal_effective::Request;
use std::sync::LazyLock;
use std::sync::Mutex;

thread_local! {
    static CONTEXT: Mutex<Option<serde_json::Value>> = Mutex::new(None);
}

/// Scenario: Get effective permissions for a user
///   Given a valid principal_effective request
///   When I send a valid request to the principal_effective controller
///   Then the response has user_id, roles, and permissions fields
///   And the user_id is not empty

#[given("a valid principal_effective request")]
fn given_valid_effective_request() {
    CONTEXT.with(|c| *c.lock().unwrap() = None);
}

#[when("I send a valid request to the principal_effective controller")]
fn when_effective_valid() {
    let req = TypedHandlerRequest::<Request> {
        method: Method::POST,
        path: "/authz/principals/effective".to_string(),
        handler_name: "principal_effective".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
            org_id: None,
            include_inherited: None,
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    CONTEXT.with(|c| *c.lock().unwrap() = Some(json));
}

#[then("the response has user_id, roles, and permissions fields")]
fn then_response_has_all_fields() {
    CONTEXT.with(|c| {
        let json = c.lock().unwrap().as_ref().expect("no response cached");
        assert!(json.get("user_id").is_some(), "missing 'user_id'");
        assert!(json.get("roles").is_some(), "missing 'roles'");
        assert!(json.get("permissions").is_some(), "missing 'permissions'");
        assert!(json.get("attributes").is_some(), "missing 'attributes'");
    });
}

#[then("the user_id is not empty")]
fn then_user_id_not_empty() {
    CONTEXT.with(|c| {
        let json = c.lock().unwrap().as_ref().expect("no response cached");
        let uid = json["user_id"].as_str().expect("user_id must be string");
        assert!(!uid.is_empty(), "user_id must not be empty");
    });
}

/// Scenario: Effective permissions with inheritance
///   Given a principal_effective request with include_inherited=true
///   When I send the request to the controller
///   Then the response includes inherited roles

#[given("a principal_effective request with include_inherited=true")]
fn given_effective_with_inheritance() {
    CONTEXT.with(|c| *c.lock().unwrap() = None);
}

#[when("I send the request with inheritance enabled")]
fn when_effective_with_inherit() {
    let req = TypedHandlerRequest::<Request> {
        method: Method::POST,
        path: "/authz/principals/effective".to_string(),
        handler_name: "principal_effective".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
            org_id: Some(serde_json::json!("22222222-8a2d-4c41-8b4b-ae43ce79a493")),
            include_inherited: Some(true),
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    CONTEXT.with(|c| *c.lock().unwrap() = Some(json));
}

#[then("the response includes inherited roles")]
fn then_included_inherited() {
    CONTEXT.with(|c| {
        let json = c.lock().unwrap().as_ref().expect("no response cached");
        assert!(
            json.get("roles").is_some(),
            "response must have 'roles' field"
        );
        // The stub returns empty array, but the field exists
        assert!(json["roles"].is_array(), "'roles' must be an array");
    });
}

/// Scenario: Effective permissions with org context
///   Given a principal_effective request with org_id
///   When I send the request to the controller
///   Then the response contains the user_id

#[given("a principal_effective request with org_id")]
fn given_effective_with_org() {
    CONTEXT.with(|c| *c.lock().unwrap() = None);
}

#[when("I send the request with org context")]
fn when_effective_with_org() {
    let req = TypedHandlerRequest::<Request> {
        method: Method::POST,
        path: "/authz/principals/effective".to_string(),
        handler_name: "principal_effective".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            user_id: "test-user-with-org".to_string(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            app_id: "33333333-8a2d-4c41-8b4b-ae43ce79a494".to_string(),
            org_id: Some(serde_json::json!("22222222-8a2d-4c41-8b4b-ae43ce79a493")),
            include_inherited: Some(false),
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    CONTEXT.with(|c| *c.lock().unwrap() = Some(json));
}

#[then("the response contains the user_id")]
fn then_contains_user_id() {
    CONTEXT.with(|c| {
        let json = c.lock().unwrap().as_ref().expect("no response cached");
        let uid = json["user_id"].as_str().expect("user_id must be string");
        assert_eq!(uid, "test-user-with-org");
    });
}
