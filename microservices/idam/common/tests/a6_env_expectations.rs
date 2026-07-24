//! Gate A6 acceptance: iss/aud expectations are ENVIRONMENT CONFIG, not
//! compile-time constants — and audience scoping rejects cross-service
//! tokens.
//!
//! Runs with `JWT_ALLOWED_ISSUERS` / `JWT_EXPECTED_AUDIENCES` set for this
//! process (nextest = process per test, so the OnceLock snapshot is
//! deterministic): a staging-style issuer + a single-service audience.

use sesame_common::jwt::helpers::{allowed_issuers, expected_audiences};
use sesame_common::{AccessClaimsBuilder, JwtValidationError, SesameAuthzClaimsBuilder};

const STAGING_ISSUER: &str = "https://idam.staging.example.net";
const THIS_SERVICE_AUD: &str = "authz-core";
const OTHER_SERVICE_AUD: &str = "api-keys";

fn set_env_once() {
    // Safe: this integration test binary's tests run in one process; both
    // tests want the same values, set before any OnceLock read.
    std::env::set_var("JWT_ALLOWED_ISSUERS", STAGING_ISSUER);
    std::env::set_var("JWT_EXPECTED_AUDIENCES", THIS_SERVICE_AUD);
}

fn claims_with(iss: &str, aud: &str) -> sesame_common::AccessClaims {
    let now = 1_800_000_000i64;
    let sx = SesameAuthzClaimsBuilder::new()
        .tenant("tenant-a")
        .portal("frontend")
        .roles(vec!["user".to_string()])
        .permissions(vec![])
        .build()
        .expect("sx claims");
    AccessClaimsBuilder::new()
        .iss(iss)
        .sub("user-1")
        .aud(vec![aud.to_string()])
        .client_id("frontend")
        .scope("openid")
        .exp(now + 300)
        .nbf(now)
        .iat(now)
        .jti("jti-1")
        .ver(1)
        .sid("sid-1")
        .tenant_id("tenant-a")
        .user_id("user-1")
        .user_type("customer")
        .sx(sx)
        .build()
        .expect("claims build")
}

/// Scenario: the env override IS the effective expectation list, and a token
/// for this service's audience from the configured issuer validates.
#[test]
fn env_overrides_define_expectations_and_accept_matching_tokens() {
    set_env_once();

    assert_eq!(allowed_issuers(), &[STAGING_ISSUER.to_string()]);
    assert_eq!(expected_audiences(), &[THIS_SERVICE_AUD.to_string()]);

    let claims = claims_with(STAGING_ISSUER, THIS_SERVICE_AUD);
    assert!(claims.validate().is_ok(), "matching iss+aud must validate");
}

/// Scenario: cross-service rejection — a token minted for ANOTHER service's
/// audience is rejected here; a token from a non-configured issuer (even the
/// compiled-in default) is rejected everywhere.
#[test]
fn cross_service_audience_and_foreign_issuer_rejected() {
    set_env_once();

    // Token for service X presented to service Y → aud mismatch.
    let cross = claims_with(STAGING_ISSUER, OTHER_SERVICE_AUD);
    assert!(
        matches!(cross.validate(), Err(JwtValidationError::InvalidAudience)),
        "token minted for {OTHER_SERVICE_AUD} must be rejected by {THIS_SERVICE_AUD}"
    );

    // Wrong issuer — including the compiled-in default once the environment
    // has pinned its own list (config wins over code).
    let foreign = claims_with("https://idam.example.com", THIS_SERVICE_AUD);
    assert!(
        matches!(foreign.validate(), Err(JwtValidationError::InvalidIssuer)),
        "issuer outside JWT_ALLOWED_ISSUERS must be rejected"
    );
}
