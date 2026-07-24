//! ADR-006 step 1 acceptance: shared signing keyset file mode.
//!
//! - N KeyManagers loading the SAME keyset file agree on every key and every
//!   kid (the multi-replica JWKS-agreement property that made scaling unsafe
//!   before).
//! - The signer (`Ed25519Signer::from_configured` — what login/session token
//!   issuers use) picks the same newest key, so issued tokens verify against
//!   the JWKS every replica serves.
//! - Editing the keyset (rotation) is picked up by
//!   `reload_from_keyset_if_changed`, and the OLD key stays published for
//!   overlap verification.
//! - No file configured → ephemeral behaviour untouched.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ring::signature::Ed25519KeyPair;

use sesame_idam_identity_session_service::key_manager::KeyManager;

fn fresh_pkcs8_b64() -> String {
    let rng = ring::rand::SystemRandom::new();
    let doc = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    URL_SAFE_NO_PAD.encode(doc.as_ref())
}

fn write_keyset(path: &std::path::Path, entries: &[(String, &str)]) {
    let keys: Vec<serde_json::Value> = entries
        .iter()
        .map(|(pkcs8, from)| serde_json::json!({ "pkcs8_b64": pkcs8, "valid_from": from }))
        .collect();
    std::fs::write(
        path,
        serde_json::json!({ "keys": keys }).to_string(),
    )
    .unwrap();
}

fn temp_keyset_path(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("keyset-{tag}-{}.json", uuid::Uuid::new_v4()))
}

/// Scenario: two "replicas" (KeyManagers) loading the same keyset serve
/// byte-identical JWKS documents with deterministic kids.
#[test]
fn replicas_agree_on_jwks() {
    let path = temp_keyset_path("agree");
    let current = fresh_pkcs8_b64();
    let previous = fresh_pkcs8_b64();
    write_keyset(
        &path,
        &[
            (current, "2026-01-01T00:00:00Z"),
            (previous, "2025-12-01T00:00:00Z"),
        ],
    );

    let a = KeyManager::from_keyset_file(path.to_str().unwrap()).unwrap();
    let b = KeyManager::from_keyset_file(path.to_str().unwrap()).unwrap();

    let jwks_a = serde_json::to_string(&a.jwks_document()).unwrap();
    let jwks_b = serde_json::to_string(&b.jwks_document()).unwrap();
    assert_eq!(jwks_a, jwks_b, "replicas must serve identical JWKS");
    assert_eq!(a.jwks_document().keys.len(), 2, "current + grace published");

    // Deterministic kid: a third load agrees too.
    let c = KeyManager::from_keyset_file(path.to_str().unwrap()).unwrap();
    assert_eq!(
        a.current_key.as_ref().unwrap().kid,
        c.current_key.as_ref().unwrap().kid
    );
    let _ = std::fs::remove_file(&path);
}

/// Scenario: the token issuers' signer picks the same newest key the
/// KeyManager publishes as current — tokens sign under a kid every replica's
/// JWKS carries.
#[test]
fn signer_and_jwks_agree_on_current_key() {
    let path = temp_keyset_path("signer");
    write_keyset(
        &path,
        &[
            (fresh_pkcs8_b64(), "2026-01-01T00:00:00Z"),
            (fresh_pkcs8_b64(), "2025-01-01T00:00:00Z"),
        ],
    );

    // Signer via keyset selection (as from_configured does after env routing).
    let keys =
        sesame_common::jwt::load_keyset_file(path.to_str().unwrap()).expect("keyset loads");
    let signing = sesame_common::jwt::signing_key(&keys).expect("valid key");

    let km = KeyManager::from_keyset_file(path.to_str().unwrap()).unwrap();
    let current = km.current_key.as_ref().expect("current key");
    assert_eq!(
        signing.kid, current.kid,
        "signer key and JWKS current key must be the same key"
    );
    assert!(km.kid_is_active(&signing.kid), "signing kid must be in JWKS");
    let _ = std::fs::remove_file(&path);
}

/// Scenario: rotation by editing the file — reload installs the new key as
/// current and keeps the old one published for overlap verification.
#[test]
fn reload_picks_up_rotation_with_overlap() {
    let path = temp_keyset_path("rotate");
    let original = fresh_pkcs8_b64();
    write_keyset(&path, &[(original.clone(), "2026-01-01T00:00:00Z")]);

    let mut km = KeyManager::from_keyset_file(path.to_str().unwrap()).unwrap();
    let old_kid = km.current_key.as_ref().unwrap().kid.clone();

    // No change → no reload.
    assert!(!km.reload_from_keyset_if_changed().unwrap());

    // Rotation: append a newer key (as the rotation job would).
    write_keyset(
        &path,
        &[
            (fresh_pkcs8_b64(), "2026-07-01T00:00:00Z"),
            (original, "2026-01-01T00:00:00Z"),
        ],
    );
    assert!(km.reload_from_keyset_if_changed().unwrap());

    let new_kid = km.current_key.as_ref().unwrap().kid.clone();
    assert_ne!(old_kid, new_kid, "newest key becomes current");
    assert!(
        km.kid_is_active(&old_kid),
        "old key must stay published for overlap verification"
    );
    let _ = std::fs::remove_file(&path);
}

/// Scenario: a future-dated key is pre-published in JWKS but NOT used for
/// signing until its valid_from passes.
#[test]
fn future_key_prepublished_not_signing() {
    let path = temp_keyset_path("future");
    write_keyset(
        &path,
        &[
            (fresh_pkcs8_b64(), "2100-01-01T00:00:00Z"),
            (fresh_pkcs8_b64(), "2026-01-01T00:00:00Z"),
        ],
    );

    let km = KeyManager::from_keyset_file(path.to_str().unwrap()).unwrap();
    assert_eq!(km.jwks_document().keys.len(), 2, "future key pre-published");
    let current = km.current_key.as_ref().unwrap();
    let now = std::time::SystemTime::now();
    assert!(current.valid_from <= now, "current key must be valid NOW");
    assert!(!km.is_rotation_due(), "file mode never self-rotates");
    let _ = std::fs::remove_file(&path);
}

/// Scenario: a malformed replacement keyset is rejected and the previously
/// loaded keys stay active (reload failure is non-fatal).
#[test]
fn bad_reload_keeps_previous_keys() {
    let path = temp_keyset_path("bad");
    write_keyset(&path, &[(fresh_pkcs8_b64(), "2026-01-01T00:00:00Z")]);
    let mut km = KeyManager::from_keyset_file(path.to_str().unwrap()).unwrap();
    let kid = km.current_key.as_ref().unwrap().kid.clone();

    std::fs::write(&path, "{ not json").unwrap();
    assert!(km.reload_from_keyset_if_changed().is_err());
    assert_eq!(
        km.current_key.as_ref().unwrap().kid,
        kid,
        "previous keys must survive a bad reload"
    );
    let _ = std::fs::remove_file(&path);
}
