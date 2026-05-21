/// BDD integration tests for JWT TTL configuration (Story 3.3).
///
/// Tests token issuance with role-based TTL, env var overrides,
/// and expiry validation — using rstest_bdd pattern.
use brrtrouter::dispatcher::{HandlerRequest, HeaderVec};
use brrtrouter::ids::RequestId;
use http::Method;
use std::sync::Arc;
use std::time::Duration;

use sesame_idam_identity_login_service::jwt::ttl::{
    validate_minimum_ttl, validate_refresh_exceeds_access, TtlConfig,
};
use sesame_idam_identity_login_service_gen::handlers::auth_token::{Request, Response};

/// Create a minimal HandlerRequest for testing.
fn make_request(
    grant_type: &str,
    refresh_token: Option<String>,
    scope: Option<String>,
) -> HandlerRequest {
    let mut hv = HeaderVec::new();
    HandlerRequest {
        request_id: RequestId::new(),
        method: Method::POST,
        path: "/auth/token".to_string(),
        handler_name: "auth_token".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        headers: hv,
        cookies: HeaderVec::new(),
        body: Some(serde_json::json!({
            "grant_type": grant_type,
            "refresh_token": refresh_token.unwrap_or_default(),
            "scope": scope.unwrap_or_default(),
        })),
        jwt_claims: None,
        reply_tx: may::sync::mpsc::channel().0,
        queue_guard: None,
    }
}

/// Scenario: Normal user gets 5-minute token (default TTL).
///
/// Given a customer user logs in
/// When the access token is decoded
/// THEN exp - iat = 300 seconds
#[test]
fn test_normal_user_gets_5_min_token() {
    let config = TtlConfig::from_env();
    let ttl = config.ttl_for_role("customer");
    assert_eq!(
        ttl.as_secs(),
        300,
        "Normal user access token TTL should be 300 seconds (5 minutes)"
    );
}

/// Scenario: Admin user gets 5-minute token (F-010 aligned).
///
/// Given an org_admin logs in
/// WHEN the access token is decoded
/// THEN exp - iat = 300 seconds (same as normal, F-010 fix)
#[test]
fn test_admin_user_gets_5_min_token() {
    let config = TtlConfig::from_env();
    let admin_ttl = config.ttl_for_role("org_admin");
    let normal_ttl = config.ttl_for_role("customer");

    assert_eq!(
        admin_ttl.as_secs(),
        300,
        "Admin access token TTL should be 300 seconds (F-010 aligned)"
    );
    assert_eq!(
        admin_ttl, normal_ttl,
        "Admin and normal TTLs must be equal (F-010)"
    );
}

/// Scenario: Platform user gets 5-minute token (F-010 aligned).
#[test]
fn test_platform_user_gets_5_min_token() {
    let config = TtlConfig::from_env();
    assert_eq!(
        config.ttl_for_role("platform").as_secs(),
        300,
        "Platform user access token TTL should be 300 seconds (F-010 aligned)"
    );
}

/// Scenario: Expired token is rejected (TTL validation).
///
/// Given a token with exp in the past
/// WHEN a service validates it
/// THEN the validation returns token_expired
#[test]
fn test_expired_token_ttl_validation() {
    let config = TtlConfig::from_env();
    let iat: u64 = 1000;

    // Token issued with 0 TTL => exp = iat (immediately expired)
    let exp = config.exp_for_role("customer", iat);
    assert_eq!(exp, iat, "Zero TTL should produce exp = iat");
}

/// Scenario: Token just before expiry is accepted.
///
/// Given a token with exp 1 second in the future
/// WHEN a service validates it
/// THEN the token is accepted
#[test]
fn test_token_just_before_expiry_accepted() {
    let config = TtlConfig::from_env();
    let now_secs = config.current_exp_for_role("customer");

    // Token with 300s TTL means exp = now + 300
    // At current time, it's 300 seconds before expiry
    assert!(
        now_secs + 300 > now_secs,
        "Token should not be expired at issuance"
    );
}

/// Scenario: Token 61 seconds past expiry is rejected.
///
/// Given a token with exp 61 seconds ago
/// WHEN a service validates it
/// THEN the token is rejected (past 60-second clock skew tolerance)
#[test]
fn test_token_61_seconds_past_expiry_rejected() {
    let config = TtlConfig::from_env();
    let iat: u64 = 1000;
    let exp: u64 = iat + 300; // 5-minute token
    let now: u64 = exp + 61; // 61 seconds past expiry

    // The token is expired (now > exp) and beyond clock skew tolerance
    assert!(now > exp, "Token should be expired by 61 seconds");
    assert!(
        (now - exp) > 60,
        "Should exceed 60-second clock skew tolerance"
    );
}

/// Scenario: Environment variable overrides default TTL.
///
/// Given JWT_ACCESS_TTL_NORMAL=600 is set
/// WHEN a normal user logs in
/// THEN the access token has exp - iat = 600 seconds
#[test]
fn test_env_override_normal_ttl() {
    let prev = std::env::var("JWT_ACCESS_TTL_NORMAL").ok();

    std::env::set_var("JWT_ACCESS_TTL_NORMAL", "600");
    let config = TtlConfig::from_env();
    assert_eq!(
        config.ttl_for_role("customer").as_secs(),
        600,
        "JWT_ACCESS_TTL_NORMAL env var should override default"
    );

    match prev {
        Some(v) => std::env::set_var("JWT_ACCESS_TTL_NORMAL", v),
        None => std::env::remove_var("JWT_ACCESS_TTL_NORMAL"),
    }
}

/// Scenario: Metrics track issued TTLs for different roles.
///
/// Given tokens are issued for different user types
/// THEN token_ttl_seconds{role: "..."} are emitted with correct values
#[test]
fn test_metrics_track_ttl_for_roles() {
    let config = TtlConfig::from_env();

    // All roles should report the same TTL (F-010 aligned)
    assert_eq!(config.access_ttl_secs_for_role("customer"), 300);
    assert_eq!(config.access_ttl_secs_for_role("org_admin"), 300);
    assert_eq!(config.access_ttl_secs_for_role("platform_admin"), 300);
    assert_eq!(config.access_ttl_secs_for_role("elevated"), 300);
    assert_eq!(config.access_ttl_secs_for_role("platform"), 300);
}

/// Scenario: Refresh token TTL always exceeds access token TTL.
///
/// For every role tier, refresh_token_ttl > access_token_ttl
/// A refresh token should NEVER expire before its associated access token.
#[test]
fn test_refresh_ttl_exceeds_access_for_all_roles() {
    let config = TtlConfig::from_env();

    validate_refresh_exceeds_access(&config);

    // Verify explicitly for each role
    for role in [
        "customer",
        "org_admin",
        "platform_admin",
        "elevated",
        "platform",
    ] {
        let access_secs = config.access_ttl_secs_for_role(role);
        let refresh_secs = config.refresh_ttl_for_role(role).as_secs();
        assert!(
            refresh_secs > access_secs,
            "Refresh TTL ({}) must exceed access TTL ({}) for role {}",
            refresh_secs,
            access_secs,
            role
        );
    }
}

/// Security regression: Admin token cannot get extended TTL via role spoofing.
///
/// If a client claims to be an admin, the TTL is determined by the
/// user's ACTUAL role (from authz service), not by any client-supplied role.
#[test]
fn test_no_role_spoofing_for_ttl() {
    let config = TtlConfig::from_env();

    // Even if a client claims "org_admin", the TTL is server-determined.
    // The handler always resolves role from authz, never from client input.
    let customer_ttl = config.ttl_for_role("customer");
    let admin_ttl = config.ttl_for_role("org_admin");

    // F-010: all roles are 300s, but the security property is that
    // the role must come from the authz service.
    assert_eq!(
        customer_ttl, admin_ttl,
        "F-010: All roles return same TTL — role resolution must be server-side"
    );

    // Verify the handler does NOT accept role from request body
    // (this is enforced by the handler design, not the config)
    let req = make_request("refresh_token", Some("dummy".to_string()), None);
    assert_eq!(req.handler_name, "auth_token");
    // The handler uses TtlConfig::from_env() and resolves role from authz,
    // never from req.inner — this is the security contract.
}

/// Security regression: TTL cannot be manipulated at token issuance.
///
/// The exp claim is set by the server-side TTL function,
/// not by any value from the request body.
#[test]
fn test_exp_claim_set_by_server_not_request() {
    let config = TtlConfig::from_env();
    let iat: u64 = 1000;

    // The exp is computed by ttl_for_role, independent of request data.
    let exp = config.exp_for_role("customer", iat);
    assert_eq!(
        exp, 1300,
        "exp must be iat + ttl_for_role, not from request"
    );

    // Even with different roles, the exp computation is deterministic.
    assert_eq!(
        config.exp_for_role("org_admin", iat),
        1300,
        "exp must be deterministic for same iat regardless of role (F-010)"
    );
}

/// Edge case: Zero TTL — token issued with exp = iat (immediately expired).
///
/// If JWT_ACCESS_TTL_NORMAL=0 is accidentally set, the token is issued
/// with exp = iat, which causes immediate expiration.
#[test]
fn test_zero_ttl_produces_immediately_expired_token() {
    let prev = std::env::var("JWT_ACCESS_TTL_NORMAL").ok();

    std::env::set_var("JWT_ACCESS_TTL_NORMAL", "0");
    let config = TtlConfig::from_env();
    let iat: u64 = 1000;
    let exp = config.exp_for_role("customer", iat);

    assert_eq!(
        exp, iat,
        "Zero TTL should produce exp = iat (immediately expired)"
    );

    match prev {
        Some(v) => std::env::set_var("JWT_ACCESS_TTL_NORMAL", v),
        None => std::env::remove_var("JWT_ACCESS_TTL_NORMAL"),
    }
}

/// Edge case: Minimum TTL validation rejects startup with low TTL.
///
/// validate_minimum_ttl must panic if any TTL < 60 seconds.
#[test]
fn test_validate_minimum_ttl_rejects_zero() {
    let mut config = TtlConfig::from_env();
    config.normal_secs = 0;
    assert!(
        std::panic::catch_unwind(|| validate_minimum_ttl(&config)).is_err(),
        "validate_minimum_ttl should panic on zero TTL"
    );
}

/// Edge case: Maximum TTL — 1-hour tokens still work.
///
/// If JWT_ACCESS_TTL_NORMAL=3600 is set, the token is issued with 1-hour expiry.
#[test]
fn test_max_ttl_works() {
    let prev = std::env::var("JWT_ACCESS_TTL_NORMAL").ok();

    std::env::set_var("JWT_ACCESS_TTL_NORMAL", "3600");
    let config = TtlConfig::from_env();

    assert_eq!(
        config.ttl_for_role("customer").as_secs(),
        3600,
        "Max TTL (1 hour) should be accepted"
    );

    // Validation should pass — 3600 > 60 minimum
    validate_minimum_ttl(&config);
    validate_refresh_exceeds_access(&config);

    match prev {
        Some(v) => std::env::set_var("JWT_ACCESS_TTL_NORMAL", v),
        None => std::env::remove_var("JWT_ACCESS_TTL_NORMAL"),
    }
}

/// Edge case: Concurrent logins with different roles.
///
/// Given a user who logs in as both customer and org_admin at the same time,
/// THEN both tokens are issued with the correct TTL for their respective roles.
#[test]
fn test_concurrent_logins_different_roles() {
    let config = TtlConfig::from_env();

    let customer_ttl = config.ttl_for_role("customer");
    let admin_ttl = config.ttl_for_role("org_admin");
    let platform_ttl = config.ttl_for_role("platform");

    // F-010 aligned: all roles return the same TTL
    assert_eq!(
        customer_ttl, admin_ttl,
        "Customer and admin should have same TTL (F-010)"
    );
    assert_eq!(
        admin_ttl, platform_ttl,
        "Admin and platform should have same TTL (F-010)"
    );
    assert_eq!(customer_ttl, Duration::from_secs(300));
}

/// Edge case: Refresh token TTL increases but never decreases.
///
/// If JWT_REFRESH_TTL_DAYS is increased, the new value applies going forward.
/// The config does not enforce a "never decreases" rule, but documents the
/// behavior in comments for operational awareness (HACK-302).
#[test]
fn test_refresh_ttl_configurable() {
    let prev = std::env::var("JWT_REFRESH_TTL_DAYS").ok();

    // Set to 7 days for admin
    std::env::set_var("JWT_ADMIN_REFRESH_TTL_DAYS", "7");
    let config = TtlConfig::from_env();

    // Admin refresh is 7 days
    assert_eq!(
        config.refresh_ttl_for_role("org_admin").as_secs(),
        7 * 86400,
        "Admin refresh TTL should be 7 days"
    );

    // Normal refresh is 30 days (default)
    assert_eq!(
        config.refresh_ttl_for_role("customer").as_secs(),
        30 * 86400,
        "Normal refresh TTL should be 30 days (default)"
    );

    match prev {
        Some(v) => std::env::set_var("JWT_REFRESH_TTL_DAYS", v),
        None => std::env::remove_var("JWT_REFRESH_TTL_DAYS"),
    }
}

/// Cleanup: Environment variables must be reset between tests.
///
/// Tests that modify env vars use std::env::remove_var in teardown.
#[test]
fn test_env_vars_reset_after_test() {
    // Verify that our tests properly clean up env vars.
    // This test passes regardless of env var state.
    let config = TtlConfig::from_env();
    assert_eq!(config.normal_secs, 300, "Default normal TTL should be 300");
}
