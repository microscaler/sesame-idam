//! Epic 11–12 protocol BDD against live Postgres/Redis via in-process handlers.
//!
//! Covers client registry preauth resolution, authorize handler outcomes,
//! and authorization-code mint/redeem/replay (Redis).
//!
//! Skips when Postgres or Redis is unreachable (same pattern as `token_lifecycle`).

use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sha2::{Digest, Sha256};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

use sesame_idam_identity_login_service::controllers::oauth_authorize::{
    self, OauthAuthorizeOutcome,
};
use sesame_idam_identity_login_service::services::client_registry::ClientRegistry;
use sesame_idam_identity_login_service::services::oidc_authorization::{
    mint_authorization_code, redeem_authorization_code, AuthorizationCode,
};
use sesame_idam_identity_login_service_gen::handlers::oauth_authorize::Request as AuthorizeRequest;

static INIT: Once = Once::new();

const FIXTURE_WEB: &str = "acme-web";
const FIXTURE_REDIRECT: &str = "https://app.example.com/auth/callback";

fn postgres_reachable() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    format!("{host}:{port}")
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some()
}

fn redis_reachable() -> bool {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let host_port = url
        .trim_start_matches("redis://")
        .split('/')
        .next()
        .unwrap_or("127.0.0.1:6379");
    host_port
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some()
}

fn configure_db() -> bool {
    if !postgres_reachable() {
        return false;
    }
    INIT.call_once(|| {
        let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
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

fn pkce_pair() -> (String, String) {
    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier.to_string(), challenge)
}

fn authorize_request(
    client_id: &str,
    redirect_uri: &str,
    challenge: &str,
    method: &str,
) -> TypedHandlerRequest<AuthorizeRequest> {
    TypedHandlerRequest {
        method: Method::GET,
        path: "/oauth/authorize".to_string(),
        handler_name: "oauth_authorize".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: AuthorizeRequest {
            client_id: client_id.to_string(),
            response_type: "code".to_string(),
            redirect_uri: redirect_uri.to_string(),
            state: "state-0123456789abcdef".to_string(),
            scope: "openid profile email".to_string(),
            nonce: "nonce-0123456789abcdef".to_string(),
            code_challenge: challenge.to_string(),
            code_challenge_method: method.to_string(),
        },
        jwt_claims: None,
    }
}

#[test]
fn preauth_registry_resolves_fixture_web_without_tenant_guc() {
    if !configure_db() {
        eprintln!("SKIP preauth_registry_resolves: Postgres unreachable");
        return;
    }
    let exec = sesame_idam_database::db();
    let binding = ClientRegistry::resolve_active(FIXTURE_WEB, exec)
        .expect("acme-web must resolve under preauth RLS");
    assert_eq!(binding.tenant_id, "acme");
    assert_eq!(binding.client_id, FIXTURE_WEB);
    assert!(
        binding
            .redirect_uris
            .iter()
            .any(|uri| uri == FIXTURE_REDIRECT),
        "registered login redirect missing: {:?}",
        binding.redirect_uris
    );
    assert!(binding.policy.grants.iter().any(|g| g == "authorization_code"));
}

#[test]
fn preauth_registry_unknown_client_is_unknown() {
    if !configure_db() {
        eprintln!("SKIP preauth_registry_unknown: Postgres unreachable");
        return;
    }
    let exec = sesame_idam_database::db();
    let err = ClientRegistry::resolve_active("definitely-not-a-registered-client", exec)
        .expect_err("unknown client");
    assert_eq!(
        err,
        sesame_idam_identity_login_service::services::client_registry::ClientRegistryError::Unknown
    );
}

#[test]
fn authorize_handler_redirects_valid_pkce_request() {
    if !configure_db() || !redis_reachable() {
        eprintln!("SKIP authorize_handler_redirects: Postgres/Redis unreachable");
        return;
    }
    let (_verifier, challenge) = pkce_pair();
    let outcome = oauth_authorize::handle(authorize_request(
        FIXTURE_WEB,
        FIXTURE_REDIRECT,
        &challenge,
        "S256",
    ));
    match outcome {
        OauthAuthorizeOutcome::Redirect(redirect) => {
            assert_eq!(redirect.status, 302);
            assert!(
                redirect.location.contains("request_id="),
                "expected hosted-auth redirect, got {}",
                redirect.location
            );
            assert!(
                redirect.location.contains("tenant="),
                "hosted-auth redirect must carry tenant: {}",
                redirect.location
            );
            assert!(
                redirect.location.contains("client_id="),
                "hosted-auth redirect must carry client_id: {}",
                redirect.location
            );
        }
        OauthAuthorizeOutcome::Error(err) => {
            panic!("expected redirect, got error {}: {:?}", err.status, err.body);
        }
    }
}

#[test]
fn authorize_handler_rejects_unknown_client() {
    if !configure_db() {
        eprintln!("SKIP authorize_handler_rejects_unknown: Postgres unreachable");
        return;
    }
    let (_verifier, challenge) = pkce_pair();
    let outcome = oauth_authorize::handle(authorize_request(
        "missing-client",
        FIXTURE_REDIRECT,
        &challenge,
        "S256",
    ));
    match outcome {
        OauthAuthorizeOutcome::Error(err) => {
            assert_eq!(err.status, 400);
            assert_eq!(err.body["error"], "invalid_client");
        }
        OauthAuthorizeOutcome::Redirect(redirect) => {
            panic!("expected invalid_client, got redirect {}", redirect.location);
        }
    }
}

#[test]
fn authorize_handler_rejects_unregistered_redirect() {
    if !configure_db() {
        eprintln!("SKIP authorize_handler_rejects_redirect: Postgres unreachable");
        return;
    }
    let (_verifier, challenge) = pkce_pair();
    let outcome = oauth_authorize::handle(authorize_request(
        FIXTURE_WEB,
        "https://attacker.example/callback",
        &challenge,
        "S256",
    ));
    match outcome {
        OauthAuthorizeOutcome::Error(err) => {
            assert_eq!(err.status, 400);
            assert_eq!(err.body["error"], "invalid_request");
        }
        OauthAuthorizeOutcome::Redirect(redirect) => {
            // Some redirect mismatches may surface as error redirect to client —
            // either form is acceptable if error is present.
            assert!(
                redirect.location.contains("error="),
                "unexpected success redirect {}",
                redirect.location
            );
        }
    }
}

#[test]
fn authorization_code_redeem_is_single_use_and_binding_checked() {
    if !redis_reachable() {
        eprintln!("SKIP authorization_code_redeem: Redis unreachable");
        return;
    }
    let (verifier, challenge) = pkce_pair();
    let code = AuthorizationCode {
        client_id: "client-a".into(),
        tenant_id: "acme".into(),
        application_id: "frontend".into(),
        redirect_uri: "https://client.example/callback".into(),
        user_id: "00000000-0000-4000-8000-000000000099".into(),
        scopes: vec!["openid".into()],
        nonce: "nonce-0123456789abcdef".into(),
        code_challenge: challenge,
        auth_time: chrono::Utc::now().timestamp(),
        created_at: chrono::Utc::now().timestamp(),
    };
    let value = mint_authorization_code(&code).expect("mint code");

    assert!(
        redeem_authorization_code(
            &value,
            "client-b",
            "https://client.example/callback",
            &verifier
        )
        .is_none(),
        "cross-client redeem must fail and burn the code"
    );
    assert!(
        redeem_authorization_code(
            &value,
            "client-a",
            "https://client.example/callback",
            &verifier
        )
        .is_none(),
        "replay after failed binding must also fail (code burned)"
    );

    let value2 = mint_authorization_code(&code).expect("mint second code");
    let redeemed = redeem_authorization_code(
        &value2,
        "client-a",
        "https://client.example/callback",
        &verifier,
    )
    .expect("valid redeem");
    assert_eq!(redeemed.user_id, code.user_id);
    assert!(
        redeem_authorization_code(
            &value2,
            "client-a",
            "https://client.example/callback",
            &verifier
        )
        .is_none(),
        "second redeem is replay and must fail"
    );
}
