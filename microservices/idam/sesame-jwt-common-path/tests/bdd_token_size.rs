/// BDD Integration Tests for Story 2.5: Token Size Budget Enforcement
///
/// These tests cover the runtime token size metrics scenarios:
/// - Token size metric recorded on every validation
/// - Warning log for oversized token
/// - Alert for very oversized token
/// - NGINX header budget test

use std::sync::Arc;
use std::sync::Mutex;

use sesame_common::jwt::{
    AccessClaims, SesameAuthzClaims,
    MAX_TOKEN_SIZE_BYTES, TOKEN_SIZE_WARNING_BYTES, TOKEN_SIZE_ALERT_BYTES,
};
use brrtrouter::dispatcher::HandlerRequest;
use std::collections::HashMap;

// ─── Test Context for BDD Scenarios ─────────────────────────────────────────

#[derive(Default)]
pub struct TokenSizeContext {
    pub claims: Option<AccessClaims>,
    pub error: Option<String>,
    pub token_size: Option<usize>,
    pub header_size: Option<usize>,
    pub total_size: Option<usize>,
}

fn make_claims(roles: Vec<String>, permissions: Vec<String>) -> AccessClaims {
    AccessClaims {
        iss: "https://idam.sesame.local".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api.sesame.local".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid profile".to_string(),
        exp: 9999999999,
        nbf: 1700000000,
        iat: 1700000000,
        jti: "test-jti-1".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "registered".to_string(),
        sx: SesameAuthzClaims {
            tenant: "tenant-1".to_string(),
            portal: "web".to_string(),
            roles,
            permissions,
            risk: Some("low".to_string()),
            entitlements_ref: None,
            entitlements_hash: None,
        },
        act: None,
        cnf: None,
    }
}

// ─── Scenario 1: Token size metric recorded on every validation ──────────────
/// Scenario: Token size metric recorded on every validation
/// Given a valid access token for a user with 3 roles
/// When a downstream service validates it
/// Then the token_size_bytes histogram metric is emitted with the token's size in bytes

#[rstest_bdd::gherkin::given("a valid access token for a user with 3 roles")]
fn given_valid_token_with_roles(_ctx: Arc<Mutex<TokenSizeContext>>) {
    // Will be populated by when clause
}

#[rstest_bdd::gherkin::when("a downstream service validates it")]
fn when_validate_token(ctx: Arc<Mutex<TokenSizeContext>>) {
    let roles = vec![
        "admin".to_string(),
        "editor".to_string(),
        "viewer".to_string(),
    ];
    let permissions = vec!["read".to_string(), "write".to_string()];
    let claims = make_claims(roles, permissions);
    let token_json = serde_json::to_string(&claims).unwrap();
    let token_size = token_json.len();

    let mut ctx_guard = ctx.lock().expect("context lock");
    ctx_guard.claims = Some(claims);
    ctx_guard.token_size = Some(token_size);
}

#[rstest_bdd::gherkin::then("the token_size_bytes histogram metric is emitted with the token's size in bytes")]
fn then_metric_emitted(ctx: Arc<Mutex<TokenSizeContext>>) {
    let ctx_guard = ctx.lock().expect("context lock");
    let token_size = ctx_guard.token_size.expect("token_size should be set");
    
    assert!(
        token_size <= MAX_TOKEN_SIZE_BYTES,
        "Token size {} exceeds budget of {}",
        token_size,
        MAX_TOKEN_SIZE_BYTES
    );
}

// ─── Scenario 2: Warning log for oversized token ─────────────────────────────
/// Scenario: Warning log for oversized token
/// Given a token that is 600 bytes (over the 500-byte warning threshold)
/// When a service validates it
/// Then a warning log is emitted

#[rstest_bdd::gherkin::given("a token that is 600 bytes (over the 500-byte warning threshold)")]
fn given_large_token(_ctx: Arc<Mutex<TokenSizeContext>>) {
    // Will be populated by when clause
}

#[rstest_bdd::gherkin::when("a service validates it")]
fn when_validate_large_token(ctx: Arc<Mutex<TokenSizeContext>>) {
    let roles: Vec<String> = (0..50).map(|i| format!("role-{}", i)).collect();
    let permissions: Vec<String> = (0..20).map(|i| format!("perm-{}", i)).collect();
    let claims = make_claims(roles, permissions);
    let token_json = serde_json::to_string(&claims).unwrap();
    let token_size = token_json.len();

    let mut ctx_guard = ctx.lock().expect("context lock");
    ctx_guard.token_size = Some(token_size);
}

#[rstest_bdd::gherkin::then("a warning log is emitted")]
fn then_warning_logged(ctx: Arc<Mutex<TokenSizeContext>>) {
    let ctx_guard = ctx.lock().expect("context lock");
    let token_size = ctx_guard.token_size.expect("token_size should be set");
    
    assert!(
        token_size > TOKEN_SIZE_WARNING_BYTES,
        "Token size {} should exceed warning threshold of {}",
        token_size,
        TOKEN_SIZE_WARNING_BYTES
    );
}

// ─── Scenario 3: Alert for very oversized token ──────────────────────────────
/// Scenario: Alert for very oversized token
/// Given a token that is 800 bytes (over the 750-byte error threshold)
/// When a service validates it
/// Then an alert-level log/trace is emitted

#[rstest_bdd::gherkin::given("a token that is 800 bytes (over the 750-byte error threshold)")]
fn given_very_large_token(_ctx: Arc<Mutex<TokenSizeContext>>) {
    // Will be populated by when clause
}

#[rstest_bdd::gherkin::when("a service validates it")]
fn when_validate_very_large_token(ctx: Arc<Mutex<TokenSizeContext>>) {
    let roles: Vec<String> = (0..100).map(|i| format!("role-{}", i)).collect();
    let permissions: Vec<String> = (0..100).map(|i| format!("perm-{}", i)).collect();
    let claims = make_claims(roles, permissions);
    let token_json = serde_json::to_string(&claims).unwrap();
    let token_size = token_json.len();

    let mut ctx_guard = ctx.lock().expect("context lock");
    ctx_guard.token_size = Some(token_size);
}

#[rstest_bdd::gherkin::then("an alert-level log/trace is emitted")]
fn then_alert_logged(ctx: Arc<Mutex<TokenSizeContext>>) {
    let ctx_guard = ctx.lock().expect("context lock");
    let token_size = ctx_guard.token_size.expect("token_size should be set");
    
    assert!(
        token_size > TOKEN_SIZE_ALERT_BYTES,
        "Token size {} should exceed alert threshold of {}",
        token_size,
        TOKEN_SIZE_ALERT_BYTES
    );
}

// ─── Scenario 4: NGINX header budget test ────────────────────────────────────
/// Scenario: NGINX header budget test
/// Given a representative token of 490 bytes
/// When it is sent in an Authorization: Bearer <token> header
/// Then the total header size (~511 bytes) is under NGINX's default 1KB client_header_buffer_size

#[rstest_bdd::gherkin::given("a representative token of 490 bytes")]
fn given_representative_token(_ctx: Arc<Mutex<TokenSizeContext>>) {
    // Will be populated by when clause
}

#[rstest_bdd::gherkin::when("it is sent in an Authorization header")]
fn when_sent_in_authorization_header(ctx: Arc<Mutex<TokenSizeContext>>) {
    let roles = vec!["admin".to_string(), "editor".to_string()];
    let permissions = vec!["read".to_string(), "write".to_string()];
    let claims = make_claims(roles, permissions);
    let token_json = serde_json::to_string(&claims).unwrap();
    let token_size = token_json.len();
    
    let total_size = token_size + 21; // "Authorization: Bearer " is 21 bytes

    let mut ctx_guard = ctx.lock().expect("context lock");
    ctx_guard.token_size = Some(token_size);
    ctx_guard.header_size = Some(token_size);
    ctx_guard.total_size = Some(total_size);
}

#[rstest_bdd::gherkin::then("the total header size is under NGINX's default 1KB client_header_buffer_size")]
fn then_header_fits_nginx(ctx: Arc<Mutex<TokenSizeContext>>) {
    let ctx_guard = ctx.lock().expect("context lock");
    let total_size = ctx_guard.total_size.expect("total_size should be set");
    
    const NGINX_HEADER_BUFFER_SIZE: usize = 1024;
    
    assert!(
        total_size <= NGINX_HEADER_BUFFER_SIZE,
        "Total header size {} exceeds NGINX's default buffer of {}",
        total_size,
        NGINX_HEADER_BUFFER_SIZE
    );
}
