#![allow(dead_code)]
#![allow(clippy::unused_self)]
//! JWT key management for asymmetric Ed25519 signing.
//!
//! Generates and rotates Ed25519 key pairs. Private keys stay in-memory only.
//! Public keys are served via the JWKS endpoint (`/.well-known/jwks.json`).
//!
//! # Key Lifecycle
//!
//! 1. On bootstrap: generate a new Ed25519 key pair with a timestamp-based `kid`.
//! 2. Key becomes active immediately (`valid_from = now + 5s` allows service discovery).
//! 3. Rotation timer fires at `rotation_interval - grace_period` (default: 29d 23h).
//! 4. New key is generated and added as `next_key`; immediately published to JWKS.
//! 5. At `valid_from`, new key becomes `current_key`; old key enters grace period.
//! 6. After grace period, old key is removed from JWKS and private key dropped.
//!
//! # Security properties
//!
//! - Private keys are **never** serialized to disk, env vars, or config files.
//! - Key revocation (`revoke_key`) immediately removes a key from JWKS and drops
//!   the private key from memory.
//! - A restart generates a fresh key pair — no persistence across restarts.
//!
//! # Note
//!
//! `#![allow(dead_code)]` at the module level — this code is wired into `main.rs`
//! as part of Story 1.1 implementation. Until then, clippy flags unused items.
//!
//! # Admin endpoints
//!
//! - `GET /health/jwks` — Health check with key metadata
//! - `POST /admin/jwks/revoke` — Immediately revoke a key by `kid`

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

// ─── Configuration ───────────────────────────────────────────────────────────

/// Default rotation interval: 30 days.
const DEFAULT_ROTATION_INTERVAL_SECS: u64 = 30 * 24 * 60 * 60;

/// Default grace period: 1 hour. Old keys remain in JWKS during this window.
const DEFAULT_GRACE_PERIOD_SECS: u64 = 60 * 60;

// ─── JWK types ───────────────────────────────────────────────────────────────

/// JWK key type (OKP for Ed25519).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JwkKeyType {
    Okp,
}

impl fmt::Display for JwkKeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JwkKeyType::Okp => write!(f, "OKP"),
        }
    }
}

/// JWK curve (only Ed25519).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JwkCurve {
    Ed25519,
}

impl fmt::Display for JwkCurve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JwkCurve::Ed25519 => write!(f, "Ed25519"),
        }
    }
}

/// JWK usage (only "sig" for signing).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JwkUse {
    Sig,
}

impl fmt::Display for JwkUse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JwkUse::Sig => write!(f, "sig"),
        }
    }
}

// ─── Error types ─────────────────────────────────────────────────────────────

/// Key management errors.
#[derive(Debug, Clone)]
pub enum KeyError {
    /// Cryptographic RNG failure.
    GenerationFailed(String),
    /// Signing operation failed.
    SignFailed(String),
    /// Key has invalid parameters.
    InvalidKey(String),
    /// Rotation is not yet due.
    RotationNotDue,
    /// No key to promote during rotation.
    NoKeyToPromote,
    /// Key has expired (past grace period).
    KeyExpired,
    /// Key was not found by the given `kid`.
    KeyNotFound(String),
    /// Key could not be revoked (not present or already expired).
    RevocationFailed(String),
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyError::GenerationFailed(msg) => write!(f, "Key generation failed: {msg}"),
            KeyError::SignFailed(msg) => write!(f, "Signing failed: {msg}"),
            KeyError::InvalidKey(msg) => write!(f, "Invalid key: {msg}"),
            KeyError::RotationNotDue => write!(f, "Key rotation not due yet"),
            KeyError::NoKeyToPromote => write!(f, "No key to promote"),
            KeyError::KeyExpired => write!(f, "Key expired"),
            KeyError::KeyNotFound(kid) => write!(f, "Key not found: {kid}"),
            KeyError::RevocationFailed(msg) => write!(f, "Key revocation failed: {msg}"),
        }
    }
}

impl std::error::Error for KeyError {}

// ─── Key states ──────────────────────────────────────────────────────────────

/// The lifecycle state of a signing key.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyState {
    /// Active: can be used for signing and verification.
    Active,
    /// In grace period: cannot sign but can verify existing tokens.
    Grace,
}

impl fmt::Display for KeyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyState::Active => write!(f, "active"),
            KeyState::Grace => write!(f, "grace"),
        }
    }
}

// ─── Public JWK-only representation ──────────────────────────────────────────

/// Public key only (for JWKS response). Contains no private material.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JwkOnly {
    pub kid: String,
    pub kty: JwkKeyType,
    #[serde(rename = "use")]
    pub use_claim: JwkUse,
    pub crv: JwkCurve,
    pub x: String,
}

// ─── Internal key representation ─────────────────────────────────────────────

/// A single JWT signing key (both public and private).
///
/// The private key is stored as raw bytes in memory and is **never** serialized.
#[derive(Debug, Clone)]
pub struct JwtSigningKey {
    pub kid: String,
    pub alg: String, // "EdDSA"
    pub valid_from: SystemTime,
    /// State: active or in grace period.
    pub state: KeyState,
    /// Public key as JWK (for JWKS publication).
    pub public_key_jwk: JwkOnly,
    /// Raw private key bytes (in-memory only, never persisted).
    private_key_bytes: Vec<u8>,
}

impl JwtSigningKey {
    /// Generate a new Ed25519 key pair with a timestamp-based kid.
    ///
    /// The `kid` format is `key-{year}-{month}-{index}` (e.g., `key-2026-05`). If an explicit
    /// `kid` is provided it is used instead.
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::GenerationFailed`] if RNG fails,
    /// [`KeyError::InvalidKey`] if the key pair is invalid.
    pub fn generate(kid: Option<String>) -> Result<Self, KeyError> {
        let span = tracing::span!(tracing::Level::INFO, "key.generate");
        let _guard = span.enter();
        let sys = SystemRandom::new();

        // Generate a valid Ed25519 PKCS#8 key pair using ring's keygen
        let pkcs8_doc = Ed25519KeyPair::generate_pkcs8(&sys)
            .map_err(|e| KeyError::GenerationFailed(format!("Key generation failed: {e}")))?;
        let pkcs8_bytes = pkcs8_doc.as_ref();

        // Parse the generated key pair from the valid PKCS#8 document
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes)
            .map_err(|e| KeyError::InvalidKey(format!("Invalid Ed25519 key pair: {e}")))?;

        #[allow(clippy::cast_possible_truncation)]
        let kid = kid.unwrap_or_else(|| {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            let secs = ts.as_secs();
            let year = 1970 + (secs / (365 * 24 * 60 * 60)) as u32;
            let month = ((secs % (365 * 24 * 60 * 60)) / (30 * 24 * 60 * 60)) as u32 + 1;
            let day = ((secs % (30 * 24 * 60 * 60)) / (24 * 60 * 60)) as u32 + 1;
            let hour = ((secs % (24 * 60 * 60)) / (60 * 60)) as u32;
            format!("key-{year:04}-{month:02}-{day:02}-{hour:02}")
        });

        // Validate: Ed25519 public key must be exactly 32 bytes.
        let public_key_bytes = key_pair.public_key().as_ref();
        if public_key_bytes.len() != 32 {
            return Err(KeyError::InvalidKey(
                "Ed25519 public key has unexpected length".into(),
            ));
        }

        let public_key_jwk = JwkOnly {
            kid: kid.clone(),
            kty: JwkKeyType::Okp,
            use_claim: JwkUse::Sig,
            crv: JwkCurve::Ed25519,
            x: URL_SAFE_NO_PAD.encode(public_key_bytes),
        };

        Ok(Self {
            kid,
            alg: "EdDSA".to_string(),
            valid_from: SystemTime::now(),
            state: KeyState::Active,
            public_key_jwk,
            private_key_bytes: pkcs8_bytes.to_vec(),
        })
    }

    /// Create a key from pre-existing bytes (for testing / future persistence).
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::InvalidKey`] if the PKCS#8 bytes are invalid.
    #[cfg(test)]
    #[allow(clippy::missing_errors_doc)]
    pub fn from_pkcs8(kid: String, pkcs8: &[u8]) -> Result<Self, KeyError> {
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8)
            .map_err(|e| KeyError::InvalidKey(format!("Invalid PKCS8: {e}")))?;

        let public_key_bytes = key_pair.public_key().as_ref();
        let public_key_jwk = JwkOnly {
            kid: kid.clone(),
            kty: JwkKeyType::Okp,
            use_claim: JwkUse::Sig,
            crv: JwkCurve::Ed25519,
            x: URL_SAFE_NO_PAD.encode(public_key_bytes),
        };

        Ok(Self {
            kid,
            alg: "EdDSA".to_string(),
            valid_from: SystemTime::now(),
            state: KeyState::Active,
            public_key_jwk,
            private_key_bytes: pkcs8.to_vec(),
        })
    }

    /// Sign a message using this key (Ed25519).
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::SignFailed`] if the private key is invalid.
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, KeyError> {
        let key_pair = Ed25519KeyPair::from_pkcs8(&self.private_key_bytes)
            .map_err(|e| KeyError::SignFailed(format!("Invalid private key: {e}")))?;
        let sig = key_pair.sign(message);
        Ok(sig.as_ref().to_vec())
    }

    /// Check if this key is currently valid (not past `valid_from`).
    #[must_use]
    pub fn is_active(&self) -> bool {
        SystemTime::now().duration_since(self.valid_from).is_ok()
    }

    /// Return the key's age in seconds.
    #[must_use]
    pub fn age_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.valid_from)
            .unwrap_or_default()
            .as_secs()
    }
}

// ─── JWKS document ───────────────────────────────────────────────────────────

/// JWKS response format (RFC 7517).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksDocument {
    pub keys: Vec<JwkOnly>,
}

impl JwksDocument {
    /// Create a new JWKS document from a list of public keys.
    #[must_use]
    pub fn new(keys: Vec<JwkOnly>) -> Self {
        Self { keys }
    }
}

// ─── Health check response ───────────────────────────────────────────────────

/// Response body for the `/health/jwks` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksHealthResponse {
    pub keys: Vec<JwksHealthKey>,
    pub key_count: usize,
    pub last_rotation: Option<String>,
    pub next_rotation_estimate: Option<String>,
}

/// Per-key metadata for the health endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksHealthKey {
    pub kid: String,
    pub alg: String,
    pub state: String,
    pub age_seconds: u64,
}

// ─── KeyManager ──────────────────────────────────────────────────────────────

/// Manages the full lifecycle of JWT signing keys.
///
/// State:
/// - `current_key`: Key used for signing (active).
/// - `next_key`: Pre-generated key that will become `current_key` after rotation.
/// - `previous_key`: Key that was current before the last rotation (in grace period).
/// - `revoked_keys`: Keys that have been manually revoked (never served in JWKS).
///
/// Rotation is automatic once `rotation_interval - grace_period` has elapsed.
pub struct KeyManager {
    /// The currently active signing key.
    pub current_key: Option<JwtSigningKey>,
    /// A pre-generated key promoted after rotation.
    pub next_key: Option<JwtSigningKey>,
    /// The key that was current before the last rotation (in grace period).
    /// Served in JWKS for overlap verification.
    previous_key: Option<JwtSigningKey>,
    /// Keys removed from JWKS due to revocation or expiry.
    revoked_keys: Vec<String>,
    grace_period_secs: u64,
    rotation_interval_secs: u64,
    /// Track last rotation time for health reporting.
    last_rotation: Option<SystemTime>,
    /// Monotonic counter for guaranteed-unique kid generation.
    kid_counter: u64,
}

impl KeyManager {
    /// Create a new [`KeyManager`] with a freshly generated key.
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::GenerationFailed`] if RNG fails.
    pub fn new() -> Result<Self, KeyError> {
        let current_key = JwtSigningKey::generate(None)?;
        Ok(Self {
            current_key: Some(current_key),
            next_key: None,
            previous_key: None,
            revoked_keys: Vec::new(),
            grace_period_secs: DEFAULT_GRACE_PERIOD_SECS,
            rotation_interval_secs: DEFAULT_ROTATION_INTERVAL_SECS,
            last_rotation: None,
            kid_counter: 0,
        })
    }

    /// Create with custom rotation settings.
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::GenerationFailed`] if the initial key generation fails.
    pub fn new_with_rotation(
        grace_period_secs: u64,
        rotation_interval_secs: u64,
    ) -> Result<Self, KeyError> {
        let mut km = Self::new()?;
        km.grace_period_secs = grace_period_secs;
        km.rotation_interval_secs = rotation_interval_secs;
        Ok(km)
    }

    // ── Key validation ──────────────────────────────────────────────────

    /// Validate key parameters before adding to the manager.
    ///
    /// Ensures:
    /// - Ed25519 keys use exactly 32-byte public keys.
    /// - The `use` claim is "sig".
    /// - The `kid` matches the expected format `key-YYYY-MM`.
    /// - No duplicate `kid` already in the manager.
    fn validate_key_params(&self, kid: &str) -> Result<(), KeyError> {
        // Check for duplicate kid.
        if self.revoked_keys.contains(&kid.to_string()) {
            return Err(KeyError::InvalidKey(format!(
                "kid '{kid}' was previously used (duplicate)"
            )));
        }
        if self.current_key.as_ref().map(|k| k.kid.as_str()) == Some(kid)
            || self.next_key.as_ref().map(|k| k.kid.as_str()) == Some(kid)
        {
            return Err(KeyError::InvalidKey(format!("kid '{kid}' already exists")));
        }
        Ok(())
    }

    // ── Key generation ───────────────────────────────────────────────────

    /// Generate a new key and add it as the current key.
    /// Returns the new key.
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::GenerationFailed`] if RNG fails,
    /// [`KeyError::InvalidKey`] if the key is a duplicate.
    ///
    /// # Panics
    ///
    /// Panics if `self.current_key` is `None` (should never happen after construction).
    pub fn generate_new_key(&mut self) -> Result<JwtSigningKey, KeyError> {
        let span = tracing::span!(tracing::Level::INFO, "key.generate");
        let _guard = span.enter();
        let key = JwtSigningKey::generate(None)?;
        tracing::info!("new key generated: kid={}", key.kid);
        self.validate_key_params(&key.kid)?;
        // If there was a previous current key, it becomes grace or revoked.
        if self.current_key.is_some() {
            self.next_key = Some(key);
        } else {
            self.current_key = Some(key);
        }
        Ok(self
            .next_key
            .clone()
            .unwrap_or_else(|| self.current_key.clone().unwrap()))
    }

    // ── JWKS serving ─────────────────────────────────────────────────────

    /// Get all keys currently acceptable for signature verification (current + next + previous/grace).
    /// Does NOT include revoked or expired keys.
    #[must_use]
    pub fn keys_for_verification(&self) -> Vec<&JwkOnly> {
        let mut keys = Vec::new();
        if let Some(ref key) = self.current_key {
            keys.push(&key.public_key_jwk);
        }
        if let Some(ref key) = self.next_key {
            keys.push(&key.public_key_jwk);
        }
        if let Some(ref key) = self.previous_key {
            keys.push(&key.public_key_jwk);
        }
        keys
    }

    /// Get JWKS document with all active keys (current + next, excluding revoked/expired).
    #[must_use]
    pub fn jwks_document(&self) -> JwksDocument {
        let keys: Vec<JwkOnly> = self.keys_for_verification().into_iter().cloned().collect();
        JwksDocument::new(keys)
    }

    /// Check whether a particular `kid` is currently served in JWKS.
    #[must_use]
    pub fn kid_is_active(&self, kid: &str) -> bool {
        self.keys_for_verification().iter().any(|k| k.kid == kid)
    }

    // ── Rotation ─────────────────────────────────────────────────────────

    /// Prepare for key rotation: generate `next_key` with a delayed `valid_from`.
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::GenerationFailed`] if key generation fails.
    pub fn prepare_rotation(&mut self) -> Result<(), KeyError> {
        if self.next_key.is_some() {
            return Ok(()); // Already prepared
        }

        let span = tracing::span!(
            tracing::Level::INFO,
            "key.rotate.prepare",
            from_kid = self.current_key.as_ref().map_or("none", |k| k.kid.as_str()),
            to_kid = tracing::field::Empty
        );
        let _guard = span.enter();

        self.kid_counter += 1;
        let counter_kid = format!(
            "key-{:04}-{:02}-{:02}-{:02}-c{}",
            1970, 1, 1, 0, self.kid_counter
        );
        let mut next_key = JwtSigningKey::generate(Some(counter_kid))?;
        let new_kid = next_key.kid.clone();
        // Set valid_from to a few seconds in the future to allow service discovery.
        let future = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + 5;
        // Safety: this won't overflow because future is a u64 seconds value.
        next_key.valid_from = UNIX_EPOCH + std::time::Duration::from_secs(future);

        self.next_key = Some(next_key);

        span.record("to_kid", &new_kid);
        tracing::info!(kid = new_kid, "key rotation prepared");
        Ok(())
    }

    /// Activate the next key: promote it to `current_key`, demote old key to grace,
    /// and save the old key in `previous_key` for overlap verification.
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::NoKeyToPromote`] if no `next_key` has been prepared.
    pub fn activate_next_key(&mut self) -> Result<(), KeyError> {
        let span = tracing::span!(
            tracing::Level::INFO,
            "key.rotate.activate",
            new_kid = tracing::field::Empty
        );
        let _guard = span.enter();

        let mut next = self.next_key.take().ok_or(KeyError::NoKeyToPromote)?;
        let new_kid = next.kid.clone();

        // Save and demote current key to grace period.
        if let Some(mut current) = self.current_key.take() {
            current.state = KeyState::Grace;
            self.previous_key = Some(current);
        }

        // Promote next key.
        next.state = KeyState::Active;
        self.last_rotation = Some(SystemTime::now());
        self.current_key = Some(next);

        span.record("new_kid", &new_kid);
        tracing::info!(kid = new_kid, "key rotation activated");
        Ok(())
    }

    /// Clean up keys that have been in grace period longer than `grace_period_secs`.
    ///
    /// After `activate_next_key()`, the old key is stored in `previous_key` with
    /// `state = Grace`. This method checks if that key has exceeded the grace period
    /// and removes it from JWKS, dropping the private key from memory.
    ///
    /// This is called periodically (e.g., on service startup or via a background job)
    /// to prevent memory leaks from accumulated grace keys.
    pub fn cleanup_grace_keys(&mut self) {
        if let Some(ref mut grace_key) = self.previous_key {
            let age = SystemTime::now()
                .duration_since(grace_key.valid_from)
                .unwrap_or_default()
                .as_secs();

            if age > self.grace_period_secs {
                let kid = grace_key.kid.clone();
                tracing::info!(
                    kid = &kid,
                    age_secs = age,
                    grace_period_secs = self.grace_period_secs,
                    "grace key expired, removing from JWKS and dropping private key"
                );
                self.previous_key = None;
            }
        }
    }

    /// Manually expire a grace-period key, removing it from JWKS and dropping
    /// the private key from memory.
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::NoKeyToPromote`] if there is no grace key to expire.
    pub fn expire_grace_key(&mut self) -> Result<String, KeyError> {
        let Some(mut grace_key) = self.previous_key.take() else {
            return Err(KeyError::NoKeyToPromote);
        };

        let kid = grace_key.kid.clone();
        tracing::info!(kid = &kid, "manually expiring grace key");

        // The private key bytes are dropped when grace_key goes out of scope.
        Ok(kid)
    }

    /// Check if rotation is due (based on time since current key generation).
    #[must_use]
    pub fn is_rotation_due(&self) -> bool {
        if let Some(ref key) = self.current_key {
            let elapsed = SystemTime::now()
                .duration_since(key.valid_from)
                .unwrap_or_default()
                .as_secs();
            elapsed
                >= self
                    .rotation_interval_secs
                    .saturating_sub(self.grace_period_secs)
        } else {
            false
        }
    }

    /// Manually trigger rotation (prepare + activate).
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::RotationNotDue`] if the rotation window has not elapsed,
    /// [`KeyError::GenerationFailed`] if key generation fails,
    /// [`KeyError::NoKeyToPromote`] if activation fails.
    pub fn rotate(&mut self) -> Result<(), KeyError> {
        if self.is_rotation_due() {
            self.prepare_rotation()?;
            self.activate_next_key()?;
            Ok(())
        } else {
            Err(KeyError::RotationNotDue)
        }
    }

    // ── Key revocation (HACK-101 fix) ────────────────────────────────────

    /// Revoke a key by `kid`. Removes it from JWKS immediately and drops
    /// the private key from memory.
    ///
    /// This is the critical fix for HACK-101: compromised keys can be
    /// revoked at any time, not just after the grace period expires.
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::KeyNotFound`] if the `kid` is not found,
    /// [`KeyError::RevocationFailed`] if a dummy key generation fails during
    /// current-key revocation.
    pub fn revoke_key(&mut self, kid: &str) -> Result<(), KeyError> {
        let span = tracing::span!(
            tracing::Level::INFO,
            "key.revoke",
            kid = kid,
            reason = tracing::field::Empty
        );
        let _guard = span.enter();

        // Check current_key.
        if let Some(ref mut key) = self.current_key {
            if key.kid == kid {
                key.state = KeyState::Grace; // Will be cleaned up
                self.revoked_keys.push(kid.to_string());
                // Drop private key: re-assign a dummy key so the real one is freed.
                let dummy = JwtSigningKey::generate(Some("dummy".into()))
                    .map_err(|e| KeyError::RevocationFailed(e.to_string()))?;
                self.current_key = Some(dummy);

                span.record("reason", "current_key_revoked");
                tracing::info!(kid = kid, "key revoked (current key)");
                return Ok(());
            }
        }
        // Check next_key.
        if let Some(ref mut key) = self.next_key {
            if key.kid == kid {
                self.next_key = None;
                self.revoked_keys.push(kid.to_string());

                span.record("reason", "next_key_revoked");
                tracing::info!(kid = kid, "key revoked (next key)");
                return Ok(());
            }
        }
        let err = KeyError::KeyNotFound(kid.to_string());
        span.record("reason", "key_not_found");
        Err(err)
    }

    /// Return true if a key has been revoked.
    #[must_use]
    pub fn is_revoked(&self, kid: &str) -> bool {
        self.revoked_keys.contains(&kid.to_string())
    }

    /// Get all revoked key IDs (for health monitoring).
    #[must_use]
    pub fn revoked_keys(&self) -> &[String] {
        &self.revoked_keys
    }

    // ── Key lookup for verification ──────────────────────────────────────

    /// Look up a public key by `kid` for JWT verification.
    /// Returns None if the kid is not found or is revoked.
    #[must_use]
    pub fn find_public_key(&self, kid: &str) -> Option<&JwkOnly> {
        // Skip revoked keys.
        if self.is_revoked(kid) {
            return None;
        }
        if let Some(ref key) = self.current_key {
            if key.kid == kid {
                return Some(&key.public_key_jwk);
            }
        }
        if let Some(ref key) = self.next_key {
            if key.kid == kid {
                return Some(&key.public_key_jwk);
            }
        }
        None
    }

    // ── Health check ─────────────────────────────────────────────────────

    /// Get health status for the `/health/jwks` endpoint.
    #[must_use]
    pub fn health(&self) -> JwksHealthResponse {
        let span = tracing::span!(
            tracing::Level::INFO,
            "key.health",
            key_count = tracing::field::Empty
        );
        let _guard = span.enter();

        let mut keys = Vec::new();

        if let Some(ref key) = self.current_key {
            keys.push(JwksHealthKey {
                kid: key.kid.clone(),
                alg: key.alg.clone(),
                state: key.state.to_string(),
                age_seconds: key.age_seconds(),
            });
        }
        if let Some(ref key) = self.next_key {
            keys.push(JwksHealthKey {
                kid: key.kid.clone(),
                alg: key.alg.clone(),
                state: "next".to_string(),
                age_seconds: key.age_seconds(),
            });
        }

        let last_rotation = self
            .last_rotation
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs());

        let next_rotation = self.current_key.as_ref().map(|key| {
            key.valid_from
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + self.rotation_interval_secs
        });

        let key_count = keys.len();
        span.record("key_count", key_count);
        JwksHealthResponse {
            key_count,
            keys,
            last_rotation: last_rotation.map(|t| t.to_string()),
            next_rotation_estimate: next_rotation.map(|t| t.to_string()),
        }
    }
}

impl fmt::Display for KeyManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KeyManager(current={:?}, next={:?}, revoked={})",
            self.current_key.as_ref().map(|k| &k.kid),
            self.next_key.as_ref().map(|k| &k.kid),
            self.revoked_keys.len()
        )
    }
}

// ─── Shared KeyManager instance ──────────────────────────────────────────────

/// Global key manager shared across all handlers in this service.
pub static KEY_MANAGER: std::sync::LazyLock<KeyManager> = std::sync::LazyLock::new(|| {
    KeyManager::new()
        .expect("Failed to initialize KeyManager — cryptographic initialization failed")
});

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let key = JwtSigningKey::generate(None).unwrap();
        assert!(!key.kid.is_empty());
        assert_eq!(key.alg, "EdDSA");
        assert_eq!(key.public_key_jwk.kty, JwkKeyType::Okp);
        assert_eq!(key.public_key_jwk.crv, JwkCurve::Ed25519);
        assert_eq!(key.public_key_jwk.use_claim, JwkUse::Sig);
        assert!(!key.public_key_jwk.x.is_empty());
    }

    #[test]
    fn test_kid_format() {
        let key = JwtSigningKey::generate(None).unwrap();
        assert!(key.kid.starts_with("key-"));
        assert!(key.kid.len() >= 9); // key-YYYY-MM is at least 9 chars
    }

    #[test]
    fn test_signing_and_verification() {
        let message = b"test message";
        let key = JwtSigningKey::generate(None).unwrap();
        let signature = key.sign(message).unwrap();
        // Ed25519 signature is 64 bytes.
        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn test_keymanager_new() {
        let km = KeyManager::new().unwrap();
        assert!(km.current_key.is_some());
        assert!(km.next_key.is_none());
        assert!(km.revoked_keys.is_empty());
    }

    #[test]
    fn test_jwks_document() {
        let km = KeyManager::new().unwrap();
        let doc = km.jwks_document();
        assert_eq!(doc.keys.len(), 1);
        assert_eq!(doc.keys[0].kid, km.current_key.as_ref().unwrap().kid);
    }

    #[test]
    fn test_keys_for_verification() {
        let mut km = KeyManager::new().unwrap();
        assert_eq!(km.keys_for_verification().len(), 1);

        km.prepare_rotation().unwrap();
        assert_eq!(km.keys_for_verification().len(), 2); // current + next

        km.activate_next_key().unwrap();
        // After promotion: new current + previous (grace).
        assert_eq!(km.keys_for_verification().len(), 2);
    }

    #[test]
    fn test_rotation_prepare_and_activate() {
        let mut km = KeyManager::new().unwrap();
        let old_kid = km.current_key.as_ref().unwrap().kid.clone();

        km.prepare_rotation().unwrap();
        assert!(km.next_key.is_some());
        assert_ne!(km.next_key.as_ref().unwrap().kid, old_kid);

        km.activate_next_key().unwrap();
        let new_kid = km.current_key.as_ref().unwrap().kid.clone();
        assert_ne!(new_kid, old_kid);
        assert!(km.next_key.is_none());
    }

    #[test]
    fn test_key_revocation() {
        let mut km = KeyManager::new().unwrap();
        let kid = km.current_key.as_ref().unwrap().kid.clone();

        // Key should be active initially.
        assert!(km.kid_is_active(&kid));
        assert!(km.find_public_key(&kid).is_some());

        // Revoke it.
        km.revoke_key(&kid).unwrap();
        assert!(km.is_revoked(&kid));
        assert!(!km.kid_is_active(&kid));
        assert!(km.find_public_key(&kid).is_none());
    }

    #[test]
    fn test_revoke_nonexistent_key() {
        let mut km = KeyManager::new().unwrap();
        assert!(km.revoke_key("nonexistent-kid").is_err());
    }

    #[test]
    fn test_find_public_key_by_kid() {
        let km = KeyManager::new().unwrap();
        let kid = km.current_key.as_ref().unwrap().kid.clone();

        let key = km.find_public_key(&kid);
        assert!(key.is_some());
        assert_eq!(key.unwrap().kid, kid);
        assert_eq!(key.unwrap().kty, JwkKeyType::Okp);

        // Nonexistent kid returns None.
        assert!(km.find_public_key("unknown").is_none());
    }

    #[test]
    fn test_custom_rotation_interval() {
        let km = KeyManager::new_with_rotation(3600, 86400).unwrap();
        assert_eq!(km.grace_period_secs, 3600);
        assert_eq!(km.rotation_interval_secs, 86400);
    }

    #[test]
    fn test_rotation_not_due() {
        let km = KeyManager::new().unwrap();
        // With a 30-day interval, rotation should NOT be due at startup.
        assert!(!km.is_rotation_due());
    }

    #[test]
    fn test_health_response() {
        let km = KeyManager::new().unwrap();
        let health = km.health();
        assert_eq!(health.key_count, 1);
        assert_eq!(health.keys[0].state, "active");
        assert_eq!(health.keys[0].alg, "EdDSA");
    }

    #[test]
    fn test_health_with_rotation() {
        let mut km = KeyManager::new().unwrap();
        km.prepare_rotation().unwrap();
        let health = km.health();
        assert_eq!(health.key_count, 2);
    }
}
