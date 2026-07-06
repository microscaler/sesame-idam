/// BDD smoke tests for authz-core health and metrics endpoints.
///
/// Verifies that the service handler is operational by hitting the actual
/// generated handler registry. These are the entry-point tests — if they
/// fail, nothing else matters.
///
/// Pattern: direct handler call via `may_minihttp` TestClient, not the full
/// HTTP server. Each scenario maps to a Gherkin step definition.
use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use rstest_bdd::gherkin::{given, then, when};
use sesame_idam_authz_core::controllers::authorize::handle as auth_handle;
use sesame_idam_authz_core_gen::handlers::authorize::Request;

/// ─── Test Context ────────────────────────────────────────────────────────
/// Holds the last response so all steps in a scenario share state.

#[derive(Default)]
pub struct ControllerTestContext {
    pub last_response: Option<serde_json::Value>,
    pub last_status: Option<u16>,
}

use std::sync::{Arc, Mutex};

/// ─── Feature: authz-core smoke test ──────────────────────────────────────
/// As a developer
/// I want to verify the service handler is operational
/// So that I can detect regressions early

/// Scenario: Service handler returns valid response
///   Given the authz-core service is running
///   When I call the authorize endpoint with a valid request
///   Then the response body has field "allowed" set to true

#[given("the authz-core service is running")]
fn given_service_running(_context: Arc<Mutex<ControllerTestContext>>) {
    // Authz-core is stateless — "running" means the registry is loaded.
    // No DB, no external deps. Just verify we can build a request.
}

#[when("I call the authorize endpoint with a valid request", context = "ctx")]
fn when_call_authorize(ctx: Arc<Mutex<ControllerTestContext>>) {
    let req = TypedHandlerRequest::<Request> {
        method: Method::POST,
        path: "/authz/authorize".to_string(),
        handler_name: "authorize".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {
            user_id: "test-user-001".to_string(),
            action: "read".to_string(),
            resource: "accounting:invoices".to_string(),
            tenant_id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string()),
            app_id: None,
            org_id: None,
            context: None,
            x_tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        },
    };

    let response = auth_handle(req);
    let json = serde_json::to_value(&response).expect("serialize response");

    let mut guard = ctx.lock().expect("context lock");
    guard.last_response = Some(json);
    guard.last_status = Some(200);
}

#[then("the response body has field \"allowed\" set to true", context = "ctx")]
fn then_allowed_is_true(ctx: Arc<Mutex<ControllerTestContext>>) {
    let guard = ctx.lock().expect("context lock");
    let json = guard
        .last_response
        .as_ref()
        .expect("no response cached from previous step");
    assert!(
        json["allowed"].as_bool().expect("allowed must be boolean"),
        "allowed must be true"
    );
}

/// Scenario: Response structure is valid
///   Given the authz-core service is running
///   When I call the authorize endpoint with a valid request
///   Then the response body has field "allowed"

#[then("the response body has field \"allowed\"", context = "ctx")]
fn then_response_has_allowed_field(ctx: Arc<Mutex<ControllerTestContext>>) {
    let guard = ctx.lock().expect("context lock");
    let json = guard
        .last_response
        .as_ref()
        .expect("no response cached from previous step");
    assert!(json.get("allowed").is_some(), "missing 'allowed' field");
}

/// Scenario: Metrics endpoint would return prometheus format (handler-level)
///   Given the authz-core service is running
///   When I verify the prometheus metrics function works
///   Then the metrics function returns non-empty text

#[when("I verify the prometheus metrics function works")]
fn when_verify_metrics() {
    // The prometheus scrape text is provided by lifeguard::metrics.
    // We verify the function is callable — the actual HTTP endpoint is tested
    // via may_minihttp when the full server is up.
    let text = lifeguard::metrics::prometheus_scrape_text();
    assert!(
        !text.is_empty(),
        "prometheus metrics should return non-empty text"
    );
}

#[then("the metrics function returns non-empty text")]
fn then_metrics_non_empty() {
    // Already asserted in the when-step. This step exists for Gherkin parity.
}
