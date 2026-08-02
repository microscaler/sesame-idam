//! Interactive OIDC Authorization Code + PKCE round-trip (in-process).
//!
//! authorize → password login → authorize/complete → token → userinfo
//! Uses `fixture-public-client` (Epic 14 seed) so token exchange needs no secret.

use std::collections::HashMap;
use std::net::TcpStream;
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
use sesame_idam_identity_login_service::controllers::oauth_authorize_complete::{
    self, OauthAuthorizeCompleteOutcome,
};
use sesame_idam_identity_login_service::controllers::oauth_token;
use sesame_idam_identity_login_service::controllers::oauth_userinfo;
use sesame_idam_identity_login_service::controllers::{auth_login, auth_register};
use sesame_idam_identity_login_service_gen::handlers::auth_login::Request as LoginRequest;
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterRequest;
use sesame_idam_identity_login_service_gen::handlers::oauth_authorize::Request as AuthorizeRequest;
use sesame_idam_identity_login_service_gen::handlers::oauth_authorize_complete::Request as CompleteRequest;
use sesame_idam_identity_login_service_gen::handlers::oauth_token::Request as TokenRequest;
use sesame_idam_identity_login_service_gen::handlers::oauth_userinfo::Request as UserinfoRequest;

use crate::common::{ensure_active_tenant, HAULIAGE_TENANT, HAULIAGE_WEB_CLIENT};

static INIT: Once = Once::new();

const FIXTURE_CLIENT: &str = "fixture-public-client";
const FIXTURE_REDIRECT: &str = "https://client.example/callback";
const DEMO_EMAIL: &str = "owner@hauliage.dev";
const DEMO_PASSWORD: &str = "SecureP@ss123!";

fn infra_available() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let pg = format!("{host}:{port}")
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some();
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let redis_ok = redis::Client::open(redis_url.as_str())
        .ok()
        .and_then(|c| c.get_connection().ok())
        .is_some();
    if !pg || !redis_ok {
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
        std::env::set_var("REDIS_URL", &redis_url);
    });
    true
}

fn fixture_client_ready() -> bool {
    let exec = sesame_idam_database::db();
    sesame_idam_identity_login_service::services::client_registry::ClientRegistry::resolve_active(
        FIXTURE_CLIENT,
        exec,
    )
    .is_ok()
}

fn pkce_pair() -> (String, String) {
    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier.to_string(), challenge)
}

fn decode_jwt_payload(token: &str) -> serde_json::Value {
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "compact JWT");
    let bytes = URL_SAFE_NO_PAD.decode(parts[1]).expect("payload b64");
    serde_json::from_slice(&bytes).expect("payload json")
}

fn query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=')?;
        if k == key {
            return Some(v.to_string());
        }
    }
    None
}

#[test]
fn interactive_pkce_authorize_login_complete_token_userinfo() {
    if !infra_available() {
        eprintln!("SKIP interactive_pkce: Postgres/Redis unavailable");
        return;
    }
    if !fixture_client_ready() {
        eprintln!(
            "SKIP interactive_pkce: seed fixture-public-client \
             (20260802220000_oidc_fixture_public_clients.sql)"
        );
        return;
    }
    ensure_active_tenant(HAULIAGE_TENANT);

    let (verifier, challenge) = pkce_pair();
    let state = "state-0123456789abcdef";
    let nonce = "nonce-0123456789abcdef";

    // 1) Authorize → hosted auth request_id
    let authorize = oauth_authorize::handle(TypedHandlerRequest {
        method: Method::GET,
        path: "/oauth/authorize".to_string(),
        handler_name: "oauth_authorize".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: AuthorizeRequest {
            client_id: FIXTURE_CLIENT.to_string(),
            response_type: "code".to_string(),
            redirect_uri: FIXTURE_REDIRECT.to_string(),
            state: state.to_string(),
            scope: "openid profile email".to_string(),
            nonce: nonce.to_string(),
            code_challenge: challenge,
            code_challenge_method: "S256".to_string(),
        },
        jwt_claims: None,
    });
    let OauthAuthorizeOutcome::Redirect(hosted) = authorize else {
        panic!("authorize should redirect to hosted auth");
    };
    assert!(hosted.location.contains("request_id="));
    assert!(hosted.location.contains("tenant=hauliage"));
    assert!(hosted.location.contains(&format!("client_id={FIXTURE_CLIENT}")));
    let request_id = query_param(&hosted.location, "request_id").expect("request_id");

    // 2) Password login (demo user or freshly registered)
    let mut login = auth_login::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login".to_string(),
        handler_name: "auth_login".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: LoginRequest {
            client_id: HAULIAGE_WEB_CLIENT.to_string(),
            email: DEMO_EMAIL.to_string(),
            organization_id: None,
            password: DEMO_PASSWORD.to_string(),
            x_tenant_id: Some(HAULIAGE_TENANT.to_string()),
        },
        jwt_claims: None,
    });
    if login.status != 200 {
        let email = format!("oidc_{}@example.com", uuid::Uuid::new_v4().simple());
        let reg = auth_register::handle(TypedHandlerRequest {
            method: Method::POST,
            path: "/auth/register".to_string(),
            handler_name: "auth_register".to_string(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            data: RegisterRequest {
                email: email.clone(),
                first_name: None,
                last_name: None,
                password: DEMO_PASSWORD.to_string(),
                phone: None,
                username: None,
                x_tenant_id: HAULIAGE_TENANT.to_string(),
            },
            jwt_claims: None,
        });
        assert_eq!(reg.status, 201, "register fallback: {:?}", reg.body);
        login = auth_login::handle(TypedHandlerRequest {
            method: Method::POST,
            path: "/auth/login".to_string(),
            handler_name: "auth_login".to_string(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            data: LoginRequest {
                client_id: HAULIAGE_WEB_CLIENT.to_string(),
                email,
                organization_id: None,
                password: DEMO_PASSWORD.to_string(),
                x_tenant_id: Some(HAULIAGE_TENANT.to_string()),
            },
            jwt_claims: None,
        });
    }
    assert_eq!(login.status, 200, "login: {:?}", login.body);
    let access = login.body["access_token"].as_str().unwrap().to_string();
    let user_id = login.body["user_id"].as_str().unwrap().to_string();
    let claims = decode_jwt_payload(&access);

    // 3) Complete → RP redirect with code
    let complete = oauth_authorize_complete::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/oauth/authorize/complete".to_string(),
        handler_name: "oauth_authorize_complete".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: CompleteRequest {
            request_id: request_id.clone(),
        },
        jwt_claims: Some(claims),
    });
    let OauthAuthorizeCompleteOutcome::Redirect(rp) = complete else {
        panic!("authorize/complete should redirect to RP with code");
    };
    assert!(rp.location.starts_with(FIXTURE_REDIRECT));
    let code = query_param(&rp.location, "code").expect("code");
    assert_eq!(query_param(&rp.location, "state").as_deref(), Some(state));

    // 4) Token exchange (public client + PKCE)
    let token = oauth_token::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/oauth/token".to_string(),
        handler_name: "oauth_token".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: TokenRequest {
            client_id: Some(FIXTURE_CLIENT.to_string()),
            client_secret: None,
            code: Some(code.clone()),
            code_verifier: Some(verifier.clone()),
            grant_type: "authorization_code".to_string(),
            redirect_uri: Some(FIXTURE_REDIRECT.to_string()),
            refresh_token: None,
            scope: None,
            authorization: None,
        },
        jwt_claims: None,
    });
    assert_eq!(token.status, 200, "token: {:?}", token.body);
    assert!(!token.body["access_token"].as_str().unwrap_or("").is_empty());
    assert!(!token.body["id_token"].as_str().unwrap_or("").is_empty());
    assert_eq!(token.body["token_type"], "Bearer");
    let oidc_access = token.body["access_token"].as_str().unwrap().to_string();
    let oidc_claims = decode_jwt_payload(&oidc_access);
    assert_eq!(oidc_claims["sub"], user_id);

    // Replay must fail closed
    let replay = oauth_token::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/oauth/token".to_string(),
        handler_name: "oauth_token".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: TokenRequest {
            client_id: Some(FIXTURE_CLIENT.to_string()),
            client_secret: None,
            code: Some(code),
            code_verifier: Some(verifier),
            grant_type: "authorization_code".to_string(),
            redirect_uri: Some(FIXTURE_REDIRECT.to_string()),
            refresh_token: None,
            scope: None,
            authorization: None,
        },
        jwt_claims: None,
    });
    assert_eq!(replay.status, 400, "replay: {:?}", replay.body);
    assert_eq!(replay.body["error"], "invalid_grant");

    // 5) UserInfo
    let mut userinfo_claims = oidc_claims.clone();
    if userinfo_claims.get("scope").is_none() {
        userinfo_claims["scope"] = serde_json::json!("openid profile email");
    }
    let userinfo = oauth_userinfo::handle(TypedHandlerRequest {
        method: Method::GET,
        path: "/oauth/userinfo".to_string(),
        handler_name: "oauth_userinfo".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: UserinfoRequest {},
        jwt_claims: Some(userinfo_claims),
    });
    assert_eq!(userinfo.status, 200, "userinfo: {:?}", userinfo.body);
    assert_eq!(userinfo.body["sub"], user_id);
}
