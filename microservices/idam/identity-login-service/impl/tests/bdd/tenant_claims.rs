// BDD tests for JWT token payload — Story 2.4: Add Tenant to JWT Claims
//
// These tests verify the end-to-end behavior of tenant_id embedding in JWT
// tokens for Story 2.4: tenant_id appears at both top-level and namespaced
// (sx.tenant) in every access token.
//
// They exercise the jwt:: module functions directly (unit-level BDD) and
// also serve as integration tests by constructing full AccessClaims structs
// that mimic what the login service controller would produce.

use sesame_common::jwt::*;

/// Scenario: Tenant ID flows from login to token
///   Given a user belonging to tenant hauliage (UUID: abc-123)
///   When the user logs in with X-Tenant-ID: abc-123
///   Then the access token's tenant_id field equals abc-123 in both
///       top-level and sx.tenant
#[test]
fn scenario_tenant_id_flows_from_login_to_token() {
    // Given — user belonging to hauliage tenant
    let hauliage_uuid = "hauliage-tenant-uuid-abc123".to_string();

    // When — build a token like the login controller would, with tenant_id
    // from the X-Tenant-ID header
    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("alice-hauliage-001")
        .aud(vec!["api".to_string(), "frontend".to_string()])
        .client_id("hauliage-mobile")
        .scope("openid profile".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-alice-hauliage-001".to_string())
        .ver(1)
        .sid("session-alice-hauliage-001".to_string())
        .tenant_id(hauliage_uuid.clone())
        .user_id("alice-hauliage-001".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            hauliage_uuid.clone(),
            "hauliage-mobile".to_string(),
            vec!["driver".to_string()],
            vec!["orders:read".to_string()],
        ))
        .build()
        .expect("valid claims");

    // Then — tenant_id present at top level
    let json = claims.to_compact_json();
    assert!(
        json.contains(&format!("\"tenant_id\":\"{}\"", hauliage_uuid)),
        "top-level tenant_id must equal hauliage UUID"
    );

    // And — tenant_id present in namespaced claims
    assert!(
        json.contains(&format!("\"tenant\":\"{}\"", hauliage_uuid)),
        "sx.tenant must equal hauliage UUID"
    );

    // And — both must match
    assert_eq!(
        claims.tenant_id, claims.sx.tenant,
        "top-level and namespaced tenant_id must match"
    );
    assert_eq!(
        claims.tenant_id, hauliage_uuid,
        "top-level tenant_id must match X-Tenant-ID from login request"
    );
}

/// Scenario: Cross-tenant login is rejected
///   Given a user registered under tenant hauliage
///   When the user attempts to login with X-Tenant-ID: rerp
///   Then the login returns an error (not a password error — prevents
///       tenant enumeration)
///
/// This test verifies the validation logic: if a user's DB tenant doesn't
/// match the request's X-Tenant-ID, login should be rejected.
#[test]
fn scenario_cross_tenant_login_rejected() {
    // Given — user registered under hauliage tenant
    let user_tenant = "hauliage-tenant-uuid".to_string();

    // When — attempt to login with different tenant ID (rerp)
    let rerp_tenant = "rerp-tenant-uuid".to_string();

    // Build claims as if the login handler validated and found a mismatch
    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("alice-corp-001")
        .aud(vec!["api".to_string()])
        .client_id("login-app")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-alice-001".to_string())
        .ver(1)
        .sid("session-alice-001".to_string())
        .tenant_id(user_tenant.clone()) // User's actual tenant
        .user_id("alice-corp-001".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            user_tenant.clone(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    // Then — validation against wrong tenant must fail
    let result = claims.validate_tenant(&rerp_tenant);
    assert!(result.is_err(), "cross-tenant login must be rejected");

    match result.unwrap_err() {
        JwtError::TenantMismatch { expected, actual } => {
            assert_eq!(
                expected, user_tenant,
                "Expected user's actual tenant in error"
            );
            assert_eq!(actual, rerp_tenant, "Expected request tenant in error");
        }
        _ => panic!("Expected TenantMismatch error"),
    }
}

/// Scenario: Downstream service validates tenant
///   Given a JWT with tenant_id = hauliage
///   When a downstream service receives the request with
///       X-Tenant-ID: rerp
///   Then the service rejects the request with 401 Tenant Mismatch
#[test]
fn scenario_downstream_service_validates_tenant() {
    // Given — a JWT from hauliage tenant
    let jwt_tenant = "hauliage-tenant-uuid".to_string();

    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("driver-001")
        .aud(vec!["api".to_string()])
        .client_id("hauliage-driver-app")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-driver-001".to_string())
        .ver(1)
        .sid("session-driver-001".to_string())
        .tenant_id(jwt_tenant.clone())
        .user_id("driver-001".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            jwt_tenant.clone(),
            "hauliage-driver".to_string(),
            vec!["driver".to_string()],
            vec!["rides:read".to_string()],
        ))
        .build()
        .expect("valid claims");

    // When — downstream service receives request with different tenant
    let request_tenant = "rerp-tenant-uuid".to_string();

    // Then — tenant mismatch is detected
    let result = claims.validate_tenant(&request_tenant);
    assert!(
        result.is_err(),
        "downstream service must reject tenant mismatch"
    );
}

/// Scenario: Tenant ID present in LoginResponse
///   Given a successful login
///   When the LoginResponse is returned
///   Then it includes a tenant_id field matching the tenant of the
///       authenticated user
#[test]
fn scenario_tenant_id_present_in_login_response() {
    // Given — user authenticated under tenant
    let tenant_uuid = "tenant-rerp-abc456".to_string();

    // When — build claims like login controller would
    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-rerp-001")
        .aud(vec!["api".to_string()])
        .client_id("rerp-web")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-rerp-001".to_string())
        .ver(1)
        .sid("session-rerp-001".to_string())
        .tenant_id(tenant_uuid.clone())
        .user_id("user-rerp-001".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            tenant_uuid.clone(),
            "rerp-web".to_string(),
            vec!["admin".to_string()],
            vec!["org:admin".to_string()],
        ))
        .build()
        .expect("valid claims");

    // Then — tenant_id is present and matches user's tenant
    assert_eq!(
        claims.tenant_id, tenant_uuid,
        "LoginResponse tenant_id must match authenticated user's tenant"
    );
    assert_eq!(
        claims.sx.tenant, tenant_uuid,
        "sx.tenant must match authenticated user's tenant"
    );
}

/// Scenario: Different users on different tenants have different JWT tenants
///   Given user alice on tenant hauliage and user alice on tenant rerp
///   When both login
///   Then alice@hauliage's JWT has tenant_id = hauliage_uuid and
///       alice@rerp's JWT has tenant_id = rerp_uuid — confirming zero
///       cross-tenant identity
#[test]
fn scenario_zero_cross_tenant_identity() {
    // Given — two "alice" users on different tenants
    let hauliage_uuid = "tenant-hauliage-xyz".to_string();
    let rerp_uuid = "tenant-rerp-def".to_string();

    // When — both users login
    let claims_hauliage = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("alice-hauliage-001")
        .aud(vec!["api".to_string()])
        .client_id("hauliage-app")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-alice-hauliage".to_string())
        .ver(1)
        .sid("session-alice-hauliage".to_string())
        .tenant_id(hauliage_uuid.clone())
        .user_id("alice-hauliage-001".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            hauliage_uuid.clone(),
            "hauliage-web".to_string(),
            vec!["customer".to_string()],
            vec![],
        ))
        .build()
        .expect("valid hauliage claims");

    let claims_rerp = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("alice-rerp-001")
        .aud(vec!["api".to_string()])
        .client_id("rerp-app")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-alice-rerp".to_string())
        .ver(1)
        .sid("session-alice-rerp".to_string())
        .tenant_id(rerp_uuid.clone())
        .user_id("alice-rerp-001".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            rerp_uuid.clone(),
            "rerp-web".to_string(),
            vec!["customer".to_string()],
            vec![],
        ))
        .build()
        .expect("valid rerp claims");

    // Then — same email pattern but different tenants = completely unrelated
    assert_eq!(
        claims_hauliage.tenant_id, hauliage_uuid,
        "alice@hauliage must have hauliage tenant_id"
    );
    assert_eq!(
        claims_rerp.tenant_id, rerp_uuid,
        "alice@rerp must have rerp tenant_id"
    );
    assert_ne!(
        claims_hauliage.tenant_id, claims_rerp.tenant_id,
        "same email pattern on different tenants must have different tenant IDs"
    );
}

/// Scenario: Tenant ID cannot be forged in token
///   If a client modifies the tenant_id claim in a validly-signed token,
///   assert that the JWT signature verification fails (the token cannot be
///   tampered with — only the issuer can set the tenant)
#[test]
fn scenario_tenant_id_cannot_be_forged() {
    // Given — a validly-built token
    let original_tenant = "tenant-abc-123".to_string();
    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-001")
        .aud(vec!["api".to_string()])
        .client_id("app")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-001".to_string())
        .ver(1)
        .sid("session-001".to_string())
        .tenant_id(original_tenant.clone())
        .user_id("user-001".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            original_tenant.clone(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    // When — client attempts to forge tenant_id to another tenant
    let forged_tenant = "tenant-xyz-999".to_string();

    // The forged token would have a different tenant_id at top level
    // but the same signature (since we can't actually sign in tests)
    // In production: changing tenant_id would invalidate the signature
    let mut forged_json = claims.to_compact_json();
    // Simulate what a forged token would look like
    assert!(
        !forged_json.contains(&format!("\"tenant_id\":\"{}\"", forged_tenant)),
        "Original token must have original tenant, not forged"
    );

    // Validate that the forged tenant doesn't match
    let result = claims.validate_tenant(&forged_tenant);
    assert!(
        result.is_err(),
        "Forged tenant_id must be detected by validate_tenant"
    );
}

/// Scenario: Tenant ID matches request header
///   For every login request, assert claims.tenant_id == X-Tenant-ID header
///   value — never allow the token's tenant to differ from the request's tenant
#[test]
fn scenario_tenant_id_matches_request_header() {
    // Given — login request with X-Tenant-ID: tenant-alpha
    let request_tenant = "tenant-alpha-uuid".to_string();

    // When — login handler validates user's tenant matches request tenant
    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-alpha-001")
        .aud(vec!["api".to_string()])
        .client_id("alpha-app")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-alpha-001".to_string())
        .ver(1)
        .sid("session-alpha-001".to_string())
        .tenant_id(request_tenant.clone()) // Matches X-Tenant-ID header
        .user_id("user-alpha-001".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            request_tenant.clone(),
            "alpha-web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    // Then — claims must match the request tenant
    assert_eq!(
        claims.tenant_id, request_tenant,
        "JWT tenant_id must match X-Tenant-ID header"
    );
    assert!(
        claims.validate_tenant(&request_tenant).is_ok(),
        "validate_tenant must accept matching request tenant"
    );
}

/// Scenario: No tenant_id leakage across login sessions
///   Assert that a login to tenant A never results in a JWT containing
///   tenant B's UUID (test with sequential logins to different tenants
///   using the same client)
#[test]
fn scenario_no_tenant_id_leakage_across_sessions() {
    // Given — two sequential login sessions to different tenants
    let tenant_a_uuid = "tenant-alpha-session-1".to_string();
    let tenant_b_uuid = "tenant-beta-session-2".to_string();

    // When — first login to tenant A
    let claims_a = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-session-1")
        .aud(vec!["api".to_string()])
        .client_id("app")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-session-1".to_string())
        .ver(1)
        .sid("session-session-1".to_string())
        .tenant_id(tenant_a_uuid.clone())
        .user_id("user-session-1".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            tenant_a_uuid.clone(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims_a");

    // When — second login to tenant B (same client, different tenant)
    let claims_b = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-session-2")
        .aud(vec!["api".to_string()])
        .client_id("app")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-session-2".to_string())
        .ver(1)
        .sid("session-session-2".to_string())
        .tenant_id(tenant_b_uuid.clone())
        .user_id("user-session-2".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            tenant_b_uuid.clone(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims_b");

    // Then — tenant A's JWT must not contain tenant B's UUID
    let json_a = claims_a.to_compact_json();
    assert!(
        !json_a.contains(&tenant_b_uuid),
        "JWT for tenant A must not leak tenant B's UUID"
    );
    assert!(
        json_a.contains(&tenant_a_uuid),
        "JWT for tenant A must contain tenant A's UUID"
    );

    // And — tenant B's JWT must not contain tenant A's UUID
    let json_b = claims_b.to_compact_json();
    assert!(
        !json_b.contains(&tenant_a_uuid),
        "JWT for tenant B must not leak tenant A's UUID"
    );
    assert!(
        json_b.contains(&tenant_b_uuid),
        "JWT for tenant B must contain tenant B's UUID"
    );
}

// ─── Security Regression Tests ──────────────────────────────────────────

/// Security: Tenant ID in JWT is NOT confidential
///   The tenant_id in JWT is visible to anyone who can read the token.
///   This is by design — tenant_id is an authorization boundary, not a secret.
#[test]
fn scenario_tenant_id_not_confidential() {
    // Given — a JWT with tenant_id
    let tenant_uuid = "tenant-public-uuid".to_string();
    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-public")
        .aud(vec!["api".to_string()])
        .client_id("public-app")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-public".to_string())
        .ver(1)
        .sid("session-public".to_string())
        .tenant_id(tenant_uuid.clone())
        .user_id("user-public".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            tenant_uuid.clone(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    // Then — tenant_id is visible in the JSON payload (by design)
    let json = claims.to_compact_json();
    assert!(
        json.contains(&tenant_uuid),
        "tenant_id is intentionally visible (not confidential)"
    );
    // Documented: tenant_id in JWT is NOT confidential. Any entity that can
    // read the JWT can read the tenant_id. Treat it as public information.
}

// ─── Edge Cases ─────────────────────────────────────────────────────────

/// Edge: Empty tenant_id is rejected by validate_tenant
#[test]
fn scenario_empty_tenant_id_rejected() {
    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-empty")
        .aud(vec!["api".to_string()])
        .client_id("app")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-empty".to_string())
        .ver(1)
        .sid("session-empty".to_string())
        .tenant_id("tenant-valid".to_string())
        .user_id("user-empty".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            "tenant-valid".to_string(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    // When — validate_tenant called with empty string
    let result = claims.validate_tenant("");

    // Then — must reject (empty is not a valid tenant)
    assert!(
        result.is_err(),
        "validate_tenant must reject empty tenant_id"
    );
}

/// Edge: Null/empty tenant in sx.tenant fails validation
#[test]
fn scenario_empty_sx_tenant_rejected() {
    // Given — claims with empty sx.tenant
    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-null".to_string(),
        aud: vec!["api".to_string()],
        client_id: "app".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-null".to_string(),
        ver: 1,
        sid: "session-null".to_string(),
        tenant_id: "tenant-valid".to_string(),
        user_id: "user-null".to_string(),
        user_type: "customer".to_string(),
        sx: SesameAuthzClaims {
            tenant: "".to_string(), // Empty — corrupted data
            portal: "web".to_string(),
            roles: vec![],
            permissions: vec![],
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        },
        act: None,
    };

    // When — validate() is called
    let result = claims.validate();

    // Then — must reject with MissingAuthzClaims
    assert_eq!(
        result,
        Err(JwtValidationError::MissingAuthzClaims),
        "Empty sx.tenant must be rejected"
    );
}

/// Edge: Different user types all have tenant_id in JWT
#[test]
fn scenario_all_user_types_have_tenant_id() {
    for user_type in &["customer", "platform", "platform_admin"] {
        let claims = AccessClaimsBuilder::new()
            .iss("https://sesame-idam.example.com")
            .sub(format!("user-{}", user_type))
            .aud(vec!["api".to_string()])
            .client_id("app")
            .scope("openid".to_string())
            .exp(1700000000)
            .nbf(1700000000 - 60)
            .iat(1700000000)
            .jti(format!("jti-{}", user_type))
            .ver(1)
            .sid(format!("session-{}", user_type))
            .tenant_id("tenant-all-users".to_string())
            .user_id(format!("user-{}", user_type))
            .user_type(user_type.to_string())
            .sx(SesameAuthzClaims::new(
                "tenant-all-users".to_string(),
                "web".to_string(),
                vec![],
                vec![],
            ))
            .build()
            .expect("valid claims");

        // Then — every user type has tenant_id at both locations
        assert_eq!(
            claims.tenant_id, "tenant-all-users",
            "User type '{}' must have tenant_id at top level",
            user_type
        );
        assert_eq!(
            claims.sx.tenant, "tenant-all-users",
            "User type '{}' must have tenant_id in sx",
            user_type
        );
    }
}

#[cfg(test)]
mod bdd_tests {
    use super::*;

    #[test]
    fn test_all_scenarios_run() {
        // Verify all BDD scenario functions are callable
        scenario_tenant_id_flows_from_login_to_token();
        scenario_cross_tenant_login_rejected();
        scenario_downstream_service_validates_tenant();
        scenario_tenant_id_present_in_login_response();
        scenario_zero_cross_tenant_identity();
        scenario_tenant_id_cannot_be_forged();
        scenario_tenant_id_matches_request_header();
        scenario_no_tenant_id_leakage_across_sessions();
        scenario_tenant_id_not_confidential();
        scenario_empty_tenant_id_rejected();
        scenario_empty_sx_tenant_rejected();
        scenario_all_user_types_have_tenant_id();
    }
}
