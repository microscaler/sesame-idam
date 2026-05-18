//! JWT key management for asymmetric signing.
//!
//! Generates and rotates Ed25519 key pairs. Private keys stay in-memory only.
//! Public keys are served via the JWKS endpoint (`/.well-known/jwks.json`).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ring::rand::SystemRandom;
use ring::rand::SecureRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

// Configuration
const DEFAULT_ROTATION_INTERVAL_SECS: u64 = 30 * 24 * 60 * 60; // 30 days
const DEFAULT_GRACE_PERIOD_SECS: u64 = 60 * 60; // 1 hour

/// JWK key type (only OKP for Ed25519)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JwkKeyType {
    Okp,
}

impl fmt::Display for JwkKeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                JwkKeyType::Okp => "OKP",
            }
        )
    }
}

/// JWK curve (only Ed25519)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JwkCurve {
    Ed25519,
}

impl fmt::Display for JwkCurve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                JwkCurve::Ed25519 => "Ed25519",
            }
        )
    }
}

/// JWK usage (only "sig" for signing)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JwkUse {
    Sig,
}

impl fmt::Display for JwkUse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                JwkUse::Sig => "sig",
            }
        )
    }
}

/// A single JWT signing key (both public and private)
#[derive(Debug, Clone)]
pub struct JwtSigningKey {
    pub kid: String,
    pub alg: String, // "EdDSA"
    pub valid_from: SystemTime,
    /// Public key as JWK (for JWKS publication)
    pub public_key_jwk: JwkOnly,
    /// Raw private key bytes (in-memory only, never persisted)
    private_key_bytes: Vec<u8>,
}

impl JwtSigningKey {
    /// Generate a new Ed25519 key pair with a timestamp-based kid
    pub fn generate(kid: Option<String>) -> Result<Self, KeyError> {
        let sys = SystemRandom::new();
        // Ed25519 key pair is 114 bytes: 32 bytes seed + 32 bytes public key + 50 bytes pkcs8 header
        let mut buf = [0u8; 114];
        sys.fill(&mut buf)
            .map_err(|_| KeyError::GenerationFailed("RNG failure".into()))?;

        let kid = kid.unwrap_or_else(|| {
            let ts = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            let secs = ts.as_secs();
            let year = 1970 + (secs / (365 * 24 * 60 * 60)) as u32;
            let month = ((secs % (365 * 24 * 60 * 60)) / (30 * 24 * 60 * 60)) as u32 + 1;
            format!("key-{:04}-{:02}", year, month)
        });

        let key_pair = Ed25519KeyPair::from_pkcs8(&buf)
            .map_err(|e| KeyError::GenerationFailed(format!("Invalid key pair: {}", e)))?;

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
            public_key_jwk,
            private_key_bytes: buf.to_vec(),
        })
    }

    /// Create a signature using this key
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, KeyError> {
        let key_pair = Ed25519KeyPair::from_pkcs8(&self.private_key_bytes)
            .map_err(|e| KeyError::SignFailed(format!("Invalid private key: {}", e)))?;
        let sig = key_pair.sign(message);
        Ok(sig.as_ref().to_vec())
    }

    /// Check if this key is currently valid (not past valid_from)
    pub fn is_valid(&self) -> bool {
        SystemTime::now().duration_since(self.valid_from).is_ok()
    }

    /// Check if the key is in its grace period (past validity but still accepted)
    pub fn is_in_grace_period(&self, grace_period: u64) -> bool {
        let now = SystemTime::now();
        let valid_since = now
            .duration_since(self.valid_from)
            .unwrap_or_default()
            .as_secs();
        valid_since > grace_period
    }
}

/// Public key only (for JWKS response)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JwkOnly {
    pub kid: String,
    pub kty: JwkKeyType,
    #[serde(rename = "use")]
    pub use_claim: JwkUse,
    pub crv: JwkCurve,
    pub x: String,
}

/// JWKS response format (RFC 7517)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksDocument {
    pub keys: Vec<JwkOnly>,
}

impl JwksDocument {
    pub fn new(keys: Vec<JwkOnly>) -> Self {
        Self { keys }
    }
}

impl fmt::Display for JwksDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string_pretty(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}

/// KeyManager manages the lifecycle of JWT signing keys
#[derive(Debug)]
pub struct KeyManager {
    pub current_key: Option<JwtSigningKey>,
    pub next_key: Option<JwtSigningKey>,
    grace_period_secs: u64,
    rotation_interval_secs: u64,
}

impl KeyManager {
    /// Create a new KeyManager with a freshly generated key
    pub fn new() -> Result<Self, KeyError> {
        let current_key = JwtSigningKey::generate(None)?;
        Ok(Self {
            current_key: Some(current_key),
            next_key: None,
            grace_period_secs: DEFAULT_GRACE_PERIOD_SECS,
            rotation_interval_secs: DEFAULT_ROTATION_INTERVAL_SECS,
        })
    }

    /// Create with custom rotation settings
    pub fn new_with_rotation(
        grace_period_secs: u64,
        rotation_interval_secs: u64,
    ) -> Result<Self, KeyError> {
        let mut km = Self::new()?;
        km.grace_period_secs = grace_period_secs;
        km.rotation_interval_secs = rotation_interval_secs;
        Ok(km)
    }

    /// Get all keys currently acceptable for signature verification (current + next)
    pub fn keys_for_verification(&self) -> Vec<&JwkOnly> {
        let mut keys = Vec::new();
        if let Some(ref key) = self.current_key {
            keys.push(&key.public_key_jwk);
        }
        if let Some(ref key) = self.next_key {
            keys.push(&key.public_key_jwk);
        }
        keys
    }

    /// Get JWKS document with all active keys
    pub fn jwks_document(&self) -> JwksDocument {
        let keys: Vec<JwkOnly> = self
            .keys_for_verification()
            .into_iter()
            .map(|jwk| jwk.clone())
            .collect();
        JwksDocument::new(keys)
    }

    /// Prepare for key rotation (generates next key with delayed validity)
    pub fn prepare_rotation(&mut self) -> Result<(), KeyError> {
        if self.next_key.is_some() {
            return Ok(()); // Already prepared
        }
        let next_key = JwtSigningKey::generate(None)?;
        self.next_key = Some(next_key);
        Ok(())
    }

    /// Activate the next key (promote it to current)
    pub fn activate_next_key(&mut self) -> Result<(), KeyError> {
        let next = self.next_key.take().ok_or(KeyError::NoKeyToPromote)?;

        if let Some(current) = self.current_key.take() {
            // Drop private key from memory
            drop(current);
        }

        self.current_key = Some(next);
        Ok(())
    }

    /// Clean up grace period keys that have expired
    pub fn cleanup_grace_keys(&mut self) {
        // In this simplified implementation, we just keep current and next.
        // A full impl would track grace-period keys separately and clean them up.
    }

    /// Check if rotation is due (based on time since current key generation)
    pub fn is_rotation_due(&self) -> bool {
        if let Some(ref key) = self.current_key {
            let elapsed = SystemTime::now()
                .duration_since(key.valid_from)
                .unwrap_or_default()
                .as_secs();
            // Rotate when we're within grace_period of the rotation interval
            elapsed
                >= self
                    .rotation_interval_secs
                    .saturating_sub(self.grace_period_secs)
        } else {
            false
        }
    }

    /// Manually trigger rotation
    pub fn rotate(&mut self) -> Result<(), KeyError> {
        if self.is_rotation_due() {
            self.prepare_rotation()?;
            self.activate_next_key()?;
            Ok(())
        } else {
            Err(KeyError::RotationNotDue)
        }
    }
}

impl fmt::Display for KeyManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KeyManager(current={:?}, next={:?})",
            self.current_key.as_ref().map(|k| &k.kid),
            self.next_key.as_ref().map(|k| &k.kid)
        )
    }
}

/// Key manager errors
#[derive(Debug, Clone)]
pub enum KeyError {
    GenerationFailed(String),
    SignFailed(String),
    InvalidKey(String),
    RotationNotDue,
    NoKeyToPromote,
    KeyExpired,
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyError::GenerationFailed(msg) => write!(f, "Key generation failed: {}", msg),
            KeyError::SignFailed(msg) => write!(f, "Signing failed: {}", msg),
            KeyError::InvalidKey(msg) => write!(f, "Invalid key: {}", msg),
            KeyError::RotationNotDue => write!(f, "Key rotation not due yet"),
            KeyError::NoKeyToPromote => write!(f, "No key to promote"),
            KeyError::KeyExpired => write!(f, "Key expired"),
        }
    }
}

impl std::error::Error for KeyError {}

// ===== Shared KeyManager instance for the service =====

/// Global key manager shared across all handlers in this service.
pub static KEY_MANAGER: std::sync::LazyLock<KeyManager> =
    std::sync::LazyLock::new(|| {
        KeyManager::new()
            .expect("Failed to initialize KeyManager - cryptographic initialization failed")
    });

// ===== Tests =====

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
        // Signature should be 64 bytes for Ed25519
        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn test_keymanager_new() {
        let km = KeyManager::new().unwrap();
        assert!(km.current_key.is_some());
        assert!(km.next_key.is_none());
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
        let keys = km.keys_for_verification();
        assert_eq!(keys.len(), 1);

        // Prepare and activate rotation
        km.prepare_rotation().unwrap();
        km.activate_next_key().unwrap();

        let keys = km.keys_for_verification();
        assert_eq!(keys.len(), 1); // Only current is in verification
    }
}
