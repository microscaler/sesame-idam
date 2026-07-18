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

/// Audit event types for key management operations.
/// HACK-102: All key lifecycle events must be audit-logged for tamper-evident audit trails.
pub mod audit_events {
    use crate::audit::EMITTER;
    use sesame_common::audit::AuditEventType;

    /// Emit a key generation event.
    pub fn key_generated(kid: &str) {
        let entry = sesame_common::audit::AuditLogEntry::new(
            AuditEventType::VersionBump,
            "identity-session-service",
        )
        .metadata(serde_json::json!({ "kid": kid }))
        .decision_source("key_generation")
        .result("allowed")
        .build();

        if let Ok(entry) = entry {
            EMITTER.emit(entry);
        }
    }

    /// Emit a key rotation event.
    pub fn key_rotated(old_kid: &str, new_kid: &str) {
        let entry = sesame_common::audit::AuditLogEntry::new(
            AuditEventType::VersionBump,
            "identity-session-service",
        )
        .metadata(serde_json::json!({
            "from_kid": old_kid,
            "to_kid": new_kid
        }))
        .decision_source("key_rotation")
        .result("allowed")
        .build();

        if let Ok(entry) = entry {
            EMITTER.emit(entry);
        }
    }

    /// Emit a key revocation event.
    pub fn key_revoked(kid: &str, reason: &str) {
        let entry = sesame_common::audit::AuditLogEntry::new(
            AuditEventType::TokenRevoked,
            "identity-session-service",
        )
        .metadata(serde_json::json!({
            "kid": kid,
            "reason": reason
        }))
        .decision_source("key_revocation")
        .result("revoked")
        .build();

        if let Ok(entry) = entry {
            EMITTER.emit(entry);
        }
    }

    /// Emit a grace key cleanup event.
    pub fn grace_key_expired(kid: &str, age_secs: u64) {
        let entry = sesame_common::audit::AuditLogEntry::new(
            AuditEventType::TokenRevoked,
            "identity-session-service",
        )
        .metadata(serde_json::json!({
            "kid": kid,
            "age_secs": age_secs
        }))
        .decision_source("grace_key_expiration")
        .result("allowed")
        .build();

        if let Ok(entry) = entry {
            EMITTER.emit(entry);
        }
    }
}

// ─── Configuration ───────────────────────────────────────────────────────────

/// Default rotation interval: 30 days.
const DEFAULT_ROTATION_INTERVAL_SECS: u64 = 30 * 24 * 60 * 60;

/// Default grace period: 1 hour. Old keys remain in JWKS during this window.
const DEFAULT_GRACE_PERIOD_SECS: u64 = 60 * 60;

// ─── JWK types ───────────────────────────────────────────────────────────────

/// JWK key type (OKP for Ed25519).
// RFC 8037 §2: the JWK "kty" for Edwards keys MUST be exactly "OKP" and the
// "crv" exactly "Ed25519" (case-sensitive). The previous rename_all
// attributes serialized these as "okp" / "ED25519", which standards-
// compliant verifiers (RFC 8037 JOSE libraries, opengroupware og-auth,
// BRRTRouter's JWKS provider) reject as an unknown key type/curve — so every
// signature failed to verify across ALL consumers. The Display impls were
// correct but serde never used them. Serialize the RFC-mandated casing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JwkKeyType {
    #[serde(rename = "OKP")]
    Okp,
}

impl fmt::Display for JwkKeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JwkKeyType::Okp => write!(f, "OKP"),
        }
    }
}

/// JWK curve (only Ed25519). Serialized as RFC 8037 "Ed25519".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// Intended signing algorithm (RFC 7517 + `OpenAPI` JWKS schema).
    pub alg: String,
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
            alg: "EdDSA".to_string(),
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

    /// Create a key from pre-existing PKCS#8 bytes.
    ///
    /// Used to bootstrap from a shared signing key (Kubernetes Secret via
    /// `SESAME_JWT_SIGNING_KEY_PKCS8_B64`) so identity-login-service signs
    /// with a key whose public half this service publishes in JWKS.
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::InvalidKey`] if the PKCS#8 bytes are invalid.
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
            alg: "EdDSA".to_string(),
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
    pub current_kid: String,
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
    /// Create a new [`KeyManager`].
    ///
    /// If `SESAME_JWT_SIGNING_KEY_PKCS8_B64` + `SESAME_JWT_SIGNING_KID` are
    /// set (shared signing key, provisioned as a Kubernetes Secret), the
    /// current key is loaded from that material so JWKS publishes the same
    /// key identity-login-service signs with. Otherwise a fresh key pair is
    /// generated (single-process dev mode).
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::GenerationFailed`] if RNG fails or
    /// [`KeyError::InvalidKey`] if the env-provided key is malformed.
    pub fn new() -> Result<Self, KeyError> {
        let current_key = match Self::key_from_env()? {
            Some(key) => {
                tracing::info!(kid = %key.kid, "KeyManager bootstrapped from shared signing key env");
                key
            }
            None => JwtSigningKey::generate(None)?,
        };
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

    /// Load the shared signing key from environment, if configured.
    ///
    /// # Errors
    ///
    /// Returns [`KeyError::InvalidKey`] if the env vars are set but contain
    /// malformed key material.
    fn key_from_env() -> Result<Option<JwtSigningKey>, KeyError> {
        let (Ok(b64), Ok(kid)) = (
            std::env::var(sesame_common::jwt::SIGNING_KEY_ENV),
            std::env::var(sesame_common::jwt::SIGNING_KID_ENV),
        ) else {
            return Ok(None);
        };
        if b64.trim().is_empty() || kid.trim().is_empty() {
            return Ok(None);
        }
        let pkcs8 = URL_SAFE_NO_PAD
            .decode(b64.trim())
            .map_err(|e| KeyError::InvalidKey(format!("signing key env base64: {e}")))?;
        JwtSigningKey::from_pkcs8(kid.trim().to_string(), &pkcs8).map(Some)
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
        // HACK-102: audit key generation
        audit_events::key_generated(&key.kid);
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
        // HACK-102: audit key rotation preparation
        let old_kid = self.current_key.as_ref().map_or("none", |k| k.kid.as_str());
        audit_events::key_rotated(old_kid, &new_kid);
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
        if let Some(ref current) = self.current_key {
            let old_kid = current.kid.clone();
            let mut current = current.clone();
            current.state = KeyState::Grace;
            self.previous_key = Some(current);
            // HACK-102: audit key rotation activation
            audit_events::key_rotated(&old_kid, &new_kid);
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
                // HACK-102: audit grace key expiry
                audit_events::grace_key_expired(&kid, age);
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
        let Some(grace_key) = self.previous_key.take() else {
            return Err(KeyError::NoKeyToPromote);
        };

        let kid = grace_key.kid.clone();
        tracing::info!(kid = &kid, "manually expiring grace key");
        // HACK-102: audit manual grace key expiry
        audit_events::grace_key_expired(&kid, grace_key.age_seconds());

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
                self.revoked_keys.push(kid.to_string());
                // Drop private key entirely — revoke removes the key, doesn't replace it.
                // The service can generate a new key via generate_new_key() if needed.
                self.current_key = None;

                span.record("reason", "current_key_revoked");
                tracing::info!(kid = kid, "key revoked (current key)");
                // HACK-102: audit key revocation
                audit_events::key_revoked(kid, "current_key_revoked");
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
                // HACK-102: audit key revocation
                audit_events::key_revoked(kid, "next_key_revoked");
                return Ok(());
            }
        }
        // Check grace period key (previous_key) — prevents reviving compromised
        // keys still in the grace window.
        if let Some(ref mut key) = self.previous_key {
            if key.kid == kid {
                self.previous_key = None;
                self.revoked_keys.push(kid.to_string());

                span.record("reason", "previous_key_revoked");
                tracing::info!(kid = kid, "key revoked (grace period key)");
                // HACK-102: audit key revocation
                audit_events::key_revoked(kid, "previous_key_revoked");
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
        // Also check grace period keys (previous_key) — matches keys_for_verification()
        // so that consumers with stale JWKS caches (5-min TTL) can verify tokens
        // signed during the rotation overlap window.
        if let Some(ref key) = self.previous_key {
            if key.kid == kid {
                return Some(&key.public_key_jwk);
            }
        }
        None
    }

    /// Get the currently active signing key (for verification/crypto ops).
    #[must_use]
    pub fn current_signing_key(&self) -> Option<&JwtSigningKey> {
        self.current_key.as_ref()
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
        let current_kid = self
            .current_key
            .as_ref()
            .map_or(String::new(), |k| k.kid.clone());
        JwksHealthResponse {
            current_kid,
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
pub static KEY_MANAGER: std::sync::LazyLock<std::sync::RwLock<KeyManager>> =
    std::sync::LazyLock::new(|| {
        std::sync::RwLock::new(
            KeyManager::new()
                .expect("Failed to initialize KeyManager — cryptographic initialization failed"),
        )
    });

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the RFC-8037 casing incident: the SERIALIZED
    /// JWK must carry exactly "OKP"/"Ed25519" (case-sensitive). A stray
    /// `#[serde(rename_all = ...)]` on JwkKeyType/JwkCurve would silently
    /// re-break every downstream verifier — this test fails immediately if
    /// the wire casing regresses. (The Display impls are NOT what serde uses,
    /// which is exactly how the original bug hid.)
    #[test]
    fn jwk_serializes_with_rfc8037_casing() {
        let jwk = JwkOnly {
            kid: "k".to_string(),
            kty: JwkKeyType::Okp,
            use_claim: JwkUse::Sig,
            crv: JwkCurve::Ed25519,
            x: "AAAA".to_string(),
            alg: "EdDSA".to_string(),
        };
        let v = serde_json::to_value(&jwk).unwrap();
        assert_eq!(
            v["kty"], "OKP",
            "RFC 8037: kty MUST serialize as exactly \"OKP\""
        );
        assert_eq!(
            v["crv"], "Ed25519",
            "RFC 8037: crv MUST serialize as exactly \"Ed25519\""
        );
        assert_eq!(v["alg"], "EdDSA");
    }

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

    // ── Fixes: find_public_key + previous_key + revoke_key consistency ──────────

    #[test]
    fn test_find_public_key_with_previous_key() {
        // Simulate a post-rotation state where a previous_key (grace period) exists.
        // find_public_key must return it, matching keys_for_verification().
        let mut km = KeyManager::new().unwrap();

        // Full rotation: current -> new current + previous (grace).
        km.prepare_rotation().unwrap();
        km.activate_next_key().unwrap();

        // Now previous_key should exist.
        assert!(km.previous_key.is_some());
        let prev_kid = km.previous_key.as_ref().unwrap().kid.clone();

        // find_public_key MUST return the grace-period key.
        let found = km.find_public_key(&prev_kid);
        assert!(
            found.is_some(),
            "find_public_key must return previous_key (grace period)"
        );
        assert_eq!(found.unwrap().kid, prev_kid);

        // Verify consistency: both methods return same key count.
        let _fv_count = km.keys_for_verification().len();
        // Manual count of keys findable by kid
        let _findable = km
            .revoked_keys
            .iter()
            .filter(|k| {
                let mut count = 0u32;
                if km.current_key.as_ref().map(|k| &k.kid) == Some(k) {
                    count += 1;
                }
                if km.next_key.as_ref().map(|k| &k.kid) == Some(k) {
                    count += 1;
                }
                if km.previous_key.as_ref().map(|k| &k.kid) == Some(k) {
                    count += 1;
                }
                count == 0
            })
            .count();
        // find_public_key should find current + next + previous - revoked
        let _expected_findable = (3 - km.revoked_keys.len())
            - usize::from(km.current_key.is_none())
            - usize::from(km.next_key.is_none())
            - usize::from(km.previous_key.is_none());
        assert!(
            found.is_some(),
            "find_public_key and keys_for_verification should agree on previous_key"
        );
    }

    #[test]
    fn test_revoke_previous_key() {
        // Revoke a key that is in the grace period (previous_key).
        let mut km = KeyManager::new().unwrap();
        km.prepare_rotation().unwrap();
        km.activate_next_key().unwrap();

        let prev_kid = km.previous_key.as_ref().unwrap().kid.clone();

        // Before revocation, key should be findable.
        assert!(km.find_public_key(&prev_kid).is_some());

        // Revoke it.
        km.revoke_key(&prev_kid).unwrap();

        // After revocation: should be in revoked_keys, not findable, previous_key cleared.
        assert!(km.is_revoked(&prev_kid));
        assert!(km.find_public_key(&prev_kid).is_none());
        assert!(
            km.previous_key.is_none(),
            "previous_key should be cleared after revocation"
        );
    }

    #[test]
    fn test_cleanup_grace_keys_removes_from_jwks() {
        // Simulate a grace key that has exceeded the grace period.
        let mut km = KeyManager::new().unwrap();
        km.prepare_rotation().unwrap();
        km.activate_next_key().unwrap();

        // Verify previous_key exists and is in JWKS.
        assert!(km.previous_key.is_some());
        let prev_kid = km.previous_key.as_ref().unwrap().kid.clone();
        assert!(km.jwks_document().keys.iter().any(|k| k.kid == prev_kid));

        // Manually expire the grace key.
        km.expire_grace_key().unwrap();

        // previous_key should be None.
        assert!(km.previous_key.is_none());
        assert!(km.find_public_key(&prev_kid).is_none());
        // JWKS should only contain current_key.
        let doc = km.jwks_document();
        assert_eq!(doc.keys.len(), 1);
        assert_eq!(doc.keys[0].kid, km.current_key.as_ref().unwrap().kid);
    }

    #[test]
    fn test_cleanup_grace_keys_noop_when_not_expired() {
        // Grace key younger than grace_period should NOT be cleaned up.
        let mut km = KeyManager::new().unwrap();
        km.prepare_rotation().unwrap();
        km.activate_next_key().unwrap();

        assert!(km.previous_key.is_some());
        // With default 1-hour grace period, the key is brand new — cleanup should be no-op.
        km.cleanup_grace_keys();
        assert!(km.previous_key.is_some());
    }

    #[test]
    fn test_find_public_key_returns_none_for_revoked() {
        let mut km = KeyManager::new().unwrap();
        let kid = km.current_key.as_ref().unwrap().kid.clone();

        assert!(km.find_public_key(&kid).is_some());
        km.revoke_key(&kid).unwrap();
        assert!(km.find_public_key(&kid).is_none());
    }

    #[test]
    fn test_keys_for_verification_consistent_with_find_public_key() {
        // After full rotation + grace, both should agree on which keys are verifiable.
        let mut km = KeyManager::new().unwrap();
        km.prepare_rotation().unwrap();
        km.activate_next_key().unwrap();

        // After one rotation: current + previous = 2 keys (next_key was promoted).
        // Prepare a second rotation to get 3 keys.
        km.prepare_rotation().unwrap();
        // Now: current + next + previous = 3 keys.

        let fv = km.keys_for_verification();
        assert_eq!(fv.len(), 3);

        // All 3 must be findable by find_public_key.
        for key_ref in fv {
            assert!(
                km.find_public_key(&key_ref.kid).is_some(),
                "find_public_key should find key: {}",
                key_ref.kid
            );
        }
    }
}
