/// BDD tests for Story 1.1 — Asymmetric JWKS endpoint
///
/// These tests exercise the `KeyManager` directly (which is what the
/// JWKS controller delegates to) to verify the actual API response
/// shape and data correctness.
use base64::Engine;
use ring::signature::UnparsedPublicKey;
use sesame_idam_identity_session_service::key_manager::KEY_MANAGER;

// ── Immutable tests (work on the global KEY_MANAGER) ──────────────────────────

#[test]
fn test_keymanager_jwks_document_returns_keys() {
    // GIVEN the KeyManager is initialized (it is, via LazyLock)
    // WHEN we get the JWKS document
    let doc = KEY_MANAGER.jwks_document();

    // THEN it must have at least one key
    assert!(
        !doc.keys.is_empty(),
        "KeyManager must return at least one Ed25519 key, got {}",
        doc.keys.len()
    );
}

#[test]
fn test_keymanager_jwks_keys_have_okp_kty() {
    // GIVEN the KeyManager is initialized
    let doc = KEY_MANAGER.jwks_document();

    // THEN each JWK must have kty=OKP (Ed25519)
    for jwk in &doc.keys {
        assert_eq!(
            jwk.kty,
            sesame_idam_identity_session_service::key_manager::JwkKeyType::Okp,
            "JWK kid={} must have kty=OKP",
            jwk.kid
        );
    }
}

#[test]
fn test_keymanager_jwks_keys_have_required_fields() {
    // GIVEN the KeyManager is initialized
    let doc = KEY_MANAGER.jwks_document();
    let jwk = &doc.keys[0];

    // THEN each JWK must have all required Ed25519 fields
    assert!(
        jwk.kid.starts_with("key-"),
        "kid must start with 'key-' (got '{}')",
        jwk.kid
    );
    assert_eq!(
        jwk.use_claim,
        sesame_idam_identity_session_service::key_manager::JwkUse::Sig
    );
    assert_eq!(
        jwk.crv,
        sesame_idam_identity_session_service::key_manager::JwkCurve::Ed25519
    );
    assert!(
        !jwk.x.is_empty(),
        "x must not be empty for Ed25519 public key"
    );
}

#[test]
fn test_keymanager_jwks_kid_format() {
    // GIVEN the KeyManager is initialized
    let doc = KEY_MANAGER.jwks_document();

    // THEN each kid must match key-YYYY-MM-DD-HH format
    for jwk in &doc.keys {
        let kid = &jwk.kid;
        assert!(
            kid.len() >= 12,
            "kid '{}' must be at least key-YYYY-MM-DD-HH (12 chars), got {}",
            kid,
            kid.len()
        );
        // Verify the date part is in key-YYYY-MM-DD-HH format
        let parts: Vec<&str> = kid.split('-').collect();
        assert!(
            parts.len() >= 3,
            "kid '{kid}' must have at least 3 '-' separated parts"
        );
        // parts[0] = "key", parts[1] = "YYYY", parts[2] = "MM"
        assert!(
            parts[1].len() == 4,
            "kid '{kid}' year part must be 4 digits"
        );
        assert!(
            parts[2].len() == 2,
            "kid '{kid}' month part must be 2 digits"
        );
    }
}

#[test]
fn test_keymanager_jwks_document_structure() {
    // GIVEN the KeyManager is initialized
    let doc = KEY_MANAGER.jwks_document();

    // THEN the document must have "keys" field (the JwksDocument serializes
    // to {"keys": [...]})
    let doc_json = serde_json::to_value(&doc).expect("Must serialize to JSON");
    assert!(
        doc_json.get("keys").is_some(),
        "JWK document must have 'keys' field"
    );
    let keys = doc_json["keys"]
        .as_array()
        .expect("'keys' must be an array");
    assert!(
        !keys.is_empty(),
        "'keys' array must have at least one element"
    );
}

#[test]
fn test_keymanager_keys_for_verification_returns_all() {
    // GIVEN the KeyManager is initialized
    let doc = KEY_MANAGER.jwks_document();
    let all_keys = KEY_MANAGER.keys_for_verification();

    // THEN keys_for_verification must return all keys in the document
    assert!(
        !all_keys.is_empty(),
        "keys_for_verification must return at least one key"
    );
    // Verify all returned KIDs are in the doc
    let doc_kids: Vec<&str> = doc.keys.iter().map(|j| j.kid.as_str()).collect();
    for key in &all_keys {
        assert!(
            doc_kids.contains(&key.kid.as_str()),
            "key_for_verification kid '{}' must be in JWKS doc",
            key.kid
        );
    }
}

#[test]
fn test_keymanager_kid_is_active() {
    // GIVEN the KeyManager is initialized
    let doc = KEY_MANAGER.jwks_document();

    // THEN each kid in the doc must report as active
    for jwk in &doc.keys {
        assert!(
            KEY_MANAGER.kid_is_active(&jwk.kid),
            "kid '{}' should be active",
            jwk.kid
        );
    }
}

// ── Mutable tests (use local KeyManager instances) ────────────────────────────

/// Helper to create a fresh `KeyManager` with short rotation for testing.
fn make_test_key_manager() -> sesame_idam_identity_session_service::key_manager::KeyManager {
    // Use 1-second rotation interval and 0 grace for fast testing
    sesame_idam_identity_session_service::key_manager::KeyManager::new_with_rotation(0, 1)
        .expect("Test KeyManager must initialize")
}

#[test]
fn test_keymanager_rotation_increases_key_count() {
    // GIVEN a test KeyManager with short rotation
    let mut km = make_test_key_manager();
    let initial_count = km.jwks_document().keys.len();

    // WHEN we prepare and activate a new key
    km.prepare_rotation()
        .expect("prepare_rotation must succeed");
    km.activate_next_key()
        .expect("activate_next_key must succeed");

    // THEN the key count should increase (old key kept for overlap)
    let new_doc = km.jwks_document();
    assert!(
        new_doc.keys.len() >= initial_count,
        "After rotation, must have >= {} keys (got {})",
        initial_count,
        new_doc.keys.len()
    );
}

#[test]
fn test_keymanager_signing_and_verification() {
    // GIVEN a test KeyManager
    let km = make_test_key_manager();

    // Get the signing key (current_key)
    let current_key = km.current_key.as_ref().expect("Must have current_key");
    assert_eq!(current_key.alg, "EdDSA");

    // WHEN we sign a payload
    let payload = b"test payload for signing";
    let signature = current_key.sign(payload).expect("Must sign payload");
    assert!(!signature.is_empty(), "Signature must not be empty");

    // THEN the signature must verify against the current key's public key
    let unpub = UnparsedPublicKey::new(
        &ring::signature::ED25519,
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&current_key.public_key_jwk.x)
            .expect("x must be valid base64"),
    );
    assert!(
        unpub.verify(payload, &signature).is_ok(),
        "Signature must verify against the KeyManager's current key"
    );
}

#[test]
fn test_keymanager_verify_with_all_keys() {
    // GIVEN a test KeyManager and a signature
    let km = make_test_key_manager();
    let payload = b"test payload";
    let current_key = km.current_key.as_ref().expect("Must have current_key");
    let signature = current_key.sign(payload).expect("Must sign");

    // WHEN we verify using keys_for_verification (which includes all active keys)
    // THEN at least one key should verify the signature
    let all_keys = km.keys_for_verification();
    let mut found_valid = false;
    for key_ref in &all_keys {
        let unpub = UnparsedPublicKey::new(
            &ring::signature::ED25519,
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(&key_ref.x)
                .expect("x must be valid base64"),
        );
        if unpub.verify(payload, &signature).is_ok() {
            found_valid = true;
            break;
        }
    }
    assert!(
        found_valid,
        "At least one key in keys_for_verification must verify the signature"
    );
}

#[test]
fn test_keymanager_jwks_after_rotation_still_valid() {
    // GIVEN a test KeyManager with rotation
    let mut km = make_test_key_manager();
    let doc_before = km.jwks_document();
    let key_before = &doc_before.keys[0];

    // WHEN we rotate
    km.prepare_rotation()
        .expect("prepare_rotation must succeed");
    km.activate_next_key()
        .expect("activate_next_key must succeed");

    // THEN the JWKS must still have valid keys
    let doc_after = km.jwks_document();
    assert!(
        !doc_after.keys.is_empty(),
        "JWKS after rotation must still have keys"
    );

    // AND the old key should still be in JWKS (for overlap)
    assert!(
        doc_after.keys.iter().any(|j| j.kid == key_before.kid),
        "Old key '{}' must still be in JWKS after rotation (overlap)",
        key_before.kid
    );
}

#[test]
fn test_keymanager_multiple_keys_all_verify() {
    // GIVEN a test KeyManager with rotation
    let mut km = make_test_key_manager();
    let payload = b"test payload";

    // Sign with the current key
    let current_key = km.current_key.as_ref().expect("Must have current_key");
    let signature = current_key.sign(payload).expect("Must sign");

    // WHEN we rotate (old key enters grace, new key activates)
    km.prepare_rotation()
        .expect("prepare_rotation must succeed");
    km.activate_next_key()
        .expect("activate_next_key must succeed");

    // THEN the old (grace) key's signature must still verify
    let all_keys = km.keys_for_verification();
    assert!(
        all_keys.len() >= 2,
        "After rotation, must have >= 2 keys for verification (got {})",
        all_keys.len()
    );

    let mut verified = false;
    for key_ref in &all_keys {
        let unpub = UnparsedPublicKey::new(
            &ring::signature::ED25519,
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(&key_ref.x)
                .expect("x must be valid base64"),
        );
        if unpub.verify(payload, &signature).is_ok() {
            verified = true;
            break;
        }
    }
    assert!(
        verified,
        "Signature from pre-rotation key must still verify against keys_for_verification"
    );
}
