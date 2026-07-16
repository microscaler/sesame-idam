//! Integration tests for the SeaRates GraphQL mock server.
//!
//! These tests hit the real handler (via `build_app`) without starting a
//! TCP listener, so they run quickly inside `cargo test`.

use axum_test::TestServer;
use serde_json::json;

/// Helper: build a test server with the searates-graphql router.
async fn test_server() -> TestServer {
    let state = pact_mock_server::searates_graphql::AppState::from_pact_file("SEARATES-API.json");
    let app = pact_mock_server::searates_graphql::build_app(state);
    TestServer::new(app).unwrap()
}

// ============================================================================
// Health check
// ============================================================================

#[tokio::test]
async fn health_returns_ok() {
    let server = test_server().await;

    let response = server.get("/health").await;
    assert_eq!(response.status_code(), 200);

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn root_path_returns_ok() {
    let server = test_server().await;

    let response = server.get("/").await;
    assert_eq!(response.status_code(), 200);
}

// ============================================================================
// GraphQL — FCL rate query (success)
// ============================================================================

#[tokio::test]
async fn fcl_rate_query_returns_200_with_rates() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .json(&json!({
            "query": "query GetShippingRates($params: GetShippingRatesParams!) { getShippingRates(params: $params) { ... } }",
            "variables": {
                "params": {
                    "shippingType": "FCL",
                    "pointIdFrom": "UAODS",
                    "pointIdTo": "JPTYO",
                    "date": "2026-05-01"
                }
            }
        }))
        .await;

    assert_eq!(response.status_code(), 200);
    let body: serde_json::Value = response.json();
    assert!(
        body.get("data").is_some(),
        "Response should have 'data' field"
    );
}

#[tokio::test]
async fn fcl_rate_query_inline_params_returns_200() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .json(&json!({
            "query": "query { getShippingRates(params: {shippingType: FCL, pointIdFrom: UAODS, pointIdTo: JPTYO, date: \"2026-05-01\"}) { ... } }"
        }))
        .await;

    assert_eq!(response.status_code(), 200);
}

// ============================================================================
// GraphQL — LCL rate query (success)
// ============================================================================

#[tokio::test]
async fn lcl_rate_query_returns_200_with_rates() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .json(&json!({
            "query": "query GetShippingRates($params: GetShippingRatesParams!) { getShippingRates(params: $params) { ... } }",
            "variables": {
                "params": {
                    "shippingType": "LCL",
                    "pointIdFrom": "CNSHA",
                    "pointIdTo": "USLAX",
                    "date": "2026-06-01"
                }
            }
        }))
        .await;

    assert_eq!(response.status_code(), 200);
    let body: serde_json::Value = response.json();
    assert!(
        body.get("data").is_some(),
        "Response should have 'data' field"
    );
}

// ============================================================================
// GraphQL — INVALID_TYPE error (400)
// ============================================================================

#[tokio::test]
async fn invalid_shipping_type_returns_400() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .json(&json!({
            "query": "query GetShippingRates($params: GetShippingRatesParams!) { getShippingRates(params: $params) { ... } }",
            "variables": {
                "params": {
                    "shippingType": "INVALID_TYPE",
                    "pointIdFrom": "UAODS",
                    "pointIdTo": "JPTYO",
                    "date": "2026-05-01"
                }
            }
        }))
        .await;

    assert_eq!(response.status_code(), 400);
    let body: serde_json::Value = response.json();
    let errors = body.get("errors").and_then(|e| e.as_array());
    assert!(errors.is_some(), "Response should have 'errors' array");
    let first_error = errors.unwrap().first();
    let msg = first_error
        .unwrap()
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("");
    assert!(
        msg.contains("INVALID_TYPE"),
        "Error message should mention INVALID_TYPE, got: {}",
        msg
    );
}

#[tokio::test]
async fn invalid_shipping_type_array_returns_400() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .json(&json!({
            "query": "query GetShippingRates($params: GetShippingRatesParams!) { getShippingRates(params: $params) { ... } }",
            "variables": {
                "params": {
                    "shippingTypes": ["INVALID_TYPE", "LCL"],
                    "pointIdFrom": "UAODS",
                    "pointIdTo": "JPTYO",
                    "date": "2026-05-01"
                }
            }
        }))
        .await;

    assert_eq!(response.status_code(), 400);
}

// ============================================================================
// GraphQL — Place not found error (400)
// ============================================================================

#[tokio::test]
async fn unknown_point_id_from_returns_400() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .json(&json!({
            "query": "query { getShippingRates(params: {shippingType: FCL, pointIdFrom: INVALIDCODE, pointIdTo: JPTYO}) { ... } }"
        }))
        .await;

    assert_eq!(response.status_code(), 400);
    let body: serde_json::Value = response.json();
    let errors = body.get("errors").and_then(|e| e.as_array());
    assert!(errors.is_some(), "Response should have 'errors' array");
    let first_error = errors.unwrap().first();
    let msg = first_error
        .unwrap()
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("");
    assert!(
        msg.contains("not found"),
        "Error should mention 'not found', got: {}",
        msg
    );
}

#[tokio::test]
async fn unknown_point_id_to_returns_400() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .json(&json!({
            "query": "query { getShippingRates(params: {shippingType: FCL, pointIdFrom: UAODS, pointIdTo: BADCODE}) { ... } }"
        }))
        .await;

    assert_eq!(response.status_code(), 400);
}

// ============================================================================
// GraphQL — Auth failure headers (401, 403)
// ============================================================================

#[tokio::test]
async fn auth_failure_401_returns_401() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .add_header("X-Auth-Failure", "401")
        .json(&json!({
            "query": "query { getShippingRates(params: {shippingType: FCL}) { ... } }"
        }))
        .await;

    assert_eq!(response.status_code(), 401);
    let body: serde_json::Value = response.json();
    let errors = body.get("errors").and_then(|e| e.as_array());
    assert!(errors.is_some(), "Response should have 'errors' array");
    let msg = errors
        .unwrap()
        .first()
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("");
    assert!(
        msg.contains("token"),
        "Error should mention 'token', got: {}",
        msg
    );
}

#[tokio::test]
async fn auth_failure_403_returns_403() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .add_header("X-Auth-Failure", "403")
        .json(&json!({
            "query": "query { getShippingRates(params: {shippingType: FCL}) { ... } }"
        }))
        .await;

    assert_eq!(response.status_code(), 403);
}

// ============================================================================
// GraphQL — Service unavailable (503)
// ============================================================================

#[tokio::test]
async fn service_unavailable_returns_503() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .add_header("X-Service-Unavailable", "true")
        .json(&json!({
            "query": "query { getShippingRates(params: {shippingType: FCL}) { ... } }"
        }))
        .await;

    assert_eq!(response.status_code(), 503);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"]["code"], 503);
    assert_eq!(body["error"]["message"], "Service temporarily unavailable");
}

// ============================================================================
// GraphQL — Rate limit (429)
// ============================================================================

#[tokio::test]
async fn rate_limit_returns_429() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .add_header("X-Rate-Limit", "true")
        .json(&json!({
            "query": "query { getShippingRates(params: {shippingType: FCL}) { ... } }"
        }))
        .await;

    assert_eq!(response.status_code(), 429);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"]["code"], 429);
    assert_eq!(body["error"]["message"], "Rate limit exceeded");
    assert!(body["error"]["retry_after"].is_number());
}

#[tokio::test]
async fn rate_limit_custom_retry_after() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .add_header("X-Rate-Limit", "true")
        .add_header("X-Rate-Limit-Retry-After", "120")
        .json(&json!({
            "query": "query { getShippingRates(params: {shippingType: FCL}) { ... } }"
        }))
        .await;

    assert_eq!(response.status_code(), 429);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"]["retry_after"], 120);
}

// ============================================================================
// GraphQL — Method not allowed
// ============================================================================

#[tokio::test]
async fn get_on_graphql_returns_405() {
    let server = test_server().await;

    let response = server.get("/graphql").await;
    assert_eq!(response.status_code(), 405);
}

// ============================================================================
// GraphQL — Empty body
// ============================================================================

#[tokio::test]
async fn empty_body_returns_400() {
    let server = test_server().await;

    let response = server.post("/graphql").text("").await;
    assert_eq!(response.status_code(), 400);
    let body: serde_json::Value = response.json();
    let errors = body.get("errors").and_then(|e| e.as_array());
    assert!(errors.is_some());
}

// ============================================================================
// GraphQL — Invalid JSON
// ============================================================================

#[tokio::test]
async fn invalid_json_returns_400() {
    let server = test_server().await;

    let response = server
        .post("/graphql")
        .content_type("application/json")
        .text("not valid json {{{")
        .await;

    assert_eq!(response.status_code(), 400);
}

// ============================================================================
// GraphQL — Variables JSON extraction
// ============================================================================

#[tokio::test]
async fn variables_json_overrides_query_params() {
    let server = test_server().await;

    // Query says FCL but variables say LCL — variables should win
    let response = server
        .post("/graphql")
        .json(&json!({
            "query": "query GetShippingRates($params: GetShippingRatesParams!) { getShippingRates(params: $params) { ... } }",
            "variables": {
                "params": {
                    "shippingType": "LCL",
                    "pointIdFrom": "UAODS",
                    "pointIdTo": "JPTYO",
                    "date": "2026-05-01"
                }
            }
        }))
        .await;

    assert_eq!(response.status_code(), 200);
    let body: serde_json::Value = response.json();
    assert!(body.get("data").is_some());
}

#[tokio::test]
async fn variables_json_with_array_shipping_types() {
    let server = test_server().await;

    // Test that shippingTypes array form is parsed from variables JSON
    let response = server
        .post("/graphql")
        .json(&json!({
            "query": "query GetShippingRates($params: GetShippingRatesParams!) { getShippingRates(params: $params) { ... } }",
            "variables": {
                "params": {
                    "shippingTypes": ["LCL"],
                    "pointIdFrom": "CNSHA",
                    "pointIdTo": "USLAX"
                }
            }
        }))
        .await;

    // Should match the LCL interaction (200) since "LCL" is valid
    assert_eq!(response.status_code(), 200);
}
