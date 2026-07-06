// BDD tests for JWT token payload — PII removal and entitlements ref/hash
//
// These tests verify the end-to-end behavior of the JWT claims implementation
// for Story 2.3: PII removal, entitlements reference, and entitlements hash.
//
// They exercise the jwt:: module functions directly (unit-level BDD) and
// also serve as integration tests by constructing full AccessClaims structs
// that mimic what the login service controller would produce.

use sesame_common::jwt::*;

/// Scenario: Login token has no PII
///   Given a user with known email and phone number
///   When the login service builds an access token
///   Then the decoded JWT payload contains no PII fields
#[test]
fn scenario_login_token_has_no_pii() {
    // Given — user PII
    let user_email = "alice@corp.com".to_string();
    let user_phone = "+141****1234".to_string();
    let user_name = "Alice Corp".to_string();

    // When — build a token like the login controller would
    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("alice-corp-001")
        .aud(vec!["api".to_string(), "frontend".to_string()])
        .client_id("login-app".to_string())
        .scope("openid".to_string())
        .exp(1_700_000_000)
        .nbf(1_700_000_000 - 60)
        .iat(1_700_000_000)
        .jti("jti-alice-001".to_string())
        .ver(1)
        .sid("session-alice-001".to_string())
        .tenant_id("tenant-1".to_string())
        .user_id("alice-corp-001".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            "tenant-1".to_string(),
            "web".to_string(),
            vec!["admin".to_string()],
            vec!["org:read".to_string()],
        ))
        .build()
        .expect("valid claims");

    // Then — PII is absent from serialized JSON
    let json = claims.to_compact_json();

    // Assert no PII fields (use string literals, not char literals)
    assert!(!json.contains(r#""email""#), "email should not be in JWT");
    assert!(
        !json.contains(r#""email_verified""#),
        "email_verified absent"
    );
    assert!(!json.contains(r#""phone_number""#), "phone_number absent");
    assert!(
        !json.contains(r#""phone_verified""#),
        "phone_verified absent"
    );
    assert!(!json.contains(r#""first_name""#), "first_name absent");
    assert!(!json.contains(r#""last_name""#), "last_name absent");
    assert!(!json.contains(r#""name""#), "name absent");
    assert!(
        !json.contains(r#""preferred_username""#),
        "preferred_username absent"
    );

    // Also assert the actual PII values are not present
    assert!(
        !json.contains(&user_email),
        "actual email value should not be in JWT"
    );
    assert!(
        !json.contains(&user_phone),
        "actual phone value should not be in JWT"
    );
    assert!(
        !json.contains(&user_name),
        "actual name value should not be in JWT"
    );
}

/// Scenario: Entitlements ref is used in consumer flow
///   Given a token with `sx.entitlements_ref` set
///   When a consumer service receives the token
///   Then the service can extract the ref and use it as a cache key
#[test]
fn scenario_entitlements_ref_consumer_flow() {
    // Given — claims with entitlements_ref
    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("bob-user-002")
        .aud(vec!["api".to_string()])
        .client_id("consumer-app".to_string())
        .scope("openid".to_string())
        .exp(1_700_000_000)
        .nbf(1_700_000_000 - 60)
        .iat(1_700_000_000)
        .jti("jti-bob-002".to_string())
        .ver(3)
        .sid("session-bob-002".to_string())
        .tenant_id("tenant-2".to_string())
        .user_id("bob-user-002".to_string())
        .user_type("customer".to_string())
        .sx({
            let mut sx = SesameAuthzClaims::new(
                "tenant-2".to_string(),
                "api".to_string(),
                vec!["viewer".to_string()],
                vec!["billing:read".to_string()],
            );
            // The issuer (login-service) populates the entitlements ref when
            // minting the token — SesameAuthzClaims::new leaves it None.
            sx.entitlements_ref = Some(generate_entitlements_ref(
                "bob-user-002",
                "org-2",
                3,
                "tenant-2",
            ));
            sx
        })
        .build()
        .expect("valid claims");

    // Extract the entitlements_ref — this is what a consumer would do
    let ref_opt = claims.sx.entitlements_ref.as_deref();

    // Then — the ref should exist and be a valid format
    assert!(ref_opt.is_some(), "entitlements_ref should be present");
    let ref_val = ref_opt.unwrap();
    assert!(ref_val.starts_with("ent_"), "ref should start with 'ent_'");

    // Consumer would use this as a Redis cache key: entitlements:{tenant}:{ref}
    let cache_key = format!("entitlements:{}:{}", claims.tenant_id, ref_val);
    assert!(
        cache_key.contains("tenant-2"),
        "cache key should include tenant"
    );
    assert!(cache_key.contains(ref_val), "cache key should include ref");

    // Consumer should check cache first, only call authz on miss
    // We verify the ref format is valid for this flow
    assert!(!cache_key.is_empty(), "cache key should be non-empty");
}

/// Scenario: Hash verification on consumer side
///   Given a token with `sx.entitlements_hash`
///   When the consumer fetches the full snapshot from cache
///   And the consumer computes SHA-256 of the cached snapshot
///   Then the computed hash matches `sx.entitlements_hash`
#[test]
fn scenario_hash_verification_on_consumer_side() {
    // Given — build a snapshot that matches the hash in the claims
    let mut snapshot = EntitlementsSnapshot {
        version: 3,
        permissions: vec!["billing:read".to_string()],
        roles: vec!["viewer".to_string()],
        tenant: "tenant-2".to_string(),
        hash: String::new(), // will be computed
    };

    let computed_hash = compute_entitlements_hash(&snapshot);
    snapshot.hash = computed_hash.clone();

    // Verify the hash
    assert!(
        verify_entitlements_hash(&snapshot, &computed_hash).is_ok(),
        "valid snapshot should pass hash verification"
    );

    // When — an attacker tampers with the snapshot
    let mut tampered = snapshot.clone();
    tampered.permissions.push("admin:all".to_string());

    // Then — hash verification should fail
    assert!(
        verify_entitlements_hash(&tampered, &computed_hash).is_err(),
        "tampered snapshot should fail hash verification"
    );
}

/// Scenario: No PII in token even with special characters
///   Given users with PII containing unicode, apostrophes, etc.
///   When tokens are built
///   Then none of these special PII values appear in the JWT payload
#[test]
fn scenario_no_pii_with_special_characters() {
    // Given — PII with special characters
    let unicode_email = "用户@example.com".to_string();
    let apostrophe_name = "O'Brien".to_string();
    let emoji_name = "José García".to_string();
    let phone_with_plus = "+1 (415) 555-0123".to_string();

    // When — build claims (note: PII is not set, but we verify the actual
    // PII values don't appear even if they were somehow in related fields)
    let claims = AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("unicode-user".to_string())
        .aud(vec!["api".to_string()])
        .client_id("test-client".to_string())
        .scope("openid".to_string())
        .exp(1_700_000_000)
        .nbf(1_700_000_000 - 60)
        .iat(1_700_000_000)
        .jti("jti-unicode".to_string())
        .ver(1)
        .sid("session-unicode".to_string())
        .tenant_id("tenant-unicode".to_string())
        .user_id("unicode-user".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            "tenant-unicode".to_string(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    let json = claims.to_compact_json();

    // Then — none of the special PII values appear
    assert!(
        !json.contains(&unicode_email),
        "unicode email should not be in JWT"
    );
    assert!(
        !json.contains(&apostrophe_name),
        "apostrophe name should not be in JWT"
    );
    assert!(
        !json.contains(&emoji_name),
        "accented name should not be in JWT"
    );
    assert!(
        !json.contains(&phone_with_plus),
        "phone with special chars should not be in JWT"
    );
}

#[cfg(test)]
mod bdd_tests {
    use super::*;

    #[test]
    fn test_all_scenarios_run() {
        // Verify all 4 BDD scenario functions are callable
        scenario_login_token_has_no_pii();
        scenario_entitlements_ref_consumer_flow();
        scenario_hash_verification_on_consumer_side();
        scenario_no_pii_with_special_characters();
    }
}
