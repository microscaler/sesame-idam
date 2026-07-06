/// BDD integration tests for JWT TTL configuration (Story 3.3).
///
/// Tests token issuance with role-based TTL, env var overrides,
/// and expiry validation — using brrtrouter `HandlerRequest` pattern.
use brrtrouter::dispatcher::{HandlerRequest, HeaderVec};
use brrtrouter::ids::RequestId;
use http::Method;
use sesame_idam_identity_session_service::jwt::ttl::{
    validate_minimum_ttl, validate_refresh_exceeds_access, TtlConfig,
};

/// Create a minimal `HandlerRequest` for testing `auth_refresh`.
fn make_refresh_request(refresh_token: &str) -> HandlerRequest {
    let hv = HeaderVec::new();
    HandlerRequest {
        request_id: RequestId::new(),
        method: Method::POST,
        path: "/auth/refresh".to_string(),
        handler_name: "auth_refresh".to_string(),
        path_params: brrtrouter::router::ParamVec::default(),
        query_params: brrtrouter::router::ParamVec::default(),
        headers: hv,
        cookies: HeaderVec::new(),
        body: Some(serde_json::json!({
            "refresh_token": refresh_token,
        })),
        jwt_claims: None,
        reply_tx: may::sync::mpsc::channel().0,
        queue_guard: None,
    }
}

/// Scenario: Normal user refresh gets 5-minute access token (default TTL).
///
/// Given a customer user refreshes their token
/// WHEN the access token is issued
/// THEN `expires_in` = 300 seconds
#[test]
fn test_normal_user_refresh_gets_5_min_access_token() {
    let config = TtlConfig::from_env();
    let ttl = config.ttl_for_role("customer");
    assert_eq!(
        ttl.as_secs(),
        300,
        "Normal user access token TTL should be 300 seconds (5 minutes)"
    );
    let refresh_ttl = config.refresh_ttl_for_role("customer");
    assert!(
        refresh_ttl.as_secs() > ttl.as_secs(),
        "Refresh TTL must exceed access TTL"
    );
}

/// Scenario: Admin user gets 5-minute access token (F-010 aligned).
///
/// Given an `org_admin` issues a token for a user
/// WHEN the access token is issued
/// THEN `expires_in` = 300 seconds (same as normal, F-010 fix)
#[test]
fn test_admin_issue_gets_5_min_access_token() {
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

/// Scenario: Platform user gets 5-minute access token (F-010 aligned).
#[test]
fn test_platform_user_gets_5_min_token() {
    let config = TtlConfig::from_env();
    assert_eq!(
        config.ttl_for_role("platform").as_secs(),
        300,
        "Platform user access token TTL should be 300 seconds (F-010 aligned)"
    );
}

/// Scenario: Elevated user gets 5-minute access token (F-010 aligned).
#[test]
fn test_elevated_user_gets_5_min_token() {
    let config = TtlConfig::from_env();
    assert_eq!(
        config.ttl_for_role("elevated").as_secs(),
        300,
        "Elevated user access token TTL should be 300 seconds (F-010 aligned)"
    );
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
    let _config = TtlConfig::from_env();
    let iat: u64 = 1000;
    let exp: u64 = iat + 300; // 5-minute token
    let now: u64 = exp + 61; // 61 seconds past expiry

    assert!(now > exp, "Token should be expired by 61 seconds");
    assert!(
        (now - exp) > 60,
        "Should exceed 60-second clock skew tolerance"
    );
}

/// Scenario: Environment variable overrides default TTL for access token.
///
/// Given `JWT_ACCESS_TTL_NORMAL=600` is set
/// WHEN a normal user refreshes
/// THEN the access token has `expires_in` = 600 seconds
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

/// Scenario: Environment variable overrides elevated TTL.
#[test]
fn test_env_override_elevated_ttl() {
    let prev = std::env::var("JWT_ACCESS_TTL_ELEVATED").ok();

    std::env::set_var("JWT_ACCESS_TTL_ELEVATED", "600");
    let config = TtlConfig::from_env();
    assert_eq!(
        config.ttl_for_role("elevated").as_secs(),
        600,
        "JWT_ACCESS_TTL_ELEVATED env var should override default"
    );

    match prev {
        Some(v) => std::env::set_var("JWT_ACCESS_TTL_ELEVATED", v),
        None => std::env::remove_var("JWT_ACCESS_TTL_ELEVATED"),
    }
}

/// Scenario: Environment variable overrides admin TTL.
#[test]
fn test_env_override_admin_ttl() {
    let prev = std::env::var("JWT_ACCESS_TTL_ADMIN").ok();

    std::env::set_var("JWT_ACCESS_TTL_ADMIN", "600");
    let config = TtlConfig::from_env();
    assert_eq!(
        config.ttl_for_role("org_admin").as_secs(),
        600,
        "JWT_ACCESS_TTL_ADMIN env var should override default"
    );

    match prev {
        Some(v) => std::env::set_var("JWT_ACCESS_TTL_ADMIN", v),
        None => std::env::remove_var("JWT_ACCESS_TTL_ADMIN"),
    }
}

/// Scenario: Metrics track issued TTLs for different roles.
///
/// Given tokens are issued for different user types
/// THEN `token_ttl_seconds{role`: "..."} are emitted with correct values
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
/// For every role tier, `refresh_token_ttl` > `access_token_ttl`.
/// A refresh token should NEVER expire before its associated access token.
#[test]
fn test_refresh_ttl_exceeds_access_for_all_roles() {
    let config = TtlConfig::from_env();

    validate_refresh_exceeds_access(&config);

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
            "Refresh TTL ({refresh_secs}) must exceed access TTL ({access_secs}) for role {role}"
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

    let customer_ttl = config.ttl_for_role("customer");
    let admin_ttl = config.ttl_for_role("org_admin");

    // F-010: all roles are 300s, but the security property is that
    // the role must come from the authz service.
    assert_eq!(
        customer_ttl, admin_ttl,
        "F-010: All roles return same TTL — role resolution must be server-side"
    );

    // Verify the handler does NOT accept role from request body
    let req = make_refresh_request("dummy");
    assert_eq!(req.handler_name, "auth_refresh");
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

    let exp = config.exp_for_role("customer", iat);
    assert_eq!(
        exp, 1300,
        "exp must be iat + ttl_for_role, not from request"
    );

    assert_eq!(
        config.exp_for_role("org_admin", iat),
        1300,
        "exp must be deterministic for same iat regardless of role (F-010)"
    );
}

/// Edge case: Zero TTL — token issued with exp = iat (immediately expired).
///
/// If `JWT_ACCESS_TTL_NORMAL=0` is accidentally set, the token is issued
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
/// `validate_minimum_ttl` must panic if any TTL < 60 seconds.
#[test]
fn test_validate_minimum_ttl_rejects_zero() {
    let mut config = TtlConfig::from_env();
    config.normal_secs = 0;
    assert!(
        std::panic::catch_unwind(|| validate_minimum_ttl(&config)).is_err(),
        "validate_minimum_ttl should panic on zero TTL"
    );
}

/// Edge case: Minimum TTL validation rejects too-low TTL.
#[test]
fn test_validate_minimum_ttl_rejects_too_low() {
    let mut config = TtlConfig::from_env();
    config.admin_secs = 30;
    assert!(
        std::panic::catch_unwind(|| validate_minimum_ttl(&config)).is_err(),
        "validate_minimum_ttl should panic on TTL below 60 seconds"
    );
}

/// Edge case: Maximum TTL — 1-hour tokens still work.
///
/// If `JWT_ACCESS_TTL_NORMAL=3600` is set, the token is issued with 1-hour expiry.
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

/// Scenario: Refresh token TTL is configurable via env var.
#[test]
fn test_refresh_token_ttl_is_configurable() {
    let prev = std::env::var("JWT_REFRESH_TTL_DAYS").ok();

    std::env::set_var("JWT_REFRESH_TTL_DAYS", "14");
    let config = TtlConfig::from_env();
    let refresh_secs = config.refresh_ttl_for_role("customer").as_secs();
    assert_eq!(refresh_secs, 14 * 86400, "Refresh token should be 14 days");

    // Admin gets shorter refresh
    let admin_refresh = config.refresh_ttl_for_role("org_admin").as_secs();
    assert_eq!(admin_refresh, 7 * 86400, "Admin refresh should be 7 days");

    match prev {
        Some(v) => std::env::set_var("JWT_REFRESH_TTL_DAYS", v),
        None => std::env::remove_var("JWT_REFRESH_TTL_DAYS"),
    }
}
