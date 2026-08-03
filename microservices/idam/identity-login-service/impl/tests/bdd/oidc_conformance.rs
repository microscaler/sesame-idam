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
        tenant_id: "acme".into(),
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

fn handle_token(
    grant_type: &str,
    client_id: &str,
    refresh_token: Option<&str>,
) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    use sesame_idam_identity_login_service::controllers::oauth_token;
    use sesame_idam_identity_login_service_gen::handlers::oauth_token::Request as TokenRequest;

    oauth_token::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/oauth/token".to_string(),
        handler_name: "oauth_token".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: TokenRequest {
            client_id: Some(client_id.into()),
            client_secret: None,
            code: None,
            code_verifier: None,
            grant_type: grant_type.into(),
            redirect_uri: None,
            refresh_token: refresh_token.map(str::to_string),
            scope: None,
            authorization: None,
        },
        jwt_claims: None,
    })
}

fn sample_access_claims(
    iss: &str,
    aud: &str,
    tenant: &str,
    exp_offset: i64,
) -> sesame_common::AccessClaims {
    use sesame_common::{AccessClaimsBuilder, SesameAuthzClaimsBuilder};
    let now = chrono::Utc::now().timestamp();
    let sx = SesameAuthzClaimsBuilder::new()
        .tenant(tenant)
        .portal("frontend")
        .roles(vec!["user".into()])
        .permissions(vec![])
        .build()
        .expect("sx");
    AccessClaimsBuilder::new()
        .iss(iss)
        .sub("00000000-0000-4000-8000-0000000000aa")
        .aud(vec![aud.into()])
        .client_id("fixture-public-client")
        .scope("openid")
        .exp(now + exp_offset)
        .nbf(now - 10)
        .iat(now)
        .jti(uuid::Uuid::new_v4().to_string())
        .ver(1)
        .sid(uuid::Uuid::new_v4().to_string())
        .tenant_id(tenant)
        .user_id("00000000-0000-4000-8000-0000000000aa")
        .user_type("customer")
        .sx(sx)
        .build()
        .expect("claims")
}

#[test]
fn conformance_access_token_forgery_set() {
    let cases = protocol_cases();
    let signer = sesame_common::jwt::Ed25519Signer::from_env_or_generate().expect("signer");
    let iss = "https://idam.example.com";
    let aud = "identity-login";

    // valid_no_org
    let valid = signer
        .sign_access_claims(&sample_access_claims(iss, aud, "acme", 300))
        .expect("sign valid");
    assert!(
        sesame_common::verify_access_token(&signer, &valid, Some("acme")).is_ok(),
        "access-valid-no-org must accept"
    );

    // wrong_issuer
    let wrong_iss = signer
        .sign_access_claims(&sample_access_claims(
            cases["access_token"]["wrong_issuer"]["mutate_claim"]["iss"]
                .as_str()
                .unwrap(),
            aud,
            "acme",
            300,
        ))
        .expect("sign");
    assert!(
        sesame_common::verify_access_token(&signer, &wrong_iss, None).is_err(),
        "access-wrong-issuer must reject"
    );

    // wrong_audience
    let wrong_aud = signer
        .sign_access_claims(&sample_access_claims(
            iss,
            cases["access_token"]["wrong_audience"]["mutate_claim"]["aud"][0]
                .as_str()
                .unwrap(),
            "acme",
            300,
        ))
        .expect("sign");
    assert!(
        sesame_common::verify_access_token(&signer, &wrong_aud, None).is_err(),
        "access-wrong-audience must reject"
    );

    // alg=none
    let parts: Vec<&str> = valid.split('.').collect();
    let none_header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"at+jwt"}"#);
    let forged = format!("{none_header}.{}.{}", parts[1], parts[2]);
    assert!(
        sesame_common::verify_access_token(&signer, &forged, None).is_err(),
        "access-alg-none must reject"
    );

    // expired
    let expired = signer
        .sign_access_claims(&sample_access_claims(iss, aud, "acme", -3600))
        .expect("sign expired");
    assert!(
        sesame_common::verify_access_token(&signer, &expired, None).is_err(),
        "access-expired must reject"
    );

    // tenant mismatch
    let tenant_tok = signer
        .sign_access_claims(&sample_access_claims(iss, aud, "acme", 300))
        .expect("sign");
    assert!(
        sesame_common::verify_access_token(&signer, &tenant_tok, Some("other-tenant")).is_err(),
        "access-tenant-mismatch must reject"
    );
}

#[test]
fn conformance_refresh_rotation_replay_and_cross_client() {
    if !redis_available() {
        eprintln!("SKIP conformance refresh: Redis unavailable");
        return;
    }
    let cases = protocol_cases();
    if !infra_available()
        || !fixture_ready(cases["refresh"]["rotation"]["client_id"].as_str().unwrap())
    {
        eprintln!("SKIP conformance refresh: Postgres/fixture client missing");
        return;
    }

    let client_a = cases["refresh"]["rotation"]["client_id"].as_str().unwrap();
    let client_b = cases["refresh"]["cross_client"]["mutate"]["client_id"]
        .as_str()
        .unwrap();

    let issued = sesame_idam_identity_login_service::services::token_issuer::issue_tokens_for_client(
        "00000000-0000-4000-8000-000000000099",
        "acme",
        "frontend",
        client_a,
        vec![],
        vec![],
        "user",
        None,
        "openid",
    )
    .expect("issue tokens");

    let first = handle_token("refresh_token", client_a, Some(&issued.refresh_token));
    assert_eq!(first.status, 200, "refresh-rotation must succeed: {:?}", first.body);
    assert!(
        first.body["refresh_token"].as_str().is_some_and(|t| !t.is_empty()),
        "expect_new_refresh_token"
    );
    let new_refresh = first.body["refresh_token"].as_str().unwrap().to_string();
    assert_ne!(new_refresh, issued.refresh_token);

    let replay = handle_token("refresh_token", client_a, Some(&issued.refresh_token));
    assert_eq!(
        replay.body["error"],
        cases["refresh"]["replay"]["expected_error"].as_str().unwrap(),
        "refresh-replay must fail: {:?}",
        replay.body
    );

    let issued2 = sesame_idam_identity_login_service::services::token_issuer::issue_tokens_for_client(
        "00000000-0000-4000-8000-000000000099",
        "acme",
        "frontend",
        client_a,
        vec![],
        vec![],
        "user",
        None,
        "openid",
    )
    .expect("issue2");
    let cross = handle_token("refresh_token", client_b, Some(&issued2.refresh_token));
    assert_eq!(
        cross.body["error"],
        cases["refresh"]["cross_client"]["expected_error"]
            .as_str()
            .unwrap(),
        "refresh-cross-client must fail: {:?}",
        cross.body
    );
}

#[test]
fn conformance_userinfo_sub_matches_token_subject() {
    let cases = protocol_cases();
    assert_eq!(cases["userinfo"]["substitution"]["accept"], false);
    assert_eq!(cases["userinfo"]["valid"]["accept"], true);

    let signer = sesame_common::jwt::Ed25519Signer::from_env_or_generate().expect("signer");
    let user_a = "00000000-0000-4000-8000-0000000000aa";
    let user_b = "00000000-0000-4000-8000-0000000000bb";
    let mut claims = sample_access_claims("https://idam.example.com", "identity-login", "acme", 300);
    claims.sub = user_a.into();
    claims.user_id = user_a.into();
    let token = signer.sign_access_claims(&claims).expect("sign");
    let verified = sesame_common::verify_access_token(&signer, &token, Some("acme")).expect("verify");
    assert_eq!(verified.sub, user_a, "userinfo-valid: sub from token");
    assert_ne!(
        verified.sub, user_b,
        "userinfo-substitution: token for A must not yield B"
    );
}

#[test]
fn conformance_metadata_fixture_contract() {
    let cases = protocol_cases();
    let meta = &cases["metadata"]["valid"];
    for key in meta["required_keys"].as_array().unwrap() {
        assert!(key.as_str().is_some());
    }
    assert_eq!(meta["response_types_supported"][0], "code");
    assert_eq!(meta["code_challenge_methods_supported"][0], "S256");
    for grant in meta["forbidden_grants"].as_array().unwrap() {
        assert!(matches!(grant.as_str(), Some("implicit") | Some("password")));
    }
    assert_eq!(cases["metadata"]["issuer_mismatch"]["accept"], false);
    assert_eq!(cases["jwks"]["unknown_kid"]["accept"], false);
    assert_eq!(cases["jwks"]["wrong_kty"]["accept"], false);
}

#[test]
fn conformance_redaction_gate_from_manifest() {
    let path = repo_root().join("conformance/oidc-v1/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let fields = sesame_common::redacted_field_names(Some(&manifest));
    assert!(fields.contains(&"access_token".into()));
    assert!(fields.contains(&"code_verifier".into()));

    let mut sample = serde_json::json!({
        "event": "oauth_token",
        "access_token": "secret-token-value",
        "refresh_token": "secret-refresh",
        "id_token": "secret-id",
        "code": "secret-code",
        "code_verifier": "secret-verifier",
        "client_secret": "secret-client",
        "grant_type": "authorization_code"
    });
    assert!(sesame_common::assert_no_redacted_fields(&sample, &fields).is_err());
    sesame_common::redact_sensitive_object(&mut sample, &fields);
    sesame_common::assert_no_redacted_fields(&sample, &fields)
        .expect("after redaction, no credential fields may remain");
    assert_eq!(sample["grant_type"], "authorization_code");
}
