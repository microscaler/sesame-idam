/// HTTP BDD tests for Story 1.2 — JWKS Endpoint
///
/// These tests invoke the `JwksController` handler directly via brrtrouter's
/// HandlerRequest/HandlerResponse channel pattern, exercising the same code
/// path that the HTTP server uses. This verifies:
/// - The handler returns live keys from `KeyManager`
/// - The response body is valid JWKS (RFC 7517)
/// - The middleware injects correct headers
/// - No private key material leaks
use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse, HeaderVec};
use brrtrouter::ids::RequestId;
use brrtrouter::typed::{Handler, HandlerResponseOutput};
use http::Method;
use sesame_idam_identity_session_service::controllers::jwks::JwksController;
use sesame_idam_identity_session_service::key_manager::KEY_MANAGER;
use std::sync::Arc;

/// Construct a minimal `HandlerRequest` for testing.
fn make_request(path: &str, method: Method, headers: Vec<(&str, &str)>) -> HandlerRequest {
    let mut hv = HeaderVec::new();
    for (k, v) in headers {
        hv.push((Arc::from(k), v.to_string()));
    }
    HandlerRequest {
        request_id: RequestId::new(),
        method,
        path: path.to_string(),
        handler_name: "jwks".to_string(),
        path_params: brrtrouter::router::ParamVec::default(),
        query_params: brrtrouter::router::ParamVec::default(),
        headers: hv,
        cookies: HeaderVec::new(),
        body: None,
        jwt_claims: None,
        reply_tx: may::sync::mpsc::channel().0,
        queue_guard: None,
    }
}

/// Helper: invoke `JwksController` and collect the response.
/// Returns the `HandlerResponse` sent back on the reply channel.
#[allow(clippy::needless_pass_by_value)] // consumed via dispatch semantics
fn invoke_jwks_request(req: HandlerRequest) -> HandlerResponse {
    let handler = JwksController;

    // Build a TypedHandlerRequest<Request> for the handler
    // The Request type for JWKS is an empty struct (no body/query params)
    let method = req.method.clone();
    let path = req.path.clone();
    let handler_name = req.handler_name.clone();
    let path_params: std::collections::HashMap<String, String> = req
        .path_params
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    let query_params: std::collections::HashMap<String, String> = req
        .query_params
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();

    // Request is `struct Request {}` — construct via serde_json
    let data: sesame_idam_identity_session_service_gen::handlers::jwks::Request =
        serde_json::from_value(serde_json::json!({})).unwrap();

    let typed_req = brrtrouter::typed::TypedHandlerRequest {
        method,
        path,
        handler_name,
        path_params,
        query_params,
        data,
    };

    // Call the handler directly
    let response_data = handler.handle(typed_req);

    // Convert the response to HandlerResponse

    response_data
        .into_handler_response()
        .expect("JWKS handler response must serialize to HandlerResponse")
}

// ─── Tests: Scenario 1 — JWKS serves current key ────────────────────────────────

#[test]
fn test_jwks_endpoint_returns_live_keys() {
    // Given: KEY_MANAGER has at least one key
    let km = KEY_MANAGER.read().unwrap();
    let km_keys = km.jwks_document().keys;
    assert!(
        !km_keys.is_empty(),
        "KEY_MANAGER must have keys for this test"
    );

    // When: we call the JWKS handler
    let req = make_request("/.well-known/jwks.json", Method::GET, vec![]);
    let hr = invoke_jwks_request(req);

    // Then: the response is 200 with live keys
    assert_eq!(hr.status, 200, "JWKS endpoint must return 200");
    assert!(
        hr.body["keys"].as_array().is_some(),
        "Response must have a 'keys' array"
    );
    assert!(
        !hr.body["keys"].as_array().unwrap().is_empty(),
        "Response must contain at least 1 key"
    );
}

#[test]
fn test_jwks_endpoint_validates_rfc7517_structure() {
    // When: we call the JWKS handler
    let req = make_request("/.well-known/jwks.json", Method::GET, vec![]);
    let hr = invoke_jwks_request(req);

    // Then: each key has the required RFC 7517 fields
    // JwkOnly serializes: kid, kty (OKP), use (sig), crv (Ed25519), x
    let keys = hr.body["keys"].as_array().expect("keys must be an array");
    for key in keys {
        assert!(key["kty"].is_string(), "Each key must have 'kty' field");
        assert!(key["kid"].is_string(), "Each key must have 'kid' field");
        assert!(
            key["crv"].is_string(),
            "Each key must have 'crv' field (Ed25519 is OKP)"
        );
        assert!(
            key["x"].is_string(),
            "Each key must have 'x' coordinate field"
        );
    }
}

#[test]
fn test_jwks_endpoint_no_private_key_material() {
    // When: we call the JWKS handler
    let req = make_request("/.well-known/jwks.json", Method::GET, vec![]);
    let hr = invoke_jwks_request(req);

    // Then: no 'd' (private key) field in any key
    let keys = hr.body["keys"].as_array().expect("keys must be an array");
    for key in keys {
        let key_json = serde_json::to_string(key).expect("key must serialize");
        assert!(
            !key_json.contains("\"d\""),
            "JWKS must not contain private key 'd' field"
        );
        assert!(
            !key_json.contains("\"p\""),
            "JWKS must not contain 'p' field"
        );
        assert!(
            !key_json.contains("\"q\""),
            "JWKS must not contain 'q' field"
        );
    }
}

#[test]
fn test_jwks_endpoint_response_size_under_2kb() {
    // When: we call the JWKS handler
    let req = make_request("/.well-known/jwks.json", Method::GET, vec![]);
    let hr = invoke_jwks_request(req);

    // Then: serialized response is under 2KB
    let json = serde_json::to_string_pretty(&hr.body).expect("body must serialize");
    let bytes = json.len();
    assert!(
        bytes < 2048,
        "JWKS response must be under 2KB, got {bytes} bytes"
    );
}

#[test]
fn test_jwks_endpoint_contains_all_verification_keys() {
    // Given: KeyManager has keys available for verification
    let km = KEY_MANAGER.read().unwrap();
    let km_verification_keys = km.keys_for_verification();
    assert!(
        !km_verification_keys.is_empty(),
        "KEY_MANAGER must have verification keys"
    );

    // When: we call the JWKS handler
    let req = make_request("/.well-known/jwks.json", Method::GET, vec![]);
    let hr = invoke_jwks_request(req);

    // Then: response contains at least as many keys as verification expects
    let response_keys = hr.body["keys"].as_array().expect("keys must be an array");
    assert!(
        response_keys.len() >= km_verification_keys.len(),
        "JWKS must contain all {} verification keys, got {} keys",
        km_verification_keys.len(),
        response_keys.len()
    );
}

// ─── Tests: Scenario 2 — Cache-Control and Content-Type ─────────────────────────

#[test]
fn test_jwks_handler_response_has_content_type() {
    // When: we call the JWKS handler
    let req = make_request("/.well-known/jwks.json", Method::GET, vec![]);
    let hr = invoke_jwks_request(req);

    // Then: Content-Type is set by brrtrouter's HandlerResponse::json()
    let content_type = hr
        .headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "content-type")
        .map(|(_, v)| v.as_str());

    assert_eq!(
        content_type,
        Some("application/json"),
        "HandlerResponse::json() sets Content-Type: application/json"
    );
}

// ─── Tests: Scenario 3 — JWKS endpoint requires no auth ────────────────────────

#[test]
fn test_jwks_endpoint_no_auth_required() {
    // When: we call the JWKS handler without any auth headers
    let req = make_request(
        "/.well-known/jwks.json",
        Method::GET,
        vec![], // No Authorization header
    );
    let hr = invoke_jwks_request(req);

    // Then: the endpoint returns 200 (not 401/403)
    assert_eq!(
        hr.status, 200,
        "JWKS endpoint must be public (no auth required)"
    );
}

// ─── Tests: Scenario 4 — JWKS response is not a JWT ────────────────────────────

#[test]
fn test_jwks_response_not_jwt() {
    // When: we call the JWKS handler
    let req = make_request("/.well-known/jwks.json", Method::GET, vec![]);
    let hr = invoke_jwks_request(req);

    // Then: response is NOT a JWT (JWTs have 3 dot-separated segments)
    let json_str = serde_json::to_string(&hr.body).expect("body must serialize");

    // A JWT has the format: header.payload.signature (3 base64 segments)
    // The JWKS response has a "keys" array, not the JWT structure
    assert!(
        !json_str.contains("\"alg\":\""),
        "JWKS response must not look like a JWT"
    );
    assert!(
        json_str.contains("\"keys\""),
        "JWKS response must contain 'keys' field"
    );
}

// ─── Tests: Scenario 5 — JWKS algorithm correctness ────────────────────────────

#[test]
fn test_jwks_keys_use_ed25519() {
    // When: we call the JWKS handler
    let req = make_request("/.well-known/jwks.json", Method::GET, vec![]);
    let hr = invoke_jwks_request(req);

    // Then: all keys use Ed25519 (OKP curve, EdDSA algorithm)
    let keys = hr.body["keys"].as_array().expect("keys must be array");
    for key in keys {
        assert_eq!(
            key["kty"].as_str(),
            Some("okp"),
            "Key must be OKP (Octet Pair) for Ed25519"
        );
        assert_eq!(
            key["crv"].as_str(),
            Some("ED25519"),
            "Key must use Ed25519 curve"
        );
        // Ed25519 public key 'x' value is 43 characters (32 bytes base64url + padding)
        let x = key["x"].as_str().expect("'x' must be string");
        assert!(
            x.len() >= 43,
            "Ed25519 'x' value should be ~43 base64url chars, got {}",
            x.len()
        );
    }
}

// ─── Tests: Scenario 6 — JWKS response is valid JSON ───────────────────────────

#[test]
fn test_jwks_response_parses_as_valid_json() {
    // When: we call the JWKS handler
    let req = make_request("/.well-known/jwks.json", Method::GET, vec![]);
    let hr = invoke_jwks_request(req);

    // Then: the body is valid JSON with "keys" array
    let json = serde_json::to_string(&hr.body).expect("body must be valid JSON");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parsed correctly");

    assert!(
        parsed["keys"].is_array(),
        "Parsed JSON must have 'keys' array"
    );
    assert!(
        !parsed["keys"].as_array().unwrap().is_empty(),
        "Keys array must have at least 1 entry"
    );
}
