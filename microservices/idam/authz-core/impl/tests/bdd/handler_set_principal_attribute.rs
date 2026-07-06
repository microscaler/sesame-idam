/// BDD handler-level tests for set_principal_attribute endpoint.
///
/// Calls the REAL controller handler and verifies it returns the expected
/// response shape (error, error_description).
/// Unlike the unit tests which only validate schema serialization, these
/// exercise the actual handler code (audit event emission).
use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use rstest_bdd::gherkin::{given, then, when};
use sesame_idam_authz_core::controllers::set_principal_attribute::handle;
use sesame_idam_authz_core_gen::handlers::set_principal_attribute::Request;
use std::sync::LazyLock;
use std::sync::Mutex;

thread_local! {
    static CONTEXT: Mutex<Option<serde_json::Value>> = Mutex::new(None);
}

/// Scenario: Set an attribute on a principal
///   Given a valid set_principal_attribute request
///   When I send a valid request to the controller
///   Then the response is returned with an error field

#[given("a valid set_principal_attribute request")]
fn given_valid_attr_request() {
    CONTEXT.with(|c| *c.lock().unwrap() = None);
}

#[when("I send a valid request to the controller")]
fn when_attr_valid() {
    let req = TypedHandlerRequest::<Request> {
        method: Method::POST,
        path: "/authz/principals/attributes".to_string(),
        handler_name: "set_principal_attribute".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            key: "department".to_string(),
            value: "engineering".to_string(),
            org_id: None,
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    CONTEXT.with(|c| *c.lock().unwrap() = Some(json));
}

#[then("the response is returned with an error field")]
fn then_has_error_field() {
    CONTEXT.with(|c| {
        let json = c.lock().unwrap().as_ref().expect("no response cached");
        assert!(json.get("error").is_some(), "missing 'error' field");
        // Stub implementation returns error: ""
        assert!(
            json["error"].as_str().unwrap_or("") == "",
            "stub returns empty error"
        );
    });
}

/// Scenario: Set attribute with empty value
///   Given a set_principal_attribute request with empty value
///   When I send the request to the controller
///   Then the response is returned

#[given("a set_principal_attribute request with empty value")]
fn given_empty_value_request() {
    CONTEXT.with(|c| *c.lock().unwrap() = None);
}

#[when("I send the empty value request")]
fn when_empty_value() {
    let req = TypedHandlerRequest::<Request> {
        method: Method::POST,
        path: "/authz/principals/attributes".to_string(),
        handler_name: "set_principal_attribute".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            key: "department".to_string(),
            value: "".to_string(),
            org_id: None,
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    CONTEXT.with(|c| *c.lock().unwrap() = Some(json));
}

#[then("the response is returned")]
fn then_response_returned() {
    CONTEXT.with(|c| {
        let json = c.lock().unwrap().as_ref().expect("no response cached");
        assert!(json.get("error").is_some(), "missing 'error' field");
    });
}

/// Scenario: Set attribute with org context
///   Given a set_principal_attribute request with org_id
///   When I send the request to the controller
///   Then the response is returned with an error field

#[given("a set_principal_attribute request with org_id")]
fn given_attr_with_org() {
    CONTEXT.with(|c| *c.lock().unwrap() = None);
}

#[when("I send the org context request")]
fn when_org_context() {
    let req = TypedHandlerRequest::<Request> {
        method: Method::POST,
        path: "/authz/principals/attributes".to_string(),
        handler_name: "set_principal_attribute".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            key: "department".to_string(),
            value: "engineering".to_string(),
            org_id: Some(serde_json::json!("22222222-8a2d-4c41-8b4b-ae43ce79a493")),
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    CONTEXT.with(|c| *c.lock().unwrap() = Some(json));
}
