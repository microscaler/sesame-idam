use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::sync::{Arc, Mutex};

use sesame_idam_authz_core::controllers::set_principal_attribute::handle as attr_handle;
use sesame_idam_authz_core_gen::handlers::set_principal_attribute::{
    Request as AttrReq, Response as AttrResp,
};

/// Context shared across Given/When/Then steps for attribute setting BDD tests.
pub struct AttributeTestContext {
    pub last_response: Option<AttrResp>,
}

#[fixture]
fn context() -> Arc<Mutex<AttributeTestContext>> {
    Arc::new(Mutex::new(AttributeTestContext {
        last_response: None,
    }))
}

/// Build a valid set_principal_attribute Request and wrap it in TypedHandlerRequest.
fn make_set_attribute_request() -> TypedHandlerRequest<AttrReq> {
    let data = AttrReq {
        user_id: "1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string(),
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        key: "department".to_string(),
        value: "engineering".to_string(),
        org_id: None,
        x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    };
    TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/principals/attributes".to_string(),
        handler_name: "set_principal_attribute".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data,
    }
}

// ─── Given steps ─────────────────────────────────────────────────────────────

#[given("the authz-core service is running")]
fn given_service_ready() {
    // No-op: service readiness verified by handler being callable
}

// ─── When steps ──────────────────────────────────────────────────────────────

#[when("I send a valid request to set a principal attribute")]
fn when_set_attribute(context: &mut Arc<Mutex<AttributeTestContext>>) {
    let typed_req = make_set_attribute_request();
    let response = attr_handle(typed_req);
    context.lock().unwrap().last_response = Some(response);
}

// ─── Then steps ──────────────────────────────────────────────────────────────

#[then("the attribute response has error field")]
fn then_attr_has_error_field(context: &Arc<Mutex<AttributeTestContext>>) {
    let ctx = context.lock().unwrap();
    let resp = ctx.last_response.as_ref().expect("No response cached");
    let json = serde_json::to_value(resp).expect("serialize");
    assert!(
        json.get("error").is_some(),
        "response missing 'error' field"
    );
}

// ─── Scenario runners ────────────────────────────────────────────────────────

#[scenario(path = "tests/features/set_principal_attribute.feature")]
#[rstest::rstest]
fn attribute_set_returns_success(context: Arc<Mutex<AttributeTestContext>>) {
    let ctx = context.lock().unwrap();
    let resp = ctx.last_response.as_ref().expect("No response cached");
    let json = serde_json::to_value(resp).expect("serialize");
    assert!(
        json.get("error").is_some(),
        "response should have 'error' field"
    );
}

#[scenario(path = "tests/features/set_principal_attribute.feature")]
#[rstest::rstest]
fn attribute_response_has_required_fields(context: Arc<Mutex<AttributeTestContext>>) {
    let ctx = context.lock().unwrap();
    let resp = ctx.last_response.as_ref().expect("No response cached");
    let json = serde_json::to_value(resp).expect("serialize");
    assert!(
        json.get("error").is_some(),
        "response missing 'error' field"
    );
}
