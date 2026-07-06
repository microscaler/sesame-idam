//! Ed25519 JWT signer shared by token-issuing services.
//!
//! The signing key is provisioned via environment (a Kubernetes Secret in
//! deployment) so that identity-login-service signs with the same key pair
//! whose public half identity-session-service publishes at
//! `/.well-known/jwks.json`:
//!
//! - `SESAME_JWT_SIGNING_KEY_PKCS8_B64` — base64url (no pad) PKCS#8 Ed25519 key
//! - `SESAME_JWT_SIGNING_KID` — the key id to place in the JWT header (must
//!   match the JWKS entry)
//!
//! If the environment is not configured, `Ed25519Signer::from_env_or_generate`
//! falls back to a freshly generated key pair. That mode is only suitable for
//! single-process development — consumers validating against the session
//! service JWKS will reject tokens signed with an unpublished key.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};

use super::types::AccessClaims;

/// Env var carrying the base64url-encoded PKCS#8 Ed25519 signing key.
pub const SIGNING_KEY_ENV: &str = "SESAME_JWT_SIGNING_KEY_PKCS8_B64";
/// Env var carrying the key id published in JWKS.
pub const SIGNING_KID_ENV: &str = "SESAME_JWT_SIGNING_KID";

/// Errors from signer construction or signing.
#[derive(Debug, thiserror::Error)]
pub enum SignerError {
    #[error("invalid signing key material: {0}")]
    InvalidKey(String),
    #[error("key generation failed: {0}")]
    GenerationFailed(String),
    #[error("claims serialization failed: {0}")]
    Serialization(String),
}

/// Ed25519 access-token signer producing RFC 9068 `at+jwt` tokens.
pub struct Ed25519Signer {
    kid: String,
    pkcs8_bytes: Vec<u8>,
}

impl Ed25519Signer {
    /// Build a signer from explicit PKCS#8 bytes and a key id.
    ///
    /// # Errors
    ///
    /// Returns [`SignerError::InvalidKey`] if the bytes are not a valid
    /// Ed25519 PKCS#8 document.
    pub fn from_pkcs8(kid: impl Into<String>, pkcs8: &[u8]) -> Result<Self, SignerError> {
        Ed25519KeyPair::from_pkcs8(pkcs8)
            .map_err(|e| SignerError::InvalidKey(format!("invalid Ed25519 PKCS#8: {e}")))?;
        Ok(Self {
            kid: kid.into(),
            pkcs8_bytes: pkcs8.to_vec(),
        })
    }

    /// Build a signer from the environment (`SESAME_JWT_SIGNING_KEY_PKCS8_B64`
    /// + `SESAME_JWT_SIGNING_KID`). Returns `Ok(None)` when the env vars are
    /// not set.
    ///
    /// # Errors
    ///
    /// Returns [`SignerError::InvalidKey`] if the env vars are set but the key
    /// material does not decode/parse.
    pub fn from_env() -> Result<Option<Self>, SignerError> {
        let (Ok(b64), Ok(kid)) = (
            std::env::var(SIGNING_KEY_ENV),
            std::env::var(SIGNING_KID_ENV),
        ) else {
            return Ok(None);
        };
        if b64.trim().is_empty() || kid.trim().is_empty() {
            return Ok(None);
        }
        let pkcs8 = URL_SAFE_NO_PAD
            .decode(b64.trim())
            .map_err(|e| SignerError::InvalidKey(format!("{SIGNING_KEY_ENV} base64: {e}")))?;
        Self::from_pkcs8(kid.trim(), &pkcs8).map(Some)
    }

    /// Build from env, falling back to a freshly generated dev-only key pair.
    ///
    /// # Errors
    ///
    /// Returns [`SignerError::InvalidKey`] for malformed env key material or
    /// [`SignerError::GenerationFailed`] if RNG fails.
    pub fn from_env_or_generate() -> Result<Self, SignerError> {
        if let Some(signer) = Self::from_env()? {
            tracing::info!(kid = %signer.kid, "JWT signer loaded from environment");
            return Ok(signer);
        }
        tracing::warn!(
            "{SIGNING_KEY_ENV}/{SIGNING_KID_ENV} not set — generating ephemeral dev signing key. \
             Consumers validating against the session-service JWKS will reject these tokens."
        );
        Self::generate("dev-ephemeral")
    }

    /// Generate a fresh Ed25519 key pair (dev/test only).
    ///
    /// # Errors
    ///
    /// Returns [`SignerError::GenerationFailed`] if RNG fails.
    pub fn generate(kid: impl Into<String>) -> Result<Self, SignerError> {
        let rng = SystemRandom::new();
        let doc = Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|e| SignerError::GenerationFailed(e.to_string()))?;
        Self::from_pkcs8(kid, doc.as_ref())
    }

    /// The key id placed in signed JWT headers.
    #[must_use]
    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// Base64url (no pad) of the raw public key — the `x` value for a JWKS
    /// `OKP`/`Ed25519` entry.
    ///
    /// # Errors
    ///
    /// Returns [`SignerError::InvalidKey`] if the stored key fails to parse
    /// (cannot happen for keys accepted by the constructors).
    pub fn public_jwk_x(&self) -> Result<String, SignerError> {
        let pair = Ed25519KeyPair::from_pkcs8(&self.pkcs8_bytes)
            .map_err(|e| SignerError::InvalidKey(e.to_string()))?;
        Ok(URL_SAFE_NO_PAD.encode(pair.public_key().as_ref()))
    }

    /// The PKCS#8 bytes, base64url-encoded — for exporting to env/Secrets.
    #[must_use]
    pub fn pkcs8_b64(&self) -> String {
        URL_SAFE_NO_PAD.encode(&self.pkcs8_bytes)
    }

    /// Sign access claims into a compact `header.payload.signature` JWT with
    /// `{"alg":"EdDSA","typ":"at+jwt","kid":...}`.
    ///
    /// # Errors
    ///
    /// Returns [`SignerError::Serialization`] if the claims fail to serialize
    /// or [`SignerError::InvalidKey`] if the key fails to load.
    pub fn sign_access_claims(&self, claims: &AccessClaims) -> Result<String, SignerError> {
        let payload =
            serde_json::to_string(claims).map_err(|e| SignerError::Serialization(e.to_string()))?;
        self.sign_payload(&payload)
    }

    /// Sign an arbitrary JSON payload string (used for refresh tokens, which
    /// carry a reduced claim set).
    ///
    /// # Errors
    ///
    /// Returns [`SignerError::InvalidKey`] if the key fails to load.
    pub fn sign_payload(&self, payload_json: &str) -> Result<String, SignerError> {
        let header = serde_json::json!({
            "alg": "EdDSA",
            "typ": "at+jwt",
            "kid": self.kid,
        });
        let header_b64 = URL_SAFE_NO_PAD.encode(
            serde_json::to_string(&header)
                .map_err(|e| SignerError::Serialization(e.to_string()))?,
        );
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json);
        let signing_input = format!("{header_b64}.{payload_b64}");

        let pair = Ed25519KeyPair::from_pkcs8(&self.pkcs8_bytes)
            .map_err(|e| SignerError::InvalidKey(e.to_string()))?;
        let sig = pair.sign(signing_input.as_bytes());
        let sig_b64 = URL_SAFE_NO_PAD.encode(sig.as_ref());

        Ok(format!("{signing_input}.{sig_b64}"))
    }

    /// Verify a compact JWT produced by [`sign_access_claims`] /
    /// [`sign_payload`] against this signer's public key. Intended for tests.
    ///
    /// # Errors
    ///
    /// Returns [`SignerError::InvalidKey`] on malformed tokens or bad
    /// signatures.
    pub fn verify(&self, token: &str) -> Result<(), SignerError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(SignerError::InvalidKey("not a compact JWT".into()));
        }
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let sig = URL_SAFE_NO_PAD
            .decode(parts[2])
            .map_err(|e| SignerError::InvalidKey(format!("signature base64: {e}")))?;

        let pair = Ed25519KeyPair::from_pkcs8(&self.pkcs8_bytes)
            .map_err(|e| SignerError::InvalidKey(e.to_string()))?;
        let public_key = ring::signature::UnparsedPublicKey::new(
            &ring::signature::ED25519,
            pair.public_key().as_ref().to_vec(),
        );
        public_key
            .verify(signing_input.as_bytes(), &sig)
            .map_err(|_| SignerError::InvalidKey("signature verification failed".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::builders::{AccessClaimsBuilder, SesameAuthzClaimsBuilder};

    fn sample_claims() -> AccessClaims {
        AccessClaimsBuilder::new()
            .iss("https://idam.example.com")
            .sub("user-1")
            .aud(vec!["sesame-idam".into()])
            .client_id("hauliage-web")
            .scope("openid profile email".to_string())
            .exp(4_102_444_800) // 2100-01-01
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("sid-1")
            .tenant_id("hauliage")
            .user_id("user-1")
            .user_type("customer")
            .sx(SesameAuthzClaimsBuilder::new()
                .tenant("hauliage")
                .portal("hauliage-web")
                .roles(vec!["OWNER".into()])
                .build()
                .unwrap())
            .build()
            .unwrap()
    }

    #[test]
    fn sign_produces_three_part_token_with_eddsa_header() {
        let signer = Ed25519Signer::generate("test-kid").unwrap();
        let token = signer.sign_access_claims(&sample_claims()).unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);

        let header_json = URL_SAFE_NO_PAD.decode(parts[0]).unwrap();
        let header: serde_json::Value = serde_json::from_slice(&header_json).unwrap();
        assert_eq!(header["alg"], "EdDSA");
        assert_eq!(header["typ"], "at+jwt");
        assert_eq!(header["kid"], "test-kid");
    }

    #[test]
    fn signed_token_verifies_and_tampering_fails() {
        let signer = Ed25519Signer::generate("test-kid").unwrap();
        let token = signer.sign_access_claims(&sample_claims()).unwrap();
        assert!(signer.verify(&token).is_ok());

        // Tamper with the payload
        let mut parts: Vec<String> = token.split('.').map(String::from).collect();
        parts[1] = URL_SAFE_NO_PAD.encode(r#"{"sub":"attacker"}"#);
        let tampered = parts.join(".");
        assert!(signer.verify(&tampered).is_err());
    }

    #[test]
    fn payload_round_trips_claims() {
        let signer = Ed25519Signer::generate("test-kid").unwrap();
        let claims = sample_claims();
        let token = signer.sign_access_claims(&claims).unwrap();
        let payload = token.split('.').nth(1).unwrap();
        let decoded = URL_SAFE_NO_PAD.decode(payload).unwrap();
        let parsed: AccessClaims = serde_json::from_slice(&decoded).unwrap();
        assert_eq!(parsed.sub, "user-1");
        assert_eq!(parsed.tenant_id, "hauliage");
        assert_eq!(parsed.sx.roles, vec!["OWNER".to_string()]);
    }

    #[test]
    fn from_pkcs8_rejects_garbage() {
        assert!(Ed25519Signer::from_pkcs8("kid", b"not a key").is_err());
    }

    #[test]
    fn export_import_round_trip() {
        let signer = Ed25519Signer::generate("kid-1").unwrap();
        let b64 = signer.pkcs8_b64();
        let bytes = URL_SAFE_NO_PAD.decode(&b64).unwrap();
        let reloaded = Ed25519Signer::from_pkcs8("kid-1", &bytes).unwrap();
        assert_eq!(
            signer.public_jwk_x().unwrap(),
            reloaded.public_jwk_x().unwrap()
        );

        // Token signed by one verifies with the other (same key material)
        let token = signer.sign_access_claims(&sample_claims()).unwrap();
        assert!(reloaded.verify(&token).is_ok());
    }
}
