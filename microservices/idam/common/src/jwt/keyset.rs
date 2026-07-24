//! Shared Ed25519 signing KEYSET (ADR-006, step 1: SOPS-delivered).
//!
//! A keyset is a JSON document carrying one or more PKCS#8 Ed25519 keys with
//! activation times, mounted from a Kubernetes Secret. Every replica of the
//! issuing services loads the SAME file, so N pods agree on every key and
//! every kid with zero coordination:
//!
//! ```json
//! { "keys": [
//!     { "pkcs8_b64": "<base64url no-pad PKCS#8 v2>", "valid_from": "2026-07-24T00:00:00Z" },
//!     { "pkcs8_b64": "...", "valid_from": "2026-06-24T00:00:00Z" }
//! ] }
//! ```
//!
//! - **kids are deterministic** — RFC 7638 JWK thumbprints of the public key
//!   (`{"crv":"Ed25519","kty":"OKP","x":...}` → SHA-256 → base64url), so the
//!   same key has the same name in every pod, forever.
//! - **Signing key = newest entry whose `valid_from` has passed**; entries
//!   with future `valid_from` are pre-published for rotation overlap; older
//!   entries stay verifiable through their grace window.
//! - **Rotation** = append a new entry (optionally future-dated), drop the
//!   oldest after grace. Step 1 delivers this via SOPS-git; step 2 moves
//!   generation/custody into the secret backend (OpenBao / GCP SM + HSM)
//!   without touching this format or any consumer code.
//!
//! Routing env (read by `Ed25519Signer::from_configured` and the session
//! service's `KeyManager`):
//! - `KEY_SOURCE` — `ephemeral` (default; today's behavior) or `file`
//! - `SESAME_SIGNING_KEYSET_FILE` — path to the mounted keyset JSON

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Env var selecting the key source (`ephemeral` | `file`).
pub const KEY_SOURCE_ENV: &str = "KEY_SOURCE";
/// Env var with the mounted keyset JSON path.
pub const KEYSET_FILE_ENV: &str = "SESAME_SIGNING_KEYSET_FILE";

/// Errors loading or interpreting a keyset.
#[derive(Debug, thiserror::Error)]
pub enum KeysetError {
    #[error("keyset io: {0}")]
    Io(String),
    #[error("keyset parse: {0}")]
    Parse(String),
    #[error("keyset key material: {0}")]
    InvalidKey(String),
    #[error("keyset has no key valid now")]
    NoValidKey,
}

/// On-disk keyset document.
#[derive(Debug, Serialize, Deserialize)]
pub struct SigningKeyset {
    pub keys: Vec<KeysetEntry>,
}

/// One key in the on-disk document.
#[derive(Debug, Serialize, Deserialize)]
pub struct KeysetEntry {
    /// base64url (no pad) PKCS#8 v2 Ed25519 document.
    pub pkcs8_b64: String,
    /// Activation time; future values pre-publish the key (rotation overlap).
    pub valid_from: DateTime<Utc>,
}

/// A parsed, validated key: deterministic kid + material + activation.
#[derive(Debug, Clone)]
pub struct LoadedKey {
    /// RFC 7638 thumbprint of the public key.
    pub kid: String,
    /// PKCS#8 v2 bytes.
    pub pkcs8: Vec<u8>,
    /// base64url raw public key (JWKS `x`).
    pub public_x: String,
    pub valid_from: SystemTime,
}

/// RFC 7638 JWK thumbprint for an `OKP`/`Ed25519` public key.
///
/// The hash input is the required members in lexicographic order with no
/// whitespace: `{"crv":"Ed25519","kty":"OKP","x":"<x>"}`.
#[must_use]
pub fn rfc7638_okp_thumbprint(x_b64url: &str) -> String {
    let canonical = format!(r#"{{"crv":"Ed25519","kty":"OKP","x":"{x_b64url}"}}"#);
    let digest = ring::digest::digest(&ring::digest::SHA256, canonical.as_bytes());
    URL_SAFE_NO_PAD.encode(digest.as_ref())
}

/// Parse a keyset JSON string into validated keys, sorted newest-first by
/// `valid_from`.
///
/// # Errors
///
/// Returns [`KeysetError`] on malformed JSON, undecodable/unparseable key
/// material, or an empty key list.
pub fn parse_keyset(json: &str) -> Result<Vec<LoadedKey>, KeysetError> {
    let doc: SigningKeyset =
        serde_json::from_str(json).map_err(|e| KeysetError::Parse(e.to_string()))?;
    if doc.keys.is_empty() {
        return Err(KeysetError::Parse("keyset has no keys".into()));
    }
    let mut keys = Vec::with_capacity(doc.keys.len());
    for entry in &doc.keys {
        let pkcs8 = URL_SAFE_NO_PAD
            .decode(entry.pkcs8_b64.trim())
            .map_err(|e| KeysetError::InvalidKey(format!("pkcs8_b64: {e}")))?;
        let pair = Ed25519KeyPair::from_pkcs8(&pkcs8)
            .map_err(|e| KeysetError::InvalidKey(format!("PKCS#8: {e}")))?;
        let public_x = URL_SAFE_NO_PAD.encode(pair.public_key().as_ref());
        keys.push(LoadedKey {
            kid: rfc7638_okp_thumbprint(&public_x),
            pkcs8,
            public_x,
            valid_from: entry.valid_from.into(),
        });
    }
    // Newest first — deterministic regardless of document order.
    keys.sort_by(|a, b| b.valid_from.cmp(&a.valid_from));
    Ok(keys)
}

/// Load + parse a keyset file.
///
/// # Errors
///
/// Returns [`KeysetError::Io`] when the file cannot be read, else as
/// [`parse_keyset`].
pub fn load_keyset_file(path: &str) -> Result<Vec<LoadedKey>, KeysetError> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| KeysetError::Io(format!("{path}: {e}")))?;
    parse_keyset(&json)
}

/// The signing key: newest entry whose `valid_from` has passed.
#[must_use]
pub fn signing_key(keys: &[LoadedKey]) -> Option<&LoadedKey> {
    let now = SystemTime::now();
    keys.iter().find(|k| k.valid_from <= now)
}

/// The configured keyset file path, when `KEY_SOURCE=file`.
#[must_use]
pub fn configured_keyset_file() -> Option<String> {
    let source = std::env::var(KEY_SOURCE_ENV).unwrap_or_default();
    if !source.trim().eq_ignore_ascii_case("file") {
        return None;
    }
    match std::env::var(KEYSET_FILE_ENV) {
        Ok(p) if !p.trim().is_empty() => Some(p.trim().to_string()),
        _ => {
            tracing::error!(
                "{KEY_SOURCE_ENV}=file but {KEYSET_FILE_ENV} is not set — falling back"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyset_json(entries: &[(String, &str)]) -> String {
        let keys: Vec<serde_json::Value> = entries
            .iter()
            .map(|(pkcs8, from)| {
                serde_json::json!({ "pkcs8_b64": pkcs8, "valid_from": from })
            })
            .collect();
        serde_json::json!({ "keys": keys }).to_string()
    }

    fn fresh_pkcs8_b64() -> String {
        let rng = ring::rand::SystemRandom::new();
        let doc = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        URL_SAFE_NO_PAD.encode(doc.as_ref())
    }

    #[test]
    fn kids_are_deterministic_rfc7638_thumbprints() {
        let pkcs8 = fresh_pkcs8_b64();
        let json = keyset_json(&[(pkcs8, "2026-01-01T00:00:00Z")]);
        let a = parse_keyset(&json).unwrap();
        let b = parse_keyset(&json).unwrap();
        assert_eq!(a[0].kid, b[0].kid, "same key must yield same kid everywhere");
        // Thumbprint of a known x is stable.
        assert_eq!(
            rfc7638_okp_thumbprint(&a[0].public_x),
            a[0].kid
        );
    }

    #[test]
    fn signing_key_is_newest_valid_and_future_keys_wait() {
        let old = fresh_pkcs8_b64();
        let new = fresh_pkcs8_b64();
        let future = fresh_pkcs8_b64();
        let json = keyset_json(&[
            (old.clone(), "2020-01-01T00:00:00Z"),
            (future, "2100-01-01T00:00:00Z"),
            (new.clone(), "2025-01-01T00:00:00Z"),
        ]);
        let keys = parse_keyset(&json).unwrap();
        // Sorted newest-first: future, new, old.
        assert_eq!(keys.len(), 3);
        let signer = signing_key(&keys).expect("a valid key");
        let new_bytes = URL_SAFE_NO_PAD.decode(&new).unwrap();
        assert_eq!(signer.pkcs8, new_bytes, "must sign with newest PAST key");
    }

    #[test]
    fn malformed_material_and_empty_sets_rejected() {
        assert!(parse_keyset("{\"keys\":[]}").is_err());
        let json = keyset_json(&[("bm90LWEta2V5".to_string(), "2026-01-01T00:00:00Z")]);
        assert!(parse_keyset(&json).is_err());
        assert!(parse_keyset("not json").is_err());
    }
}
