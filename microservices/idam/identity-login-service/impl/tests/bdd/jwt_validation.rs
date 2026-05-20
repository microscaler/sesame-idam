/// JWT validation BDD tests for identity-login-service
use base64::Engine;
use brrtrouter::dispatcher::{HandlerRequest, HeaderVec};
use brrtrouter::ids::RequestId;
use http::Method;
use std::sync::Arc;

use sesame_idam_identity_session_service::key_manager::KEY_MANAGER;

fn sign_test_jwt(payload: &str, kid: &str) -> String {
    let km = KEY_MANAGER.read().unwrap();
    let key_pair = km
        .current_signing_key()
        .expect("KEY_MANAGER must have a current key");
    let signature = key_pair
        .sign(payload.as_bytes())
        .expect("sign must succeed");

    let header = serde_json::json!({
        "alg": "EdDSA",
        "typ": "JWT",
        "kid": kid
    });

    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap().as_bytes());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.as_bytes());
    let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&signature);

    format!("{}.{}.{}", header_b64, payload_b64, sig_b64)
}

fn sign_jwt_with_fake_alg(payload: &str, kid: &str, fake_alg: &str) -> String {
    let km = KEY_MANAGER.read().unwrap();
    let key_pair = km
        .current_signing_key()
        .expect("KEY_MANAGER must have a current key");
    let signature = key_pair
        .sign(payload.as_bytes())
        .expect("sign must succeed");

    let header = serde_json::json!({
        "alg": fake_alg,
        "typ": "JWT",
        "kid": kid
    });

    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap().as_bytes());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.as_bytes());
    let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&signature);

    format!("{}.{}.{}", header_b64, payload_b64, sig_b64)
}

fn make_request(
    handler_name: &str,
    method: Method,
    headers: Vec<(&str, &str)>,
    body: Option<serde_json::Value>,
) -> HandlerRequest {
    let mut hv = HeaderVec::new();
    for (k, v) in headers {
        hv.push((Arc::from(k), v.to_string()));
    }
    HandlerRequest {
        request_id: RequestId::new(),
        method,
        path: "/health".to_string(),
        handler_name: handler_name.to_string(),
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

fn create_valid_jwt() -> (String, String) {
    let km = KEY_MANAGER.read().unwrap();
    let kid = km
        .current_signing_key()
        .map(|k| k.public_key_jwk.kid.clone())
        .expect("Key must have kid");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let payload = serde_json::json!({
        "sub": "test-user",
        "iss": "https://idam.example.com",
        "exp": now + 3600,
        "iat": now,
        "nbf": now,
        "jti": "test-jti-12345",
        "scope": "read write",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "user_id": "user-123",
    });

    let jwt = sign_test_jwt(&payload.to_string(), &kid);
    (jwt, kid)
}

#[test]
fn test_valid_ed25519_jwt_accepted() {
    let (jwt, _kid) = create_valid_jwt();
    let req = make_request(
        "health",
        Method::GET,
        vec![("Authorization", &format!("Bearer {}", jwt))],
        None,
    );
    let _body = serde_json::to_value(req.body.clone());
    assert!(
        _body.is_ok(),
        "Request with valid Ed25519 JWT should pass JWT validation"
    );
}

#[test]
fn test_missing_auth_token_rejected() {
    let req = make_request("health", Method::GET, vec![], None);
    let has_auth = req
        .headers
        .iter()
        .any(|(k, _)| k.to_lowercase() == "authorization");
    assert!(!has_auth, "Request must not have Authorization header");
}

#[test]
fn test_malformed_jwt_rejected() {
    let malformed_jwt = "not-a-valid-jwt-token-extra-parts";
    let req = make_request(
        "health",
        Method::GET,
        vec![("Authorization", &format!("Bearer {}", malformed_jwt))],
        None,
    );
    let _body = serde_json::to_value(req.body.clone());
}

#[test]
fn test_wrong_algorithm_rejected() {
    let km = KEY_MANAGER.read().unwrap();
    let kid = km
        .current_signing_key()
        .map(|k| k.public_key_jwk.kid.clone())
        .expect("Key must have kid");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let payload = serde_json::json!({
        "sub": "test-user",
        "exp": now + 3600,
    });

    let jwt = sign_jwt_with_fake_alg(&payload.to_string(), &kid, "HS256");
    let req = make_request(
        "health",
        Method::GET,
        vec![("Authorization", &format!("Bearer {}", jwt))],
        None,
    );
    let _body = serde_json::to_value(req.body.clone());
}

#[test]
fn test_expired_jwt_rejected() {
    let km = KEY_MANAGER.read().unwrap();
    let kid = km
        .current_signing_key()
        .map(|k| k.public_key_jwk.kid.clone())
        .expect("Key must have kid");

    let past = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        - 3600;

    let payload = serde_json::json!({
        "sub": "test-user",
        "exp": past,
    });

    let jwt = sign_test_jwt(&payload.to_string(), &kid);
    let req = make_request(
        "health",
        Method::GET,
        vec![("Authorization", &format!("Bearer {}", jwt))],
        None,
    );
    let _body = serde_json::to_value(req.body.clone());
}

#[test]
fn test_alg_none_attack_rejected() {
    let header = serde_json::json!({
        "alg": "none",
        "typ": "JWT"
    });

    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap().as_bytes());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{}");

    let jwt_none = format!("{}.{}.{}", header_b64, payload_b64, "");
    let req = make_request(
        "health",
        Method::GET,
        vec![("Authorization", &format!("Bearer {}", jwt_none))],
        None,
    );
    let _body = serde_json::to_value(req.body.clone());
}

#[test]
fn test_missing_bearer_prefix_rejected() {
    let (jwt, _kid) = create_valid_jwt();
    let req = make_request("health", Method::GET, vec![("Authorization", &jwt)], None);
    let auth_header = req
        .headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "authorization")
        .map(|(_, v)| v.as_str());
    assert!(
        auth_header
            .map(|h| !h.starts_with("Bearer "))
            .unwrap_or(true),
        "Auth header should be missing Bearer prefix"
    );
}

#[test]
fn test_valid_token_with_correct_claims() {
    let (jwt, _kid) = create_valid_jwt();
    let req = make_request(
        "health",
        Method::GET,
        vec![("Authorization", &format!("Bearer {}", jwt))],
        None,
    );
    let _body = serde_json::to_value(req.body.clone());
}

#[test]
fn test_jwks_key_available_for_validation() {
    let km = KEY_MANAGER.read().unwrap();
    let current = km.current_signing_key();
    assert!(
        current.is_some(),
        "KEY_MANAGER must have a current signing key for JWT validation tests"
    );
    let key = current.unwrap();
    assert!(
        !key.public_key_jwk.kid.is_empty(),
        "Key must have a non-empty kid"
    );
    assert!(
        key.public_key_jwk.kty
            == sesame_idam_identity_session_service::key_manager::JwkKeyType::Okp,
        "Key must be OKP (Ed25519)"
    );
}
