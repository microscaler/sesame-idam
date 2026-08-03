//! Epic 11–13 live-API BDD against the running north-south Sesame surface.
//!
//! Hits public hosts (not in-cluster DNS):
//! - `https://id.sesameidentity.dev.local` — discovery + JWKS
//! - `https://auth.sesameidentity.dev.local` — `/oauth/authorize`
//! - `https://api.sesameidentity.dev.local` — `/oauth/token`, `/oauth/userinfo`
//!
//! Skips when the issuer host is unreachable so CI without the cluster still passes.

use std::net::{SocketAddr, ToSocketAddrs, TcpStream};
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

const ID_BASE: &str = "https://id.sesameidentity.dev.local";
const AUTH_BASE: &str = "https://auth.sesameidentity.dev.local";
const API_BASE: &str = "https://api.sesameidentity.dev.local";

/// Seeded confidential SPA client (`acme-web`). Override via `SESAME_LIVE_TEST_*`.
fn demo_web_client() -> String {
    std::env::var("SESAME_LIVE_TEST_CLIENT_ID").unwrap_or_else(|_| "acme-web".into())
}
fn demo_web_redirect() -> String {
    std::env::var("SESAME_LIVE_TEST_REDIRECT")
        .unwrap_or_else(|_| "https://app.example.com/auth/callback".into())
}
fn demo_tenant() -> String {
    std::env::var("SESAME_LIVE_TEST_TENANT").unwrap_or_else(|_| "acme".into())
}

fn live_available() -> bool {
    let host = std::env::var("SESAME_OIDC_LIVE_HOST")
        .unwrap_or_else(|_| "id.sesameidentity.dev.local".to_string());
    let Ok(mut addrs) = (host.as_str(), 443u16).to_socket_addrs() else {
        return false;
    };
    let Some(addr) = addrs.next() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(800)).is_ok()
        || TcpStream::connect_timeout(
            &SocketAddr::from(([10, 177, 76, 220], 443)),
            Duration::from_millis(400),
        )
        .is_ok()
}

fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .expect("http client")
}

fn pkce_challenge() -> String {
    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn authorize_url(client_id: &str, redirect_uri: &str, challenge: &str, method: &str) -> String {
    format!(
        "{AUTH_BASE}/oauth/authorize?client_id={client_id}&response_type=code&redirect_uri={redirect}&scope=openid%20profile%20email&state=state1234567890abcd&nonce=nonce1234567890abcd&code_challenge={challenge}&code_challenge_method={method}",
        redirect = urlencoding_encode(redirect_uri),
    )
}

fn urlencoding_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[test]
fn live_discovery_is_truthful_oidc_metadata() {
    if !live_available() {
        eprintln!("SKIP live_discovery: issuer host unreachable");
        return;
    }
    let client = http_client();
    let resp = client
        .get(format!("{ID_BASE}/.well-known/openid-configuration"))
        .send()
        .expect("discovery request");
    assert_eq!(resp.status(), 200);
    let doc: serde_json::Value = resp.json().expect("discovery json");
    assert_eq!(doc["issuer"], ID_BASE);
    assert_eq!(
        doc["authorization_endpoint"],
        format!("{AUTH_BASE}/oauth/authorize")
    );
    assert_eq!(doc["token_endpoint"], format!("{API_BASE}/oauth/token"));
    assert_eq!(
        doc["userinfo_endpoint"],
        format!("{API_BASE}/oauth/userinfo")
    );
    assert_eq!(
        doc["jwks_uri"],
        format!("{ID_BASE}/.well-known/jwks.json")
    );
    let grants = doc["grant_types_supported"]
        .as_array()
        .expect("grants");
    assert!(grants.iter().any(|g| g == "authorization_code"));
    assert!(grants.iter().any(|g| g == "refresh_token"));
    assert!(!grants.iter().any(|g| g == "implicit"));
    assert_eq!(
        doc["response_types_supported"],
        serde_json::json!(["code"])
    );
    assert_eq!(
        doc["code_challenge_methods_supported"],
        serde_json::json!(["S256"])
    );
}

#[test]
fn live_jwks_publishes_verification_keys_only() {
    if !live_available() {
        eprintln!("SKIP live_jwks: issuer host unreachable");
        return;
    }
    let client = http_client();
    let resp = client
        .get(format!("{ID_BASE}/.well-known/jwks.json"))
        .send()
        .expect("jwks request");
    assert_eq!(resp.status(), 200);
    let doc: serde_json::Value = resp.json().expect("jwks json");
    let keys = doc["keys"].as_array().expect("keys");
    assert!(!keys.is_empty());
    for key in keys {
        assert!(key.get("kid").and_then(|v| v.as_str()).is_some());
        assert!(key.get("d").is_none(), "private key material must not leak");
        assert!(key.get("k").is_none(), "symmetric key material must not leak");
    }
}

#[test]
fn live_authorize_valid_pkce_redirects_to_hosted_auth() {
    if !live_available() {
        eprintln!("SKIP live_authorize_valid: issuer host unreachable");
        return;
    }
    let client = http_client();
    let url = authorize_url(demo_web_client().as_str(), demo_web_redirect().as_str(), &pkce_challenge(), "S256");
    let resp = client.get(&url).send().expect("authorize request");
    assert_eq!(resp.status(), 302, "body={}", resp.text().unwrap_or_default());
    let location = resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .expect("Location header");
    assert!(
        location.starts_with(&format!("{AUTH_BASE}/authorize?")),
        "unexpected Location {location}"
    );
    assert!(
        location.contains("request_id="),
        "missing request_id in {location}"
    );
    // Present after login-service rolls the authorize→hosted-auth query enrichment.
    if location.contains("tenant=") {
        assert!(
            location.contains("client_id="),
            "tenant without client_id in {location}"
        );
    }
}

#[test]
fn live_authorize_unknown_client_returns_invalid_client() {
    if !live_available() {
        eprintln!("SKIP live_authorize_unknown: issuer host unreachable");
        return;
    }
    let client = http_client();
    let url = authorize_url(
        "not-a-real-client",
        demo_web_redirect().as_str(),
        &pkce_challenge(),
        "S256",
    );
    let resp = client.get(&url).send().expect("authorize request");
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().expect("error json");
    assert_eq!(body["error"], "invalid_client");
}

#[test]
fn live_authorize_redirect_prefix_is_rejected() {
    if !live_available() {
        eprintln!("SKIP live_authorize_redirect_prefix: issuer host unreachable");
        return;
    }
    let client = http_client();
    let url = authorize_url(
        demo_web_client().as_str(),
        "https://app.example.com/auth/callback/evil",
        &pkce_challenge(),
        "S256",
    );
    let resp = client.get(&url).send().expect("authorize request");
    // Prefer JSON invalid_request; error redirect is also acceptable.
    if resp.status() == 400 {
        let body: serde_json::Value = resp.json().expect("error json");
        assert_eq!(body["error"], "invalid_request");
    } else {
        assert_eq!(resp.status(), 302);
        let location = resp
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(
            location.contains("error="),
            "expected error redirect, got {location}"
        );
    }
}

#[test]
fn live_token_without_client_auth_fails_closed() {
    if !live_available() {
        eprintln!("SKIP live_token_without_client_auth: issuer host unreachable");
        return;
    }
    let client = http_client();
    let resp = client
        .post(format!("{API_BASE}/oauth/token"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&code=not-a-code&redirect_uri={}&client_id={}&code_verifier=abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~",
            urlencoding_encode(demo_web_redirect().as_str()),
            demo_web_client().as_str()
        ))
        .send()
        .expect("token request");
    assert!(
        resp.status().as_u16() == 400 || resp.status().as_u16() == 401,
        "unexpected status {}",
        resp.status()
    );
    let body: serde_json::Value = resp.json().expect("error json");
    let error = body["error"].as_str().unwrap_or_default();
    assert!(
        matches!(error, "invalid_client" | "invalid_grant" | "invalid_request"),
        "unexpected error {error}"
    );
}

#[test]
fn live_userinfo_without_bearer_is_unauthorized() {
    if !live_available() {
        eprintln!("SKIP live_userinfo_without_bearer: issuer host unreachable");
        return;
    }
    let client = http_client();
    let resp = client
        .get(format!("{API_BASE}/oauth/userinfo"))
        .send()
        .expect("userinfo request");
    assert_eq!(resp.status(), 401);
}

#[test]
fn live_discovery_advertised_endpoints_are_reachable() {
    if !live_available() {
        eprintln!("SKIP live_discovery_advertised_endpoints: issuer host unreachable");
        return;
    }
    let client = http_client();
    let discovery: serde_json::Value = client
        .get(format!("{ID_BASE}/.well-known/openid-configuration"))
        .send()
        .expect("discovery")
        .json()
        .expect("json");

    // JWKS + UserInfo accept bare GET (401 for missing bearer is fine).
    for key in ["jwks_uri", "userinfo_endpoint"] {
        let url = discovery[key].as_str().expect(key);
        let resp = client
            .get(url)
            .send()
            .unwrap_or_else(|e| panic!("{key} {url} unreachable: {e}"));
        assert!(
            resp.status().as_u16() < 500,
            "{key} {url} returned server error {}",
            resp.status()
        );
    }

    // Authorize needs a full PKCE query — bare GET can 500 on request validation.
    let authorize = discovery["authorization_endpoint"]
        .as_str()
        .expect("authorization_endpoint");
    let authorize_url = format!(
        "{authorize}?client_id={client_id}&response_type=code&redirect_uri={redirect}&scope=openid&state=state1234567890abcd&nonce=nonce1234567890abcd&code_challenge={challenge}&code_challenge_method=S256",
        client_id = demo_web_client(),
        redirect = urlencoding_encode(&demo_web_redirect()),
        challenge = pkce_challenge(),
    );
    let resp = client
        .get(&authorize_url)
        .send()
        .expect("authorize probe");
    assert!(
        matches!(resp.status().as_u16(), 302 | 400),
        "authorize probe unexpected {}",
        resp.status()
    );

    let token = discovery["token_endpoint"].as_str().expect("token_endpoint");
    let resp = client
        .post(token)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("grant_type=authorization_code&code=x&client_id=acme-web&redirect_uri=https://example/cb&code_verifier=abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~")
        .send()
        .expect("token probe");
    assert!(
        resp.status().as_u16() < 500,
        "token probe server error {}",
        resp.status()
    );
}

const PUBLIC_CLIENT: &str = "fixture-public-client";
const PUBLIC_REDIRECT: &str = "https://client.example/callback";
fn demo_email() -> String {
    std::env::var("SESAME_LIVE_TEST_EMAIL").unwrap_or_else(|_| "owner@acme.example".into())
}
const DEMO_PASSWORD: &str = "SecureP@ss123!";

/// Live interactive path without the SPA: authorize → login → complete → token → userinfo.
#[test]
fn live_interactive_pkce_round_trip() {
    if !live_available() {
        eprintln!("SKIP live_interactive_pkce: issuer host unreachable");
        return;
    }
    let client = http_client();
    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
    let challenge = pkce_challenge();

    let authorize = authorize_url(PUBLIC_CLIENT, PUBLIC_REDIRECT, &challenge, "S256");
    let resp = client.get(&authorize).send().expect("authorize");
    if resp.status() == 400 {
        eprintln!("SKIP live_interactive_pkce: fixture-public-client not seeded");
        return;
    }
    assert_eq!(resp.status(), 302, "authorize: {}", resp.text().unwrap_or_default());
    let location = resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let request_id = location
        .split(['?', '&'])
        .find_map(|p| p.strip_prefix("request_id="))
        .expect("request_id")
        .to_string();

    let login = client
        .post(format!("{API_BASE}/idam/v1/auth/login"))
        .header("Content-Type", "application/json")
        .header("X-Tenant-ID", demo_tenant().as_str())
        .body(format!(
            r#"{{"email":"{email}","password":"{password}","client_id":"{client_id}"}}"#,
            email = demo_email(),
            password = DEMO_PASSWORD,
            client_id = demo_web_client(),
        ))
        .send()
        .expect("login");
    let login_status = login.status();
    let login_body: serde_json::Value = login.json().unwrap_or_else(|_| serde_json::json!({}));
    assert_eq!(login_status, 200, "login failed: {login_body}");
    let access = login_body["access_token"].as_str().unwrap().to_string();
    let user_id = login_body["user_id"].as_str().unwrap().to_string();

    let complete = client
        .post(format!("{AUTH_BASE}/oauth/authorize/complete"))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {access}"))
        .header("X-Tenant-ID", demo_tenant().as_str())
        .json(&serde_json::json!({ "request_id": request_id }))
        .send()
        .expect("complete");
    assert_eq!(
        complete.status(),
        302,
        "complete: {}",
        complete.text().unwrap_or_default()
    );
    let rp = complete
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(rp.starts_with(PUBLIC_REDIRECT), "rp redirect {rp}");
    let code = rp
        .split(['?', '&'])
        .find_map(|p| p.strip_prefix("code="))
        .expect("code")
        .to_string();

    let token = client
        .post(format!("{API_BASE}/oauth/token"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&code={code}&redirect_uri={}&client_id={PUBLIC_CLIENT}&code_verifier={verifier}",
            urlencoding_encode(PUBLIC_REDIRECT),
        ))
        .send()
        .expect("token");
    let token_status = token.status();
    let token_body: serde_json::Value = token.json().unwrap_or_else(|_| serde_json::json!({}));
    assert_eq!(token_status, 200, "token: {token_body}");
    let oidc_access = token_body["access_token"].as_str().unwrap();
    assert!(!token_body["id_token"].as_str().unwrap_or("").is_empty());

    let userinfo = client
        .get(format!("{API_BASE}/oauth/userinfo"))
        .header("Authorization", format!("Bearer {oidc_access}"))
        .send()
        .expect("userinfo");
    assert_eq!(userinfo.status(), 200);
    let info: serde_json::Value = userinfo.json().expect("userinfo json");
    assert_eq!(info["sub"], user_id);
}
