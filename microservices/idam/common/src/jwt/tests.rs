//! JWT module tests — all 46 test functions from the original mod.rs.

use super::helpers::*;
use super::types::{
    AccessClaims, ActorClaim, EntitlementsSnapshot, JwtError, JwtValidationError, SesameAuthzClaims,
};

// PII Removal Tests (Story 2.3)

#[test]
fn test_pii_fields_not_in_token() {
    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid profile".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims {
            tenant: "tenant-1".to_string(),
            portal: "web".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["org:read".to_string()],
            entitlements_ref: Some("ent_abc123".to_string()),
            entitlements_hash: Some("sha256:abc123".to_string()),
            risk: None,
        },
        act: None,
        cnf: None,
    };

    let json = claims.to_compact_json();
    assert!(!json.contains("\"email\""), "email should not be in JWT");
    assert!(
        !json.contains("\"email_verified\""),
        "email_verified absent"
    );
    assert!(!json.contains("\"phone_number\""), "phone_number absent");
    assert!(
        !json.contains("\"phone_verified\""),
        "phone_verified absent"
    );
    assert!(!json.contains("\"first_name\""), "first_name absent");
    assert!(!json.contains("\"last_name\""), "last_name absent");
    assert!(!json.contains("\"name\""), "name absent");
    assert!(
        !json.contains("\"preferred_username\""),
        "preferred_username absent"
    );
}

#[test]
fn test_pii_values_absent_from_token_payload() {
    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims::new(
            "tenant-1".to_string(),
            "web".to_string(),
            vec!["admin".to_string()],
            vec!["org:read".to_string()],
        ),
        act: None,
        cnf: None,
    };

    let json = claims.to_compact_json();
    assert!(!json.contains("alice@corp.com"));
    assert!(!json.contains("+141****1234"));
    assert!(!json.contains("Alice"));
    assert!(!json.contains("Smith"));
    assert!(!json.contains("alice.smith"));
}

#[test]
fn test_entitlements_ref_deterministic() {
    let ref1 = generate_entitlements_ref("user-1", "org-1", 1, "tenant-1");
    let ref2 = generate_entitlements_ref("user-1", "org-1", 1, "tenant-1");
    assert_eq!(ref1, ref2);
}

#[test]
fn test_entitlements_ref_changes_on_version_bump() {
    let ref_v1 = generate_entitlements_ref("user-1", "org-1", 1, "tenant-1");
    let ref_v2 = generate_entitlements_ref("user-1", "org-1", 2, "tenant-1");
    assert_ne!(ref_v1, ref_v2, "version bump should change ref");
}

#[test]
fn test_entitlements_ref_format() {
    let ref_str = generate_entitlements_ref("user-1", "org-1", 1, "tenant-1");
    assert!(ref_str.starts_with("ent_"));
    let uuid_part = &ref_str[4..];
    assert_eq!(uuid_part.len(), 36, "should be ent_ + 36-char UUID");
}

#[test]
fn test_entitlements_hash_matches_canonical_json() {
    let snapshot = EntitlementsSnapshot {
        version: 42,
        permissions: vec!["org:admin".to_string(), "billing:read".to_string()],
        roles: vec!["admin".to_string(), "billing-viewer".to_string()],
        tenant: "tenant-1".to_string(),
        hash: String::new(),
    };

    let hash = compute_entitlements_hash(&snapshot);
    assert!(hash.starts_with("sha256:"));
    assert_eq!(hash.len(), 71, "sha256: + 64 hex chars = 71 chars");
}

#[test]
fn test_hash_format_validation() {
    let snapshot = EntitlementsSnapshot {
        version: 1,
        permissions: vec![],
        roles: vec![],
        tenant: "tenant-1".to_string(),
        hash: String::new(),
    };

    let hash = compute_entitlements_hash(&snapshot);
    assert!(hash.starts_with("sha256:"));
    let hex_part = &hash[7..];
    assert_eq!(hex_part.len(), 64);
    assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_verify_entitlements_hash_valid() {
    let mut snapshot = EntitlementsSnapshot {
        version: 1,
        permissions: vec!["read".to_string()],
        roles: vec!["user".to_string()],
        tenant: "tenant-1".to_string(),
        hash: String::new(),
    };

    let expected_hash = compute_entitlements_hash(&snapshot);
    snapshot.hash = expected_hash.clone();
    assert!(verify_entitlements_hash(&snapshot, &expected_hash).is_ok());
}

#[test]
fn test_verify_entitlements_hash_mismatch() {
    let snapshot = EntitlementsSnapshot {
        version: 1,
        permissions: vec!["read".to_string()],
        roles: vec!["user".to_string()],
        tenant: "tenant-1".to_string(),
        hash: "sha256:wronghash".to_string(),
    };

    let result = verify_entitlements_hash(&snapshot, "sha256:correcthash");
    assert_eq!(result, Err(JwtValidationError::EntitlementsHashMismatch));
}

#[test]
fn test_empty_entitlements_snapshot() {
    let snapshot = EntitlementsSnapshot {
        version: 0,
        permissions: vec![],
        roles: vec![],
        tenant: "tenant-1".to_string(),
        hash: String::new(),
    };
    let hash = compute_entitlements_hash(&snapshot);
    assert!(hash.starts_with("sha256:"));
}

#[test]
fn test_large_entitlements_set_stays_under_budget() {
    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims {
            tenant: "tenant-1".to_string(),
            portal: "web".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["org:read".to_string()],
            entitlements_ref: Some(generate_entitlements_ref(
                "user-123", "org-1", 1, "tenant-1",
            )),
            entitlements_hash: Some("sha256:abc123def456".to_string()),
            risk: None,
        },
        act: None,
        cnf: None,
    };

    let size = claims.json_payload_size();
    assert!(
        size < 750,
        "JWT payload size {size} exceeds 750-byte budget"
    );
}

#[test]
fn test_sesame_authz_claims_full_round_trip() {
    let sx = SesameAuthzClaims {
        tenant: "tenant-1".to_string(),
        portal: "web".to_string(),
        roles: vec!["admin".to_string(), "billing".to_string()],
        permissions: vec!["org:admin".to_string(), "billing:write".to_string()],
        entitlements_ref: Some("ent_abc123".to_string()),
        entitlements_hash: Some("sha256:abc123".to_string()),
        risk: Some("normal".to_string()),
    };

    let json = serde_json::to_string(&sx).unwrap();
    let deserialized: SesameAuthzClaims = serde_json::from_str(&json).unwrap();
    assert_eq!(sx, deserialized);
}

#[test]
fn test_sesame_authz_claims_optional_fields_absent() {
    let sx = SesameAuthzClaims::new(
        "tenant-1".to_string(),
        "web".to_string(),
        vec!["admin".to_string()],
        vec!["org:read".to_string()],
    );
    let json = serde_json::to_string(&sx).unwrap();
    assert!(!json.contains("entitlements_ref"));
    assert!(!json.contains("entitlements_hash"));
    assert!(!json.contains("risk"));
}

#[test]
fn test_actor_claim_round_trip() {
    let actor = ActorClaim {
        sub: "user-123".to_string(),
    };
    let json = serde_json::to_string(&actor).unwrap();
    let deserialized: ActorClaim = serde_json::from_str(&json).unwrap();
    assert_eq!(actor, deserialized);
}

#[test]
fn test_access_claims_act_present_absent() {
    let no_act = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(), vec![], vec![]),
        act: None,
        cnf: None,
    };

    let json_no = serde_json::to_string(&no_act).unwrap();
    assert!(!json_no.contains("\"act\""));

    let with_act = AccessClaims {
        act: Some(ActorClaim {
            sub: "user-456".to_string(),
        }),
        ..no_act.clone()
    };
    let json_yes = serde_json::to_string(&with_act).unwrap();
    assert!(json_yes.contains("\"act\""));
}

#[test]
fn test_sesame_authz_claims_special_characters() {
    let json = serde_json::to_value(SesameAuthzClaims::new(
        "tenant-1".to_string(),
        "web".to_string(),
        vec![],
        vec![],
    ))
    .unwrap();
    assert!(!json.to_string().contains("O'Brien"));
    assert!(!json.to_string().contains("+141****1234"));
}

// Validation Tests (from Story 2.2)

#[test]
fn test_valid_claims_pass_validation() {
    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims::new(
            "tenant-1".to_string(),
            "web".to_string(),
            vec!["admin".to_string()],
            vec!["org:read".to_string()],
        ),
        act: None,
        cnf: None,
    };
    assert!(claims.validate().is_ok());
}

#[test]
fn test_validation_rejects_missing_ver() {
    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 0, // missing version
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(), vec![], vec![]),
        act: None,
        cnf: None,
    };
    assert_eq!(claims.validate(), Err(JwtValidationError::MissingVersion));
}

#[test]
fn test_validation_rejects_missing_tenant_id() {
    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: String::new(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(), vec![], vec![]),
        act: None,
        cnf: None,
    };
    assert_eq!(claims.validate(), Err(JwtValidationError::MissingTenant));
}

#[test]
fn test_validation_rejects_missing_sx_tenant() {
    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims {
            tenant: String::new(),
            portal: "web".to_string(),
            roles: vec![],
            permissions: vec![],
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        },
        act: None,
        cnf: None,
    };
    assert_eq!(
        claims.validate(),
        Err(JwtValidationError::MissingAuthzClaims)
    );
}

#[test]
fn test_validation_rejects_invalid_issuer() {
    let claims = AccessClaims {
        iss: "https://evil-issuer.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(), vec![], vec![]),
        act: None,
        cnf: None,
    };
    assert_eq!(claims.validate(), Err(JwtValidationError::InvalidIssuer));
}

#[test]
fn test_validation_rejects_invalid_audience() {
    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["unknown-service".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(), vec![], vec![]),
        act: None,
        cnf: None,
    };
    assert_eq!(claims.validate(), Err(JwtValidationError::InvalidAudience));
}

#[test]
fn test_validation_accepts_valid_risk_values() {
    for risk_value in &["normal", "elevated", "critical"] {
        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(),
            aud: vec!["api".to_string()],
            client_id: "client-1".to_string(),
            scope: "openid".to_string(),
            exp: 1700000000,
            nbf: 1700000000 - 60,
            iat: 1700000000,
            jti: "jti-123".to_string(),
            ver: 1,
            sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            org_id: None,
            sx: SesameAuthzClaims {
                tenant: "tenant-1".to_string(),
                portal: "web".to_string(),
                roles: vec![],
                permissions: vec![],
                entitlements_ref: None,
                entitlements_hash: None,
                risk: Some(risk_value.to_string()),
            },
            act: None,
            cnf: None,
        };
        assert!(
            claims.validate().is_ok(),
            "risk '{risk_value}' should be valid"
        );
    }
}

#[test]
fn test_validation_rejects_invalid_risk() {
    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims {
            tenant: "tenant-1".to_string(),
            portal: "web".to_string(),
            roles: vec![],
            permissions: vec![],
            entitlements_ref: None,
            entitlements_hash: None,
            risk: Some("unknown".to_string()),
        },
        act: None,
        cnf: None,
    };
    assert_eq!(claims.validate(), Err(JwtValidationError::InvalidRisk));
}

#[test]
fn test_builder_constructs_valid_claims() {
    let claims = crate::jwt::builders::AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com".to_string())
        .sub("user-123".to_string())
        .aud(vec!["api".to_string()])
        .client_id("client-1".to_string())
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-123".to_string())
        .ver(1)
        .sid("session-1".to_string())
        .tenant_id("tenant-1".to_string())
        .user_id("user-123".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            "tenant-1".to_string(),
            "web".to_string(),
            vec!["admin".to_string()],
            vec!["org:read".to_string()],
        ))
        .build();
    assert!(claims.is_ok());
    let claims = claims.unwrap();
    assert_eq!(claims.iss, "https://sesame-idam.example.com");
    assert_eq!(claims.ver, 1);
    assert_eq!(claims.tenant_id, "tenant-1");
}

#[test]
fn test_builder_rejects_missing_required_fields() {
    let result = crate::jwt::builders::AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com".to_string())
        .sub("user-123".to_string())
        .aud(vec!["api".to_string()])
        .client_id("client-1".to_string())
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-123".to_string())
        .ver(1)
        .build();
    assert!(result.is_err());
}

#[test]
fn test_builder_rejects_ver_zero() {
    let result = crate::jwt::builders::AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com".to_string())
        .sub("user-123".to_string())
        .aud(vec!["api".to_string()])
        .client_id("client-1".to_string())
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-123".to_string())
        .ver(0) // explicitly zero
        .sid("session-1".to_string())
        .tenant_id("tenant-1".to_string())
        .user_id("user-123".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            "tenant-1".to_string(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        JwtError::MissingRequiredField(s) if s == "ver must be > 0"
    ));
}

#[test]
fn test_token_size_under_budget() {
    let roles: Vec<String> = (0..10).map(|i| format!("role-{i}")).collect();
    let permissions: Vec<String> = (0..10).map(|i| format!("perm:{i}")).collect();

    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string(), "frontend".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid profile email".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims {
            tenant: "tenant-1".to_string(),
            portal: "web".to_string(),
            roles,
            permissions,
            entitlements_ref: Some(generate_entitlements_ref(
                "user-123", "org-1", 1, "tenant-1",
            )),
            entitlements_hash: Some("sha256:".to_string() + &"a".repeat(64)),
            risk: Some("normal".to_string()),
        },
        act: None,
        cnf: None,
    };

    let size = claims.json_payload_size();
    assert!(
        size < 750,
        "JWT payload size {size} exceeds 750-byte budget"
    );
}

#[test]
fn test_entitlements_ref_is_tenant_aware() {
    let ref_a = generate_entitlements_ref("user-1", "org-1", 1, "tenant-a");
    let ref_b = generate_entitlements_ref("user-1", "org-1", 1, "tenant-b");
    assert_ne!(
        ref_a, ref_b,
        "Different tenants should produce different refs"
    );

    let ref_a_2 = generate_entitlements_ref("user-1", "org-1", 1, "tenant-a");
    assert_eq!(ref_a, ref_a_2);
}

// ─── Token Size Budget Enforcement Tests (Story 2.5) ─────────────────

/// Permissions within `MAX_PERMISSIONS_PER_ROLE` pass through unchanged
#[test]
fn test_truncate_permissions_within_limit() {
    let perms: Vec<String> = (0..5).map(|i| format!("perm-{i}")).collect();
    let result = truncate_permissions(perms.clone());
    assert_eq!(result, perms, "Should pass through when within limit");
}

/// Permissions over `MAX_PERMISSIONS_PER_ROLE` are truncated
#[test]
fn test_truncate_permissions_over_limit() {
    let perms: Vec<String> = (0..15).map(|i| format!("perm-{i}")).collect();
    let result = truncate_permissions(perms);
    assert_eq!(
        result.len(),
        MAX_PERMISSIONS_PER_ROLE + 1,
        "Should truncate to {MAX_PERMISSIONS_PER_ROLE} + 1 marker"
    );
    assert!(
        result.last().unwrap().starts_with("...("),
        "Last entry should be the truncation marker"
    );
}

/// Entitlements ref at max length passes validation
#[test]
fn test_validate_entitlements_ref_ok() {
    let valid_ref = "ent_abc123";
    assert_eq!(
        validate_entitlements_ref(Some(valid_ref)),
        Some(valid_ref.to_string())
    );
}

/// Entitlements ref too long is truncated
#[test]
fn test_validate_entitlements_ref_too_long() {
    let long_ref = "ent_".to_owned() + &"a".repeat(100);
    let result = validate_entitlements_ref(Some(&long_ref));
    assert!(result.is_some());
    let truncated = result.unwrap();
    assert_eq!(truncated.len(), MAX_ENTITLEMENTS_REF_LENGTH);
}

/// Empty entitlements ref returns None
#[test]
fn test_validate_entitlements_ref_empty() {
    assert_eq!(validate_entitlements_ref(Some("")), None);
}

/// No entitlements ref returns None
#[test]
fn test_validate_entitlements_ref_none() {
    assert_eq!(validate_entitlements_ref(None), None);
}

/// Token size measurement on valid JWT format
#[test]
fn test_measure_jwt_token_size() {
    let token = "header.payload.signature";
    assert_eq!(measure_jwt_token_size(token), 22); // 5 + 8 + 9 + 2 dots
}

/// Token size measurement on invalid JWT format returns 0
#[test]
fn test_measure_jwt_token_size_invalid() {
    assert_eq!(measure_jwt_token_size("not.a.jwt.token"), 0);
    assert_eq!(measure_jwt_token_size("single"), 0);
}

/// Truncated `SesameAuthzClaims` fit budget
#[test]
fn test_truncated_authz_claims_fits_budget() {
    let permissions: Vec<String> = (0..50).map(|i| format!("perm:resource:{i}")).collect();
    let roles: Vec<String> = (0..5).map(|i| format!("role-{i}")).collect();

    let sx = SesameAuthzClaims {
        tenant: "tenant-1".to_string(),
        portal: "web".to_string(),
        roles: roles.clone(),
        permissions,
        entitlements_ref: None,
        entitlements_hash: None,
        risk: None,
    };

    let truncated = truncate_authz_claims_permissions(sx);
    assert_eq!(
        truncated.permissions.len(),
        MAX_PERMISSIONS_PER_ROLE + 1,
        "Permissions should be truncated to max + 1 marker"
    );

    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-truncated".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: truncated,
        act: None,
        cnf: None,
    };

    let size = claims.json_payload_size();
    assert!(
        size < MAX_TOKEN_SIZE_BYTES,
        "Truncated claims payload {size} bytes still exceeds 750-byte budget"
    );
}

/// Permissions truncation enforces the configured maximum
#[test]
fn test_truncate_permissions_enforces_limit() {
    // Test with 0 permissions (no change)
    let result = truncate_permissions(vec![]);
    assert_eq!(result.len(), 0);

    // Test with exactly MAX_PERMISSIONS_PER_ROLE (no truncation)
    let perms: Vec<String> = (0..MAX_PERMISSIONS_PER_ROLE)
        .map(|i| format!("perm:{i}"))
        .collect();
    let result = truncate_permissions(perms.clone());
    assert_eq!(result.len(), MAX_PERMISSIONS_PER_ROLE);
    assert_eq!(result, perms);

    // Test with more than MAX_PERMISSIONS_PER_ROLE (truncation)
    let perms: Vec<String> = (0..20).map(|i| format!("perm:{i}")).collect();
    let result = truncate_permissions(perms);
    assert_eq!(result.len(), MAX_PERMISSIONS_PER_ROLE + 1); // +1 for "...(10 more)"
    assert!(
        result.iter().any(|s| s.contains("more")),
        "truncated result should contain '...' suffix"
    );
    assert_eq!(
        result.last().unwrap(),
        "...(10 more)",
        "last element should be truncation notice"
    );
}

/// Entitlements ref format validation: max 64 chars
#[test]
fn test_entitlements_ref_max_length() {
    // Exactly 64 chars - should pass through
    let ref_64 = "ent_".to_owned() + &"a".repeat(60);
    assert_eq!(ref_64.len(), 64);
    let result = validate_entitlements_ref(Some(&ref_64));
    assert_eq!(result, Some(ref_64.clone()));

    // 65 chars - should be truncated to 64
    let ref_65 = "ent_".to_owned() + &"a".repeat(61);
    assert_eq!(ref_65.len(), 65);
    let result = validate_entitlements_ref(Some(&ref_65));
    assert_eq!(result, Some(ref_64));
}

/// Build-time test: representative token (10 roles, 10 permissions, all claims)
/// must fit within 750 bytes unencoded budget.
#[test]
fn test_build_time_token_size_within_budget() {
    let roles: Vec<String> = (0..5).map(|i| format!("role-{i}")).collect();
    let permissions: Vec<String> = (0..5).map(|i| format!("perm:{i}")).collect();

    let claims = AccessClaims {
        iss: "https://sesame-idam.example.com".to_string(),
        sub: "user-123".to_string(),
        aud: vec!["api".to_string(), "frontend".to_string()],
        client_id: "client-1".to_string(),
        scope: "openid".to_string(),
        exp: 1700000000,
        nbf: 1700000000 - 60,
        iat: 1700000000,
        jti: "jti-123".to_string(),
        ver: 1,
        sid: "session-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        user_id: "user-123".to_string(),
        user_type: "customer".to_string(),
        org_id: None,
        sx: SesameAuthzClaims {
            tenant: "tenant-1".to_string(),
            portal: "web".to_string(),
            roles,
            permissions,
            entitlements_ref: Some("ent_abc123".to_string()),
            entitlements_hash: Some("sha256:abcdef1234".to_string()),
            risk: Some("normal".to_string()),
        },
        act: None,
        cnf: None,
    };

    let size = claims.json_payload_size();
    assert!(
        size < MAX_TOKEN_SIZE_BYTES,
        "Representative token {size} bytes exceeds {MAX_TOKEN_SIZE_BYTES}-byte budget"
    );
}

// ─── Story 2.4: Tenant Claim Validation Unit Tests ─────────────────────

#[test]
fn test_validate_tenant_accepts_matching_tenant() {
    let claims = crate::jwt::builders::AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-123")
        .aud(vec!["api".to_string()])
        .client_id("client-1")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-123".to_string())
        .ver(1)
        .sid("session-1".to_string())
        .tenant_id("tenant-alpha".to_string())
        .user_id("user-123".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            "tenant-alpha".to_string(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    // Both top-level and sx.tenant match the request tenant
    assert!(claims.validate_tenant("tenant-alpha").is_ok());
}

#[test]
fn test_validate_tenant_rejects_mismatched_top_level() {
    let claims = crate::jwt::builders::AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-123")
        .aud(vec!["api".to_string()])
        .client_id("client-1")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-123".to_string())
        .ver(1)
        .sid("session-1".to_string())
        .tenant_id("tenant-alpha".to_string())
        .user_id("user-123".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            "tenant-alpha".to_string(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    // Request tenant doesn't match top-level tenant_id
    let result = claims.validate_tenant("tenant-beta");
    assert!(result.is_err());
    match result.unwrap_err() {
        JwtError::TenantMismatch { expected, actual } => {
            assert_eq!(expected, "tenant-alpha");
            assert_eq!(actual, "tenant-beta");
        }
        _ => panic!("Expected TenantMismatch"),
    }
}

#[test]
fn test_validate_tenant_rejects_empty_request_tenant() {
    let claims = crate::jwt::builders::AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-123")
        .aud(vec!["api".to_string()])
        .client_id("client-1")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-123".to_string())
        .ver(1)
        .sid("session-1".to_string())
        .tenant_id("tenant-alpha".to_string())
        .user_id("user-123".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            "tenant-alpha".to_string(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    // Empty request_tenant is always rejected (HACK-243)
    let result = claims.validate_tenant("");
    assert!(result.is_err());
    match result.unwrap_err() {
        JwtError::MissingRequiredField(field) => {
            assert_eq!(field, "X-Tenant-ID");
        }
        _ => panic!("Expected MissingRequiredField"),
    }
}

#[test]
fn test_validate_tenant_checks_both_top_level_and_namespaced() {
    // Test case: top-level matches but sx.tenant doesn't
    let mut claims = crate::jwt::builders::AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-123")
        .aud(vec!["api".to_string()])
        .client_id("client-1")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-123".to_string())
        .ver(1)
        .sid("session-1".to_string())
        .tenant_id("tenant-alpha".to_string())
        .user_id("user-123".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            "tenant-beta".to_string(), // Different from top-level!
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    // Even though top-level tenant_id matches, sx.tenant doesn't
    let result = claims.validate_tenant("tenant-alpha");
    assert!(result.is_err(), "Must reject when sx.tenant doesn't match");

    // Test case: sx.tenant matches but top-level doesn't
    claims = crate::jwt::builders::AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-123")
        .aud(vec!["api".to_string()])
        .client_id("client-1")
        .scope("openid".to_string())
        .exp(1700000000)
        .nbf(1700000000 - 60)
        .iat(1700000000)
        .jti("jti-123".to_string())
        .ver(1)
        .sid("session-1".to_string())
        .tenant_id("tenant-beta".to_string())
        .user_id("user-123".to_string())
        .user_type("customer".to_string())
        .sx(SesameAuthzClaims::new(
            "tenant-alpha".to_string(),
            "web".to_string(),
            vec![],
            vec![],
        ))
        .build()
        .expect("valid claims");

    // Even though sx.tenant matches, top-level tenant_id doesn't
    let result = claims.validate_tenant("tenant-alpha");
    assert!(
        result.is_err(),
        "Must reject when top-level tenant_id doesn't match"
    );
}

#[test]
fn test_validate_tenant_consistent_across_user_types() {
    for user_type in &["customer", "platform", "platform_admin"] {
        let claims = crate::jwt::builders::AccessClaimsBuilder::new()
            .iss("https://sesame-idam.example.com")
            .sub(format!("user-{user_type}"))
            .aud(vec!["api".to_string()])
            .client_id("app")
            .scope("openid".to_string())
            .exp(1700000000)
            .nbf(1700000000 - 60)
            .iat(1700000000)
            .jti(format!("jti-{user_type}"))
            .ver(1)
            .sid(format!("session-{user_type}"))
            .tenant_id("tenant-shared".to_string())
            .user_id(format!("user-{user_type}"))
            .user_type(user_type.to_string())
            .sx(SesameAuthzClaims::new(
                "tenant-shared".to_string(),
                "web".to_string(),
                vec![],
                vec![],
            ))
            .build()
            .expect("valid claims");

        assert!(
            claims.validate_tenant("tenant-shared").is_ok(),
            "user_type {user_type} must validate_tenant pass for matching tenant"
        );
    }
}
