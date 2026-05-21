/// Edge Case Tests for Story 2.5: Token Size Budget Enforcement
///
/// These tests cover edge cases from the story doc:
/// - Empty token payload (minimum possible size)
/// - Maximum roles/permissions at budget limit
/// - Token size stability across rebuilds
/// - Empty permissions array
/// - Maximum entitlements_ref length
/// - Multiple very short fields

use rstest::rstest;

use sesame_common::jwt::{
    AccessClaims, MAX_TOKEN_SIZE_BYTES, TOKEN_SIZE_WARNING_BYTES, TOKEN_SIZE_ALERT_BYTES,
    MAX_ENTITLEMENTS_REF_LENGTH, MAX_PERMISSIONS_PER_ROLE,
};

// ─── Edge Case: Empty token payload (minimum possible size) ─────────────────

/// Create a JWT with only the minimum required claims.
/// Assert the size is the minimum possible (approximately the baseline of ~350 bytes).
#[rstest]
#[case("minimum required claims only")]
fn test_empty_token_payload_minimum_size(#[case] _name: &str) {
    let claims = AccessClaims {
        iss: "https://idam.sesame.local".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api.sesame.local".to_string()],
        exp: 9999999999,
        iat: 1700000000,
        jti: "test-jti-min".to_string(),
        typ: Some("at+jwt".to_string()),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        sx: sesame_common::jwt::NamespacedClaims {
            tenant: "tenant-1".to_string(),
            roles: vec![],  // Empty roles
            permissions: vec![],  // Empty permissions
            risk: "low".to_string(),
            entitl: None,
            enthash: None,
        },
    };

    let token_json = serde_json::to_string(&claims).unwrap();
    let token_size = token_json.len();
    
    // Verify minimum size is reasonable (should be > 200 bytes due to required fields)
    assert!(
        token_size > 200,
        "Minimum token size {} should be > 200 bytes",
        token_size
    );
    
    // Verify it's well under the budget (headroom for roles/permissions)
    assert!(
        token_size < MAX_TOKEN_SIZE_BYTES / 2,
        "Minimum token size {} should be under half the budget",
        token_size
    );
}

// ─── Edge Case: Maximum roles/permissions at budget limit ────────────────────

/// Create a token with the maximum number of roles/permissions that still fits
/// under 750 bytes. Assert the size is close to (but not exceeding) the budget,
/// confirming the budget is being utilized effectively.
#[rstest]
#[case("max roles at budget limit")]
fn test_max_roles_permissions_at_budget_limit(#[case] _name: &str) {
    // Start with a baseline token and add roles/permissions until we approach the budget
    let mut roles: Vec<String> = Vec::new();
    let mut permissions: Vec<String> = Vec::new();
    
    // Build up incrementally
    let mut size = 0;
    let mut role_count = 0;
    let mut perm_count = 0;
    
    while size < MAX_TOKEN_SIZE_BYTES {
        // Add a role
        roles.push(format!("role-{}", role_count));
        role_count += 1;
        
        // Add a permission
        permissions.push(format!("perm-{}", perm_count));
        perm_count += 1;
        
        // Recalculate size
        let claims = AccessClaims {
            iss: "https://idam.sesame.local".to_string(),
            sub: "user-test".to_string(),
            aud: vec!["api.sesame.local".to_string()],
            exp: 9999999999,
            iat: 1700000000,
            jti: "test-jti-budget".to_string(),
            typ: Some("at+jwt".to_string()),
            ver: 1,
            sid: "session-budget".to_string(),
            tenant_id: "tenant-test".to_string(),
            sx: sesame_common::jwt::NamespacedClaims {
                tenant: "tenant-test".to_string(),
                roles: roles.clone(),
                permissions: permissions.clone(),
                risk: "low".to_string(),
                entitl: None,
                enthash: None,
            },
        };
        
        let token_json = serde_json::to_string(&claims).unwrap();
        size = token_json.len();
        
        // Safety check: if we're way over budget, stop
        if size > MAX_TOKEN_SIZE_BYTES * 2 {
            break;
        }
    }
    
    // Create the final token with the last valid configuration
    let claims = AccessClaims {
        iss: "https://idam.sesame.local".to_string(),
        sub: "user-test".to_string(),
        aud: vec!["api.sesame.local".to_string()],
        exp: 9999999999,
        iat: 1700000000,
        jti: "test-jti-budget".to_string(),
        typ: Some("at+jwt".to_string()),
        ver: 1,
        sid: "session-budget".to_string(),
        tenant_id: "tenant-test".to_string(),
        sx: sesame_common::jwt::NamespacedClaims {
            tenant: "tenant-test".to_string(),
            roles,
            permissions,
            risk: "low".to_string(),
            entitl: None,
            enthash: None,
        },
    };
    
    let token_json = serde_json::to_string(&claims).unwrap();
    let token_size = token_json.len();
    
    // The token should be close to the budget (within 10%)
    assert!(
        token_size <= MAX_TOKEN_SIZE_BYTES,
        "Token size {} should be within budget of {}",
        token_size,
        MAX_TOKEN_SIZE_BYTES
    );
    
    // Verify the budget is being utilized effectively (> 50% of budget)
    assert!(
        token_size as f64 > MAX_TOKEN_SIZE_BYTES as f64 * 0.5,
        "Token size {} should utilize at least 50% of budget ({} bytes)",
        token_size,
        MAX_TOKEN_SIZE_BYTES
    );
}

// ─── Edge Case: Token size stability across rebuilds ─────────────────────────

/// Run the token size calculation multiple times with the same claims.
/// Assert the token size does not drift between runs (no random UUIDs or timestamps
/// in the representative claims structure).
#[rstest]
#[case("stability across rebuilds")]
fn test_token_size_stability_across_rebuilds(#[case] _name: &str) {
    let claims_template = AccessClaims {
        iss: "https://idam.sesame.local".to_string(),
        sub: "user-stable".to_string(),
        aud: vec!["api.sesame.local".to_string()],
        exp: 9999999999,
        iat: 1700000000,
        jti: "test-jti-stable".to_string(),
        typ: Some("at+jwt".to_string()),
        ver: 1,
        sid: "session-stable".to_string(),
        tenant_id: "tenant-stable".to_string(),
        sx: sesame_common::jwt::NamespacedClaims {
            tenant: "tenant-stable".to_string(),
            roles: vec!["role-1".to_string(), "role-2".to_string()],
            permissions: vec!["perm-1".to_string(), "perm-2".to_string()],
            risk: "low".to_string(),
            entitl: None,
            enthash: None,
        },
    };
    
    // Serialize the same claims 100 times
    let mut sizes: Vec<usize> = Vec::new();
    for _ in 0..100 {
        let token_json = serde_json::to_string(&claims_template).unwrap();
        sizes.push(token_json.len());
    }
    
    // All sizes should be identical (stable)
    let first_size = sizes[0];
    for (i, &size) in sizes.iter().enumerate() {
        assert!(
            size == first_size,
            "Token size at iteration {} drifted: {} != {}",
            i,
            size,
            first_size
        );
    }
    
    // Verify the size is deterministic and reasonable
    assert!(
        first_size > 200 && first_size < 1000,
        "Token size {} should be in reasonable range",
        first_size
    );
}

// ─── Edge Case: Empty permissions array ─────────────────────────────────────

/// Verify that a token with no permissions still meets the budget.
#[rstest]
#[case("empty permissions array")]
fn test_empty_permissions_array(#[case] _name: &str) {
    let claims = AccessClaims {
        iss: "https://idam.sesame.local".to_string(),
        sub: "user-test".to_string(),
        aud: vec!["api.sesame.local".to_string()],
        exp: 9999999999,
        iat: 1700000000,
        jti: "test-jti-empty".to_string(),
        typ: Some("at+jwt".to_string()),
        ver: 1,
        sid: "session-empty".to_string(),
        tenant_id: "tenant-test".to_string(),
        sx: sesame_common::jwt::NamespacedClaims {
            tenant: "tenant-test".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec![],  // Empty permissions
            risk: "low".to_string(),
            entitl: None,
            enthash: None,
        },
    };
    
    let token_json = serde_json::to_string(&claims).unwrap();
    assert!(
        token_json.len() <= MAX_TOKEN_SIZE_BYTES,
        "Token with empty permissions should fit budget (got {} bytes)",
        token_json.len()
    );
}

// ─── Edge Case: Maximum entitlements_ref length ─────────────────────────────

/// Verify that an entitlements_ref at the maximum allowed length doesn't
/// cause budget overflow.
#[rstest]
#[case("max entitlements_ref length")]
fn test_max_entitlements_ref_length(#[case] _name: &str) {
    assert_eq!(
        MAX_ENTITLEMENTS_REF_LENGTH, 64,
        "MAX_ENTITLEMENTS_REF_LENGTH should be 64"
    );
    
    let claims = AccessClaims {
        iss: "https://idam.sesame.local".to_string(),
        sub: "user-test".to_string(),
        aud: vec!["api.sesame.local".to_string()],
        exp: 9999999999,
        iat: 1700000000,
        jti: "test-jti-ref".to_string(),
        typ: Some("at+jwt".to_string()),
        ver: 1,
        sid: "session-ref".to_string(),
        tenant_id: "tenant-test".to_string(),
        sx: sesame_common::jwt::NamespacedClaims {
            tenant: "tenant-test".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["read".to_string()],
            risk: "low".to_string(),
            entitl: Some("a".repeat(MAX_ENTITLEMENTS_REF_LENGTH)),
            enthash: Some("sha256:abcdef1234567890".to_string()),
        },
    };
    
    let token_json = serde_json::to_string(&claims).unwrap();
    let token_size = token_json.len();
    
    // Verify the token with max-length ref still fits budget
    assert!(
        token_size <= MAX_TOKEN_SIZE_BYTES,
        "Token with max-length entitlements_ref ({}) should fit budget (got {} bytes)",
        MAX_ENTITLEMENTS_REF_LENGTH,
        token_size
    );
}

// ─── Edge Case: Multiple very short fields ──────────────────────────────────

/// Verify that even with ultra-short field values, required fields take up
/// a minimum amount of space.
#[rstest]
#[case("ultra-short fields")]
fn test_ultra_short_fields_minimum_size(#[case] _name: &str) {
    let claims = AccessClaims {
        iss: "https://i.sl".to_string(),  // Ultra-short issuer
        sub: "u".to_string(),
        aud: vec!["a".to_string()],
        exp: 9999999999,
        iat: 1700000000,
        jti: "j".to_string(),
        typ: Some("a".to_string()),
        ver: 1,
        sid: "s".to_string(),
        tenant_id: "t".to_string(),
        sx: sesame_common::jwt::NamespacedClaims {
            tenant: "t".to_string(),
            roles: vec!["r".to_string()],
            permissions: vec!["p".to_string()],
            risk: "r".to_string(),
            entitl: None,
            enthash: None,
        },
    };
    
    let token_json = serde_json::to_string(&claims).unwrap();
    let token_size = token_json.len();
    
    // Even with minimal values, required fields have a minimum JSON size
    assert!(
        token_size > 150,
        "Token with ultra-short fields should still have minimum size (got {} bytes)",
        token_size
    );
}
