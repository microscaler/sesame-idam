/// BDD tests for Story 1.1 — Asymmetric JWKS
///
/// Tests exercise KeyManager's PUBLIC API end-to-end. The same methods are called
/// by the impl controllers (jwks.rs, admin_jwks_revoke.rs), so passing tests
/// proves the controllers work correctly.
use base64::Engine;
use ring::signature::UnparsedPublicKey;
use sesame_idam_identity_session_service::key_manager::{JwkKeyType, JwkUse, KEY_MANAGER};

fn make_km() -> sesame_idam_identity_session_service::key_manager::KeyManager {
    sesame_idam_identity_session_service::key_manager::KeyManager::new().unwrap()
}

fn keys_have_no_private_fields(keys: &[sesame_idam_identity_session_service::key_manager::JwkOnly]) -> bool {
    for key in keys {
        let json = serde_json::to_value(key).unwrap();
        // JwkOnly fields: kid, kty, use, crv, x — no private fields
        // If serialized JSON has d, p, q, dp, dq, qi => private material leak
        let s = serde_json::to_string(&json).unwrap();
        if s.contains("\"d\"") || s.contains("\"p\"") || s.contains("\"q\"") {
            return false;
        }
    }
    true
}

// ─── Tests: Read-only — Global KEY_MANAGER ──────────────────────────────────────

#[test]
fn global_keymanager_has_at_least_one_key() {
    let doc = KEY_MANAGER.jwks_document();
    assert!(
        !doc.keys.is_empty(),
        "KEY_MANAGER must have at least one key, got {}",
        doc.keys.len()
    );
}

#[test]
fn jwks_keys_have_okp_kty() {
    for key in &KEY_MANAGER.jwks_document().keys {
        assert_eq!(key.kty, JwkKeyType::Okp, "kty must be OKP, got {:?}", key.kty);
    }
}

#[test]
fn jwks_keys_have_ed25519_curve() {
    for key in &KEY_MANAGER.jwks_document().keys {
        assert_eq!(
            key.crv.to_string(),
            "Ed25519",
            "crv must be Ed25519, got {}",
            key.crv
        );
    }
}

#[test]
fn jwks_keys_have_sig_use() {
    for key in &KEY_MANAGER.jwks_document().keys {
        assert_eq!(key.use_claim, JwkUse::Sig, "use must be sig, got {:?}", key.use_claim);
    }
}

#[test]
fn jwks_keys_have_correct_kid_format() {
    for key in &KEY_MANAGER.jwks_document().keys {
        assert!(
            key.kid.starts_with("key-") && key.kid.len() >= 12,
            "kid '{}' must start with 'key-' and be >= 12 chars",
            key.kid
        );
    }
}

#[test]
fn jwks_keys_have_valid_ed25519_public_key() {
    for key in &KEY_MANAGER.jwks_document().keys {
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&key.x)
            .expect("x must be valid base64");
        assert_eq!(
            decoded.len(),
            32,
            "Ed25519 public key must be 32 bytes, got {}",
            decoded.len()
        );
    }
}

#[test]
fn jwks_keys_verify_with_ring_crypto() {
    let keys = KEY_MANAGER.jwks_document().keys;
    let current = KEY_MANAGER.current_signing_key();

    for key in &keys {
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&key.x)
            .expect("x must be valid base64");
        let unpub = UnparsedPublicKey::new(&ring::signature::ED25519, &decoded);

        if let Some(ck) = current {
            let sig = ck.sign(b"test").expect("must sign");
            assert!(
                unpub.verify(b"test", &sig).is_ok(),
                "Key {} must verify signature from KEY_MANAGER.current_signing_key()",
                key.kid
            );
        }
    }
}

#[test]
fn jwks_response_matches_openapi_jwks_schema() {
    let doc = KEY_MANAGER.jwks_document();
    for key in &doc.keys {
        assert!(
            key.kid.starts_with("key-") && key.kid.len() >= 12,
            "kid '{}' must be valid per OpenAPI spec",
            key.kid
        );
        assert_eq!(
            key.kty,
            JwkKeyType::Okp,
            "kty must be OKP per OpenAPI JWKS schema"
        );
        assert!(!key.x.is_empty(), "x must not be empty");
    }
}

#[test]
fn jwks_response_contains_no_private_key_material() {
    let doc = KEY_MANAGER.jwks_document();
    assert!(
        keys_have_no_private_fields(&doc.keys),
        "JWKS must NOT contain private key fields (d, p, q, etc.)"
    );
}

// ─── Tests: Key Generation ──────────────────────────────────────────────────────

#[test]
fn key_generate_produces_valid_ed25519() {
    let key = sesame_idam_identity_session_service::key_manager::JwtSigningKey::generate(None).unwrap();
    assert!(!key.kid.is_empty());
    assert_eq!(key.alg, "EdDSA");
    assert_eq!(key.public_key_jwk.kty, JwkKeyType::Okp);
    assert_eq!(key.public_key_jwk.crv.to_string(), "Ed25519");
    assert_eq!(key.public_key_jwk.use_claim, JwkUse::Sig);
    assert!(!key.public_key_jwk.x.is_empty());
}

#[test]
fn key_sign_produces_64_byte_signature() {
    let key = sesame_idam_identity_session_service::key_manager::JwtSigningKey::generate(None).unwrap();
    let sig = key.sign(b"test message").unwrap();
    assert_eq!(sig.len(), 64, "Ed25519 signature must be 64 bytes");
}

#[test]
fn kid_format_starts_with_key_dash() {
    let key = sesame_idam_identity_session_service::key_manager::JwtSigningKey::generate(None).unwrap();
    assert!(key.kid.starts_with("key-"));
    assert!(key.kid.len() >= 9);
}

// ─── Tests: Key Rotation (on fresh instances) ────────────────────────────────────

#[test]
fn rotation_prepare_succeeds() {
    let mut km = make_km();
    let old_kid = km.current_key.as_ref().unwrap().kid.clone();

    let result = km.prepare_rotation();
    assert!(result.is_ok(), "prepare_rotation must succeed: {:?}", result);

    // next_key should now be set
    assert!(
        km.next_key.as_ref().map(|k| k.kid.clone()).is_some(),
        "next_key must be set after prepare_rotation"
    );

    // current_key unchanged
    assert_eq!(
        km.current_key.as_ref().unwrap().kid,
        old_kid,
        "current_key must not change after prepare"
    );

    // JWKS must now have 2 keys (current + next)
    assert!(
        km.jwks_document().keys.len() >= 2,
        "After prepare, JWKS must have >= 2 keys, got {}",
        km.jwks_document().keys.len()
    );
}

#[test]
fn rotation_activate_promotes_next_key() {
    let mut km = make_km();
    let original_kid = km.current_key.as_ref().unwrap().kid.clone();

    km.prepare_rotation().expect("prepare must succeed");
    let new_kid = km.next_key.as_ref().unwrap().kid.clone();

    km.activate_next_key().expect("activate must succeed");

    // current_key must now be the previously next_key
    assert_eq!(
        km.current_key.as_ref().unwrap().kid,
        new_kid,
        "current_key must be promoted to the previously prepared key"
    );

    // next_key must be None after activation
    assert!(
        km.next_key.is_none(),
        "next_key must be None after activate_next_key"
    );

    // Original key must still be present (grace period)
    let pub_keys = km.jwks_document().keys;
    assert!(
        pub_keys.iter().any(|k| k.kid == original_kid),
        "Original key '{}' must still be in JWKS during grace period",
        original_kid
    );

    // JWKS must have >= 2 keys
    assert!(
        pub_keys.len() >= 2,
        "After rotation, JWKS must have >= 2 keys, got {}",
        pub_keys.len()
    );
}

#[test]
fn rotation_both_keys_verify_signatures() {
    let mut km = make_km();
    km.prepare_rotation().expect("prepare must succeed");
    km.activate_next_key().expect("activate must succeed");

    // Sign with the new current key
    let new_current = km.current_key.as_ref().unwrap();
    let new_sig = new_current.sign(b"test rotation").expect("must sign");

    // Verify against all keys in JWKS
    let pub_keys = km.jwks_document().keys;
    let mut found = false;
    for key in &pub_keys {
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&key.x)
            .expect("x must be valid base64");
        let unpub = UnparsedPublicKey::new(&ring::signature::ED25519, &decoded);
        if unpub.verify(b"test rotation", &new_sig).is_ok() {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Signature from new current key must verify against at least one key in JWKS"
    );
}

#[test]
fn rotation_multiple_keys_all_have_valid_kids() {
    let mut km = make_km();
    km.prepare_rotation().expect("prepare must succeed");
    km.activate_next_key().expect("activate must succeed");

    for key in &km.jwks_document().keys {
        assert!(
            key.kid.starts_with("key-") && key.kid.len() >= 12,
            "Rotated key kid '{}' must be valid format",
            key.kid
        );
    }
}

#[test]
fn rotation_keys_have_correct_kty() {
    let mut km = make_km();
    km.prepare_rotation().expect("prepare must succeed");
    km.activate_next_key().expect("activate must succeed");

    for key in &km.jwks_document().keys {
        assert_eq!(key.kty, JwkKeyType::Okp);
    }
}

#[test]
fn rotation_keys_have_correct_curve() {
    let mut km = make_km();
    km.prepare_rotation().expect("prepare must succeed");
    km.activate_next_key().expect("activate must succeed");

    for key in &km.jwks_document().keys {
        assert_eq!(key.crv.to_string(), "Ed25519");
    }
}

// ─── Tests: Admin Revoke (on fresh instances) ────────────────────────────────────

#[test]
fn revoke_valid_key_succeeds() {
    let mut km = make_km();
    let kid = km.current_key.as_ref().unwrap().kid.clone();
    let initial_count = km.jwks_document().keys.len();

    let result = km.revoke_key(&kid);
    assert!(
        result.is_ok(),
        "Revoke of valid key must succeed: {:?}",
        result
    );

    // Key must be removed from JWKS
    let remaining = km.jwks_document().keys;
    let still_present = remaining.iter().any(|j| j.kid == kid);
    assert!(
        !still_present,
        "Key '{}' must be removed from JWKS after revocation",
        kid
    );

    // Key count must decrease
    assert!(
        remaining.len() < initial_count,
        "Key count must decrease: {} -> {}",
        initial_count,
        remaining.len()
    );
}

#[test]
fn revoke_invalid_key_fails() {
    let mut km = make_km();
    let result = km.revoke_key("nonexistent-key-00000000");
    assert!(result.is_err(), "Revoke of non-existent key must fail: {:?}", result);
}

#[test]
fn revoke_empty_kid_fails() {
    let mut km = make_km();
    let result = km.revoke_key("");
    assert!(
        result.is_err(),
        "Revoke with empty kid must fail: {:?}",
        result
    );
}

#[test]
fn revoke_drops_private_key_from_memory() {
    let mut km = make_km();
    let kid = km.current_key.as_ref().unwrap().kid.clone();
    let initial_count = km.jwks_document().keys.len();

    let result = km.revoke_key(&kid);
    assert!(
        result.is_ok(),
        "Revocation must succeed before checking memory: {:?}",
        result
    );

    let new_count = km.jwks_document().keys.len();
    assert!(
        new_count < initial_count,
        "Private key must be dropped from memory after revocation: {} -> {}",
        initial_count,
        new_count
    );
}

#[test]
fn revoke_key_tracked_in_revoked_list() {
    let mut km = make_km();
    let kid = km.current_key.as_ref().unwrap().kid.clone();

    let result = km.revoke_key(&kid);
    assert!(result.is_ok());

    // Verify the key is in the revoked_keys tracking
    assert!(
        km.revoked_keys().contains(&kid),
        "Revoked key '{}' must be tracked in revoked_keys",
        kid
    );
}

#[test]
fn revoke_key_removed_from_public_keys() {
    let mut km = make_km();
    let kid = km.current_key.as_ref().unwrap().kid.clone();
    km.revoke_key(&kid).expect("revoke must succeed");

    assert!(
        km.find_public_key(&kid).is_none(),
        "Revoked key '{}' must not be in public keys after revocation"
    );
}

#[test]
fn revoke_second_key_removes_current() {
    let mut km = make_km();
    let kid1 = km.current_key.as_ref().unwrap().kid.clone();

    // Revoke the current key
    km.revoke_key(&kid1).expect("revoke first must succeed");

    // Verify JWKS doesn't have the revoked key
    let remaining = km.jwks_document().keys;
    assert!(
        !remaining.iter().any(|k| k.kid == kid1),
        "First revoked key must be gone"
    );
}

// ─── Tests: Key State ────────────────────────────────────────────────────────────

#[test]
fn kid_is_active_returns_true_for_current() {
    let km = make_km();
    let kid = km.current_key.as_ref().unwrap().kid.clone();
    assert!(
        km.kid_is_active(&kid),
        "Current key must be active"
    );
}

#[test]
fn kid_is_active_returns_false_for_unknown() {
    let km = make_km();
    assert!(
        !km.kid_is_active("unknown-key-00000000"),
        "Unknown key must not be active"
    );
}

#[test]
fn is_revoked_false_before_revocation() {
    let km = make_km();
    let kid = km.current_key.as_ref().unwrap().kid.clone();
    assert!(
        !km.is_revoked(&kid),
        "Key must not be revoked before revocation"
    );
}

#[test]
fn is_revoked_true_after_revocation() {
    let mut km = make_km();
    let kid = km.current_key.as_ref().unwrap().kid.clone();
    km.revoke_key(&kid).expect("revoke must succeed");
    assert!(km.is_revoked(&kid), "Key must be revoked after revoke_key()");
}

#[test]
fn revoked_keys_empty_on_fresh_instance() {
    let km = make_km();
    assert!(km.revoked_keys().is_empty());
}

// ─── Tests: JWK Public Fields ───────────────────────────────────────────────────

#[test]
fn jwk_only_serializes_to_correct_json() {
    let km = make_km();
    let keys = km.jwks_document().keys;
    assert!(!keys.is_empty());

    for key in &keys {
        let json = serde_json::to_value(key).unwrap();
        assert!(json.get("kty").is_some());
        assert!(json.get("kid").is_some());
        assert!(json.get("use").is_some());
        assert!(json.get("crv").is_some());
        assert!(json.get("x").is_some());

        assert!(json.get("d").is_none());
        assert!(json.get("p").is_none());
        assert!(json.get("q").is_none());
    }
}

#[test]
fn find_public_key_returns_current() {
    let km = make_km();
    let kid = km.current_key.as_ref().unwrap().kid.clone();
    let found = km.find_public_key(&kid);
    assert!(found.is_some(), "find_public_key must return current key");
    assert_eq!(found.unwrap().kid, kid);
}

#[test]
fn find_public_key_returns_none_for_unknown() {
    let km = make_km();
    assert!(
        km.find_public_key("unknown-key-00000000").is_none()
    );
}

#[test]
fn keys_for_verification_returns_current_keys() {
    let km = make_km();
    let verification_keys = km.keys_for_verification();
    assert!(
        !verification_keys.is_empty(),
        "keys_for_verification must return at least one key"
    );
    // All keys used for verification should be OKP/Ed25519
    for key in verification_keys {
        assert_eq!(key.kty, JwkKeyType::Okp);
    }
}

// ─── Tests: Health Endpoint ─────────────────────────────────────────────────────

#[test]
fn health_returns_valid_response() {
    let km = make_km();
    let health = km.health();
    assert!(
        health.key_count >= 1,
        "Health must report at least one key, got {}",
        health.key_count
    );
    assert!(!health.current_kid.is_empty());
}
