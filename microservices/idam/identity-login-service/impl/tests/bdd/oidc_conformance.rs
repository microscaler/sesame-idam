//! Epic 14 conformance fixtures — machine-readable protocol negatives/positives.
//!
//! Loads `conformance/oidc-v1/protocol-cases.json` and exercises the running
//! authorize/token handlers (in-process). Skips when infra or fixture clients
//! are missing.

use std::collections::HashMap;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Once;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sha2::{Digest, Sha256};

use sesame_idam_identity_login_service::controllers::oauth_authorize::{
    self, OauthAuthorizeOutcome,
};
use sesame_idam_identity_login_service::services::oidc_authorization::{
    mint_authorization_code, redeem_authorization_code, AuthorizationCode,
};
use sesame_idam_identity_login_service_gen::handlers::oauth_authorize::Request as AuthorizeRequest;

static INIT: Once = Once::new();

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .expect("repo root")
}

fn protocol_cases() -> serde_json::Value {
    let path = repo_root().join("conformance/oidc-v1/protocol-cases.json");
    let raw = std::fs::read_to_string(&path).expect("read protocol-cases.json");
    serde_json::from_str(&raw).expect("parse protocol-cases.json")
}

fn infra_available() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let pg = format!("{host}:{port}")
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some();
    if !pg {
        return false;
    }
    INIT.call_once(|| {
        std::env::set_var("DB_POOL_MAX", "2");
        std::env::set_var("DB_HOST", &host);
        std::env::set_var("DB_PORT", &port);
        std::env::set_var(
            "DB_USER",
            std::env::var("TEST_DB_USER").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
        std::env::set_var(
            "DB_PASS",
            std::env::var("TEST_DB_PASS")
                .unwrap_or_else(|_| "dev_password_change_in_prod".to_string()),
        );
        std::env::set_var(
            "DB_NAME",
            std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
    });
    true
}

fn fixture_ready(client_id: &str) -> bool {
    sesame_idam_identity_login_service::services::client_registry::ClientRegistry::resolve_active(
        client_id,
        sesame_idam_database::db(),
    )
    .is_ok()
}

fn authorize_from_valid(cases: &serde_json::Value, mutate: Option<&serde_json::Value>) -> AuthorizeRequest {
    let valid = &cases["authorization"]["valid"];
    let mut req = AuthorizeRequest {
        client_id: valid["client_id"].as_str().unwrap().to_string(),
        response_type: valid["response_type"].as_str().unwrap().to_string(),
        redirect_uri: valid["redirect_uri"].as_str().unwrap().to_string(),
        state: valid["state"].as_str().unwrap().to_string(),
        scope: valid["scope"].as_str().unwrap().to_string(),
        nonce: valid["nonce"].as_str().unwrap().to_string(),
        code_challenge: valid["code_challenge"].as_str().unwrap().to_string(),
        code_challenge_method: valid["code_challenge_method"].as_str().unwrap().to_string(),
    };
    if let Some(m) = mutate {
        if let Some(v) = m.get("redirect_uri").and_then(|v| v.as_str()) {
            req.redirect_uri = v.to_string();
        }
        if let Some(v) = m.get("code_challenge_method").and_then(|v| v.as_str()) {
            req.code_challenge_method = v.to_string();
        }
        if let Some(v) = m.get("client_id").and_then(|v| v.as_str()) {
            req.client_id = v.to_string();
        }
    }
    req
}

fn handle_authorize(req: AuthorizeRequest) -> OauthAuthorizeOutcome {
    oauth_authorize::handle(TypedHandlerRequest {
        method: Method::GET,
        path: "/oauth/authorize".to_string(),
        handler_name: "oauth_authorize".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: req,
        jwt_claims: None,
    })
}

#[test]
fn conformance_manifest_lists_required_families() {
    let path = repo_root().join("conformance/oidc-v1/manifest.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(doc["algorithm"], "EdDSA");
    let ids: Vec<&str> = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["id"].as_str())
        .collect();
    for required in [
        "authorization-valid-public-pkce",
        "authorization-redirect-prefix",
        "authorization-pkce-plain",
        "code-replay",
        "code-cross-client",
        "userinfo-substitution",
        "metadata-valid",
    ] {
        assert!(ids.contains(&required), "manifest missing {required}");
    }
}

#[test]
fn conformance_authorization_redirect_prefix_rejected() {
    if !infra_available() {
        eprintln!("SKIP conformance redirect prefix: no Postgres");
        return;
    }
    let cases = protocol_cases();
    if !fixture_ready(cases["authorization"]["valid"]["client_id"].as_str().unwrap()) {
        eprintln!("SKIP conformance redirect prefix: fixture client missing");
        return;
    }
    let mutate = &cases["authorization"]["invalid_redirect_prefix"]["mutate"];
    let expected = cases["authorization"]["invalid_redirect_prefix"]["expected_error"]
        .as_str()
        .unwrap();
    match handle_authorize(authorize_from_valid(&cases, Some(mutate))) {
        OauthAuthorizeOutcome::Error(err) => {
            assert_eq!(err.body["error"], expected);
        }
        OauthAuthorizeOutcome::Redirect(redirect) => {
            assert!(
                redirect.location.contains("error="),
                "expected error redirect, got {}",
                redirect.location
            );
        }
    }
}

#[test]
fn conformance_authorization_pkce_plain_rejected() {
    if !infra_available() {
        eprintln!("SKIP conformance pkce plain: no Postgres");
        return;
    }
    let cases = protocol_cases();
    if !fixture_ready(cases["authorization"]["valid"]["client_id"].as_str().unwrap()) {
        eprintln!("SKIP conformance pkce plain: fixture client missing");
        return;
    }
    let mutate = &cases["authorization"]["invalid_plain_pkce"]["mutate"];
    match handle_authorize(authorize_from_valid(&cases, Some(mutate))) {
        OauthAuthorizeOutcome::Error(err) => {
            assert_eq!(err.body["error"], "invalid_request");
        }
        OauthAuthorizeOutcome::Redirect(redirect) => {
            assert!(
                redirect.location.contains("error="),
                "plain PKCE must fail: {}",
                redirect.location
            );
        }
    }
}

#[test]
fn conformance_authorization_valid_public_pkce_redirects() {
    if !infra_available() || !redis_available() {
        eprintln!("SKIP conformance valid pkce: Postgres/Redis unavailable");
        return;
    }
    let cases = protocol_cases();
    if !fixture_ready(cases["authorization"]["valid"]["client_id"].as_str().unwrap()) {
        eprintln!("SKIP conformance valid pkce: fixture client missing");
        return;
    }
    match handle_authorize(authorize_from_valid(&cases, None)) {
        OauthAuthorizeOutcome::Redirect(redirect) => {
            assert!(
                redirect.location.contains("request_id="),
                "expected hosted-auth redirect: {}",
                redirect.location
            );
        }
        OauthAuthorizeOutcome::Error(err) => {
            panic!("valid public PKCE should redirect: {} {:?}", err.status, err.body);
        }
    }
}

#[test]
fn conformance_code_replay_and_cross_client() {
    if !redis_available() {
        eprintln!("SKIP conformance code replay: Redis unavailable");
        return;
    }
    let cases = protocol_cases();
    let valid = &cases["token"]["valid_code"];
    let verifier = valid["code_verifier"].as_str().unwrap();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let client_a = valid["client_id"].as_str().unwrap();
    let client_b = cases["token"]["cross_client"]["mutate"]["client_id"]
        .as_str()
        .unwrap();
    let redirect = valid["redirect_uri"].as_str().unwrap();

    let code = AuthorizationCode {
        client_id: client_a.into(),
        tenant_id: "hauliage".into(),
        application_id: "frontend".into(),
        redirect_uri: redirect.into(),
        user_id: "00000000-0000-4000-8000-000000000099".into(),
        scopes: vec!["openid".into()],
        nonce: "nonce-0123456789abcdef".into(),
        code_challenge: challenge,
        auth_time: chrono::Utc::now().timestamp(),
        created_at: chrono::Utc::now().timestamp(),
    };
    let value = mint_authorization_code(&code).expect("mint");

    assert!(
        redeem_authorization_code(&value, client_b, redirect, verifier).is_none(),
        "code-cross-client must fail"
    );
    assert!(
        redeem_authorization_code(&value, client_a, redirect, verifier).is_none(),
        "code-replay after burn must fail"
    );

    let value2 = mint_authorization_code(&code).expect("mint2");
    assert!(
        redeem_authorization_code(&value2, client_a, redirect, verifier).is_some(),
        "first redeem must succeed"
    );
    assert!(
        redeem_authorization_code(&value2, client_a, redirect, verifier).is_none(),
        "code-replay must fail"
    );
}

fn redis_available() -> bool {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    redis::Client::open(url.as_str())
        .ok()
        .and_then(|c| c.get_connection().ok())
        .is_some()
}
