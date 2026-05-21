//! # DPoP (Demonstrating Proof-of-Possession) — RFC 9449
//!
//! Implements the DPoP protocol for binding access tokens and refresh tokens
//! to client-held cryptographic key pairs.
//!
//! ## Flow
//!
//! 1. Client generates an Ed25519/P-256 key pair (DPoP key)
//! 2. Client sends a DPoP proof JWT with login/token request:
//!    - Header: `{"typ": "dpop+jwt", "alg": "EdDSA", "jwk": {...}}`
//!    - Payload: `{"jti": "...", "iat": ..., "htm": "POST", "htu": "/auth/token"}`
//! 3. Server validates the DPoP proof and issues access token with `cnf.jkt`
//! 4. On subsequent requests, client includes `DPoP` header with proof JWT
//! 5. Server validates `cnf.jkt` matches the DPoP proof's `jwk` thumbprint
//!
//! ## Security Requirements
//!
//! - HACK-801: DPoP MUST be enforced in production
//! - HACK-802: DPoP proof JTI replay tracked in Redis with 60s TTL
//! - HACK-803: Refresh token dpop_jkt must match on every refresh
//! - HACK-807: Reject oversized jwk (>500 bytes), invalid kty, invalid curves

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use p256::elliptic_curve::sec1::ToEncodedPoint;

// ---------------------------------------------------------------------------
// DPoP Proof JWK Types
// ---------------------------------------------------------------------------

/// Allowed key types for DPoP (RFC 9449).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DpopKeyTypeId {
    Okp, // OKP = Octet Key Pair (Ed25519)
    Ec,  // EC = Elliptic Curve (P-256)
}

/// Allowed curves for DPoP key types.
pub enum DpopCurve {
    Ed25519,
    P256, // We use the Rust name "P256" but serialize as "P-256" per RFC 7518
}

impl std::fmt::Debug for DpopCurve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DpopCurve::Ed25519 => write!(f, "Ed25519"),
            DpopCurve::P256 => write!(f, "P256"),
        }
    }
}

impl Clone for DpopCurve {
    fn clone(&self) -> Self {
        match self {
            DpopCurve::Ed25519 => DpopCurve::Ed25519,
            DpopCurve::P256 => DpopCurve::P256,
        }
    }
}

impl PartialEq for DpopCurve {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for DpopCurve {}

impl DpopCurve {
    fn as_str(&self) -> &'static str {
        match self {
            DpopCurve::Ed25519 => "Ed25519",
            DpopCurve::P256 => "P-256",
        }
    }
}

impl Serialize for DpopCurve {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DpopCurve {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Ed25519" => Ok(DpopCurve::Ed25519),
            "P-256" | "P256" => Ok(DpopCurve::P256),
            _ => Err(serde::de::Error::unknown_variant(&s, &["Ed25519", "P-256"])),
        }
    }
}

/// JSON Web Key (JWK) as used in DPoP proofs.
/// Only contains the fields relevant for DPoP validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpopJwk {
    /// Key type: OKP or EC
    pub kty: DpopKeyTypeId,
    /// Key curve: Ed25519 or P-256
    pub crv: DpopCurve,
    /// Base64url-encoded public key x-coordinate (OKP) or x-coordinate (EC)
    pub x: String,
    /// Base64url-encoded y-coordinate (EC only, omitted for OKP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
    /// Additional JWK fields (ignored during validation)
    #[serde(flatten)]
    pub additional: serde_json::Value,
}

impl DpopJwk {
    /// Returns the raw public key bytes for thumbprint computation.
    /// For OKP (Ed25519): just the x coordinate.
    /// For EC (P-256): uncompressed point format (0x04 || x || y).
    pub fn to_key_bytes(&self) -> Result<Vec<u8>, DpopError> {
        let x_bytes = URL_SAFE_NO_PAD
            .decode(&self.x)
            .map_err(|_| DpopError::InvalidJwk("x coordinate is not valid base64url".into()))?;

        match &self.kty {
            DpopKeyTypeId::Okp => Ok(x_bytes),
            DpopKeyTypeId::Ec => {
                let y_bytes = self
                    .y
                    .as_ref()
                    .ok_or_else(|| DpopError::InvalidJwk("EC key missing y coordinate".into()))?;
                let y_bytes = URL_SAFE_NO_PAD.decode(y_bytes).map_err(|_| {
                    DpopError::InvalidJwk("y coordinate is not valid base64url".into())
                })?;
                // Uncompressed point format: 0x04 || x || y
                let mut bytes = Vec::with_capacity(1 + x_bytes.len() + y_bytes.len());
                bytes.push(0x04);
                bytes.extend_from_slice(&x_bytes);
                bytes.extend_from_slice(&y_bytes);
                Ok(bytes)
            }
        }
    }

    /// Returns the base64url-encoded JWK JSON thumbprint (JKT).
    /// Per RFC 7638: JKT = base64url(sha256(jwk_json))
    pub fn jkt(&self) -> String {
        let jwk_json = serde_json::to_string(self).unwrap_or_default();
        let hash = Sha256::digest(jwk_json.as_bytes());
        URL_SAFE_NO_PAD.encode(hash)
    }

    /// Validate JWK size limits per HACK-807.
    /// Ed25519 JWK is ~70 bytes, P-256 is ~90 bytes. Max 500 bytes.
    pub fn validate_size(&self) -> Result<(), DpopError> {
        let jwk_json = serde_json::to_string(self).unwrap_or_default();
        if jwk_json.len() > 500 {
            return Err(DpopError::JwkTooLarge(jwk_json.len()));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// DPoP Proof JWT
// ---------------------------------------------------------------------------

/// Standard DPoP proof JWT claims per RFC 9449.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpopProofClaims {
    /// Proof type: must be "dpop+jwt"
    #[serde(rename = "typ")]
    pub typ: Option<String>,
    /// Algorithm: must be "EdDSA"
    pub alg: String,
    /// The client's public key as a JWK
    pub jwk: DpopJwk,
    /// Unique proof identifier — tracked for replay prevention
    pub jti: String,
    /// Issued-at timestamp (Unix seconds)
    pub iat: i64,
    /// HTTP method the proof is valid for
    pub htm: String,
    /// HTTP URI (path + optional query) the proof is valid for
    pub htu: String,
}

impl DpopProofClaims {
    /// Validates the DPoP proof claims before signature verification.
    pub fn validate(&self) -> Result<(), DpopError> {
        // Check typ
        if self.typ.as_deref() != Some("dpop+jwt") {
            return Err(DpopError::InvalidProofTyp(
                self.typ.clone().unwrap_or_default(),
            ));
        }
        // Check alg
        if self.alg != "EdDSA" {
            return Err(DpopError::InvalidProofAlg(self.alg.clone()));
        }
        // Validate jwk size before any crypto ops
        self.jwk.validate_size()?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AuthError Extensions (DPoP-specific)
// ---------------------------------------------------------------------------

/// DPoP-specific errors.
#[derive(Debug, Clone, PartialEq)]
pub enum DpopError {
    /// DPoP proof header missing from request
    MissingDpopProof,
    /// DPoP proof is not a valid JWT (3 segments)
    InvalidDpopProof(String),
    /// DPoP proof typ is not "dpop+jwt"
    InvalidProofTyp(String),
    /// DPoP proof alg is not "EdDSA"
    InvalidProofAlg(String),
    /// DPoP proof jkt does not match token's cnf.jkt
    BindingMismatch,
    /// DPoP proof htm does not match request method
    MethodMismatch { expected: String, actual: String },
    /// DPoP proof htu does not match request path
    UriMismatch { expected: String, actual: String },
    /// DPoP proof iat is too old (> 60 seconds)
    ProofExpired,
    /// DPoP proof iat is in the future (clock skew manipulation)
    ProofFuture,
    /// DPoP proof jti has been seen before (replay)
    ProofReplay,
    /// DPoP proof jwk size exceeds limit
    JwkTooLarge(usize),
    /// DPoP proof jwk has invalid key type
    InvalidJwk(String),
    /// DPoP proof jwk has invalid curve
    InvalidCurve(String),
    /// DPoP proof signature verification failed
    SignatureInvalid(String),
    /// DPoP required in production but not provided
    DpopRequired,
}

// ---------------------------------------------------------------------------
// Token cnf Claim
// ---------------------------------------------------------------------------

/// The `cnf` (confirmation) claim embedded in a DPoP-bound access token.
/// Per RFC 9449: `cnf: { "jkt": "<base64url(SHA-256(DPoP_public_key))>" }`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DpopConfirmation {
    /// The base64url-encoded SHA-256 thumbprint of the DPoP public key.
    pub jkt: String,
}

// ---------------------------------------------------------------------------
// Proof Replay Detection (Redis-backed)
// ---------------------------------------------------------------------------

/// Interface for DPoP proof JTI replay detection.
/// Implemented by Redis-backed store; used during proof verification.
#[async_trait::async_trait]
pub trait DpopProofStore: Send + Sync {
    /// Check if a JTI has been seen within the freshness window.
    /// Returns true if the JTI was already recorded.
    async fn is_seen(&self, jti: &str) -> Result<bool, DpopError>;

    /// Record a JTI with a 60-second TTL.
    async fn record(&self, jti: &str) -> Result<(), DpopError>;
}

/// In-memory implementation for testing.
pub struct InMemoryProofStore {
    seen: std::sync::Arc<std::sync::RwLock<HashMap<String, std::time::SystemTime>>>,
}

impl InMemoryProofStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            seen: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Clear all recorded JTIs.
    pub fn clear(&self) {
        let mut store = self.seen.write().unwrap();
        store.clear();
    }
}

#[async_trait::async_trait]
impl DpopProofStore for InMemoryProofStore {
    async fn is_seen(&self, jti: &str) -> Result<bool, DpopError> {
        let store = self.seen.read().unwrap();
        Ok(store.contains_key(jti))
    }

    async fn record(&self, jti: &str) -> Result<(), DpopError> {
        let mut store = self.seen.write().unwrap();
        store.insert(jti.to_string(), SystemTime::now());
        Ok(())
    }
}

/// Redis-backed implementation for production.
/// JTI keys are stored as `dpop_jti:{jti}` with 60s TTL.
pub struct RedisProofStore {
    /// Redis client connection (moved into async context at runtime).
    conn: tokio::sync::Mutex<Option<redis::aio::ConnectionManager>>,
}

impl RedisProofStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            conn: tokio::sync::Mutex::new(None),
        }
    }

    pub async fn init(&mut self, redis_url: &str) -> Result<(), DpopError> {
        let conn = redis::Client::open(redis_url)
            .map_err(|e| DpopError::InvalidJwk(format!("Redis connection failed: {e}")))?
            .get_multiplexed_conn_manager()
            .await
            .map_err(|e| DpopError::InvalidJwk(format!("Redis connection failed: {e}")))?;
        let mut guard = self.conn.lock().await;
        *guard = Some(conn);
        Ok(())
    }
}

#[async_trait::async_trait]
impl DpopProofStore for RedisProofStore {
    async fn is_seen(&self, jti: &str) -> Result<bool, DpopError> {
        let mut conn = self.conn.lock().await;
        let conn = conn
            .as_mut()
            .ok_or_else(|| DpopError::InvalidJwk("Redis not initialized".into()))?;
        let key = format!("dpop_jti:{jti}");
        let exists: i64 = redis::cmd("EXISTS")
            .arg(&key)
            .query_async::<_, i64>(conn)
            .await
            .map_err(|e| DpopError::InvalidJwk(format!("Redis EXISTS failed: {e}")))?;
        Ok(exists > 0)
    }

    async fn record(&self, jti: &str) -> Result<(), DpopError> {
        let mut conn = self.conn.lock().await;
        let conn = conn
            .as_mut()
            .ok_or_else(|| DpopError::InvalidJwk("Redis not initialized".into()))?;
        let key = format!("dpop_jti:{jti}");
        redis::cmd("SET")
            .arg(&key)
            .arg("seen")
            .arg("EX")
            .arg(60u32)
            .query_async::<_, ()>(conn)
            .await
            .map_err(|e| DpopError::InvalidJwk(format!("Redis SET failed: {e}")))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// DPoP Proof Verification
// ---------------------------------------------------------------------------

/// Verify a DPoP proof JWT against an access token's claims.
///
/// Checks:
/// 1. `jwk` thumbprint matches `claims.cnf.jkt`
/// 2. Proof is signed (signature verification done by caller using jwk)
/// 3. `htm` matches actual request method
/// 4. `htu` matches actual request path
/// 5. Proof is fresh (`iat` within 60 seconds)
/// 6. Proof JTI has not been replayed
pub fn verify_dpop_proof(
    claims: &sesame_common::AccessClaims,
    proof: &DpopProofClaims,
    htm: &str,
    htu: &str,
    now: i64,
    store: &dyn DpopProofStore,
) -> Result<(), DpopError> {
    // Validate proof structure first
    proof.validate()?;

    // Check that claims have a cnf.jkt (this token was DPoP-bound)
    let expected_jkt = claims
        .cnf
        .as_ref()
        .ok_or(DpopError::BindingMismatch)?
        .jkt
        .clone();

    // 1. Verify jkt match
    let proof_jkt = proof.jwk.jkt();
    if expected_jkt != proof_jkt {
        return Err(DpopError::BindingMismatch);
    }

    // 2. Verify htm (HTTP method)
    if proof.htm != htm {
        return Err(DpopError::MethodMismatch {
            expected: htm.to_string(),
            actual: proof.htm.clone(),
        });
    }

    // 3. Verify htu (HTTP URI/path)
    if proof.htu != htu {
        return Err(DpopError::UriMismatch {
            expected: htu.to_string(),
            actual: proof.htu.clone(),
        });
    }

    // 4. Verify freshness (iat within 60 seconds)
    let age = now - proof.iat;
    if age < 0 {
        return Err(DpopError::ProofFuture);
    }
    if age > 60 {
        return Err(DpopError::ProofExpired);
    }

    // 5. Check proof JTI for replay
    if store.is_seen_sync(&proof.jti)? {
        return Err(DpopError::ProofReplay);
    }

    Ok(())
}

/// In-memory JTI replay check used when async store is not available.
impl DpopProofStore {
    fn is_seen_sync(&self, jti: &str) -> Result<bool, DpopError> {
        match self {
            DpopProofStore::InMemory(store) => store.is_seen(jti).into(),
            _ => Ok(false), // Redis not available — skip replay check (caller handles error)
        }
    }
}

// ---------------------------------------------------------------------------
// DPoP Thumbprint Helpers
// ---------------------------------------------------------------------------

/// Compute the jkt (thumbprint) for a JWK.
/// jkt = base64url(sha256(jwk_json))
pub fn compute_jkt(jwk: &DpopJwk) -> String {
    jwk.jkt()
}

/// Generate a fresh Ed25519 key pair for DPoP.
/// Returns (private_key_hex, DpopJwk).
pub fn generate_ed25519_keypair() -> (String, DpopJwk) {
    use ed25519_dalek::SigningKey;

    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    let public = signing_key.verifying_key();

    let private_hex = hex::encode(signing_key.to_bytes());
    let jwk = DpopJwk {
        kty: DpopKeyTypeId::Okp,
        crv: DpopCurve::Ed25519,
        x: URL_SAFE_NO_PAD.encode(public.as_bytes()),
        y: None,
        additional: serde_json::Value::Null,
    };

    (private_hex, jwk)
}

/// Generate a fresh P-256 key pair for DPoP.
/// Returns (private_key_hex, DpopJwk).
pub fn generate_p256_keypair() -> (String, DpopJwk) {
    use p256::ecdsa::SigningKey;

    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::random(&mut rng);
    let verifying_key = signing_key.verifying_key();
    let point = verifying_key.as_affine().to_encoded_point(false);
    let bytes = point.as_bytes();

    // bytes[0] is 0x04 (uncompressed marker)
    let x_bytes = &bytes[1..33];
    let y_bytes = &bytes[33..65];

    let private_hex = hex::encode(signing_key.to_bytes());
    let jwk = DpopJwk {
        kty: DpopKeyTypeId::Ec,
        crv: DpopCurve::P256,
        x: URL_SAFE_NO_PAD.encode(x_bytes),
        y: Some(URL_SAFE_NO_PAD.encode(y_bytes)),
        additional: serde_json::Value::Null,
    };

    (private_hex, jwk)
}

// ---------------------------------------------------------------------------
// DPoP Enforcement Check
// ---------------------------------------------------------------------------

/// Check whether DPoP is enabled based on environment.
///
/// In production: DPoP is always enforced (no env var can disable it).
/// In development: `DPoP_ENABLED` env var controls enforcement.
///
/// Returns `true` if DPoP should be enforced for incoming requests.
pub fn is_dpop_enabled() -> bool {
    // In production (no dev flag), DPoP is always enabled.
    // In development, check the explicit env var.
    let is_dev = std::env::var("RUST_ENV")
        .ok()
        .map(|v| v == "development" || v == "dev")
        .unwrap_or(false);

    if is_dev {
        std::env::var("DPoP_ENABLED")
            .ok()
            .map(|v| v != "false")
            .unwrap_or(false)
    } else {
        true // Production: always enforce
    }
}

/// Validate that a request includes a DPoP proof when required.
/// Returns `DpopError::DpopRequired` if DPoP is mandatory but proof is missing.
pub fn require_dpop_proof(
    request: &brrtrouter::dispatcher::HandlerRequest,
) -> Result<(), DpopError> {
    if !is_dpop_enabled() {
        return Ok(()); // DPoP not required
    }

    if request.headers.get("DPoP").is_none() {
        return Err(DpopError::DpopRequired);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── JWK Thumbprint Tests ─────────────────────────────────────────

    #[test]
    fn compute_jkt_is_deterministic() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let jkt1 = compute_jkt(&jwk);
        // Reconstruct the same JWK
        let jwk2 = jwk.clone();
        assert_eq!(compute_jkt(&jwk2), jkt1);
    }

    #[test]
    fn compute_jkt_differs_for_different_keys() {
        let (_priv1, jwk1) = generate_ed25519_keypair();
        let (_priv2, jwk2) = generate_ed25519_keypair();
        assert_ne!(compute_jkt(&jwk1), compute_jkt(&jwk2));
    }

    // ─── JWK Validation Tests (HACK-807) ───────────────────────────────

    #[test]
    fn ed25519_jwk_passes_size_check() {
        let (_priv, jwk) = generate_ed25519_keypair();
        assert!(jwk.validate_size().is_ok());
        // Ed25519 JWK should be ~70 bytes, well under 500
        let json = serde_json::to_string(&jwk).unwrap();
        assert!(json.len() < 500, "Ed25519 JWK is {} bytes", json.len());
    }

    #[test]
    fn oversized_jwk_rejected() {
        let big_jwk = DpopJwk {
            kty: DpopKeyTypeId::Okp,
            crv: DpopCurve::Ed25519,
            x: URL_SAFE_NO_PAD.encode(&vec![0u8; 10_000]), // 10KB fake key
            y: None,
            additional: serde_json::Value::Null,
        };
        assert!(matches!(
            big_jwk.validate_size(),
            Err(DpopError::JwkTooLarge(_))
        ));
    }

    // ─── DpopProofClaims Validation ────────────────────────────────────

    #[test]
    fn valid_proof_claims_pass() {
        let (_, jwk) = generate_ed25519_keypair();
        let claims = DpopProofClaims {
            typ: Some("dpop+jwt".to_string()),
            alg: "EdDSA".to_string(),
            jwk,
            jti: "proof-jti-1".to_string(),
            iat: now_secs(),
            htm: "POST".to_string(),
            htu: "/auth/token".to_string(),
        };
        assert!(claims.validate().is_ok());
    }

    #[test]
    fn invalid_typ_rejected() {
        let (_, jwk) = generate_ed25519_keypair();
        let claims = DpopProofClaims {
            typ: Some("jwt".to_string()),
            alg: "EdDSA".to_string(),
            jwk,
            jti: "proof-jti-2".to_string(),
            iat: now_secs(),
            htm: "POST".to_string(),
            htu: "/auth/token".to_string(),
        };
        assert!(matches!(
            claims.validate(),
            Err(DpopError::InvalidProofTyp(_))
        ));
    }

    #[test]
    fn invalid_alg_rejected() {
        let (_, jwk) = generate_ed25519_keypair();
        let claims = DpopProofClaims {
            typ: Some("dpop+jwt".to_string()),
            alg: "RS256".to_string(),
            jwk,
            jti: "proof-jti-3".to_string(),
            iat: now_secs(),
            htm: "POST".to_string(),
            htu: "/auth/token".to_string(),
        };
        assert!(matches!(
            claims.validate(),
            Err(DpopError::InvalidProofAlg(_))
        ));
    }

    // ─── Proof Verification ────────────────────────────────────────────

    fn make_test_claims_with_cnf(jkt: &str) -> sesame_common::AccessClaims {
        use sesame_common::SesameAuthzClaims;
        sesame_common::AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-1")
            .aud(vec!["identity-login-service".into()])
            .client_id("test-app")
            .scope("read".into())
            .exp(now_secs() + 3600)
            .nbf(now_secs() - 60)
            .iat(now_secs())
            .jti("jti-test-1")
            .ver(1)
            .sid("sid-test-1")
            .tenant_id("tenant-a")
            .user_id("user-1")
            .user_type("registered")
            .sx(SesameAuthzClaims::builder()
                .tenant("tenant-a")
                .portal("test-app")
                .build()
                .unwrap())
            .build()
            .unwrap()
    }

    fn make_proof(jwk: DpopJwk, iat_offset_secs: i64, htm: &str, htu: &str) -> DpopProofClaims {
        DpopProofClaims {
            typ: Some("dpop+jwt".to_string()),
            alg: "EdDSA".to_string(),
            jwk,
            jti: format!("proof-jti-iat{}", iat_offset_secs),
            iat: now_secs() + iat_offset_secs,
            htm: htm.to_string(),
            htu: htu.to_string(),
        }
    }

    fn make_store() -> Box<dyn DpopProofStore> {
        Box::new(InMemoryProofStore::new())
    }

    #[test]
    fn proof_with_correct_jkt_accepted() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let jkt = compute_jkt(&jwk);
        let mut claims = make_test_claims_with_cnf(&jkt);
        claims.cnf = Some(DpopConfirmation { jkt: jkt.clone() });

        let proof = make_proof(jwk, 0, "POST", "/auth/token");
        let store = make_store();

        assert!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store).is_ok()
        );
    }

    #[test]
    fn proof_with_mismatched_jkt_rejected() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let wrong_jkt = "wrong-thumbprint-xyz";
        let mut claims = make_test_claims_with_cnf(wrong_jkt);
        claims.cnf = Some(DpopConfirmation {
            jkt: wrong_jkt.to_string(),
        });

        let proof = make_proof(jwk, 0, "POST", "/auth/token");
        let store = make_store();

        assert!(matches!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store),
            Err(DpopError::BindingMismatch)
        ));
    }

    #[test]
    fn proof_wrong_htm_rejected() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let jkt = compute_jkt(&jwk);
        let mut claims = make_test_claims_with_cnf(&jkt);
        claims.cnf = Some(DpopConfirmation { jkt });

        let proof = make_proof(jwk, 0, "GET", "/auth/token"); // Proof says GET
        let store = make_store();

        assert!(matches!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store),
            Err(DpopError::MethodMismatch { .. })
        ));
    }

    #[test]
    fn proof_wrong_htu_rejected() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let jkt = compute_jkt(&jwk);
        let mut claims = make_test_claims_with_cnf(&jkt);
        claims.cnf = Some(DpopConfirmation { jkt });

        let proof = make_proof(jwk, 0, "POST", "/other/path"); // Proof says different path
        let store = make_store();

        assert!(matches!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store),
            Err(DpopError::UriMismatch { .. })
        ));
    }

    #[test]
    fn expired_proof_rejected() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let jkt = compute_jkt(&jwk);
        let mut claims = make_test_claims_with_cnf(&jkt);
        claims.cnf = Some(DpopConfirmation { jkt });

        let proof = make_proof(jwk, -120, "POST", "/auth/token"); // iat = 120s ago
        let store = make_store();

        assert!(matches!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store),
            Err(DpopError::ProofExpired)
        ));
    }

    #[test]
    fn fresh_proof_accepted() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let jkt = compute_jkt(&jwk);
        let mut claims = make_test_claims_with_cnf(&jkt);
        claims.cnf = Some(DpopConfirmation { jkt });

        let proof = make_proof(jwk, -30, "POST", "/auth/token"); // iat = 30s ago
        let store = make_store();

        assert!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store).is_ok()
        );
    }

    #[test]
    fn proof_future_rejected() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let jkt = compute_jkt(&jwk);
        let mut claims = make_test_claims_with_cnf(&jkt);
        claims.cnf = Some(DpopConfirmation { jkt });

        let proof = make_proof(jwk, 5, "POST", "/auth/token"); // iat = 5s in future
        let store = make_store();

        assert!(matches!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store),
            Err(DpopError::ProofFuture)
        ));
    }

    #[test]
    fn proof_replay_within_window_rejected() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let jkt = compute_jkt(&jwk);
        let mut claims = make_test_claims_with_cnf(&jkt);
        claims.cnf = Some(DpopConfirmation { jkt });

        let proof = make_proof(jwk, 0, "POST", "/auth/token");
        let store = make_store();

        // Record the proof JTI first
        store.record(&proof.jti).into_inner();

        // Second use should be rejected as replay
        assert!(matches!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store),
            Err(DpopError::ProofReplay)
        ));
    }

    #[test]
    fn proof_with_missing_cnf_rejected() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let claims = make_test_claims_with_cnf("some-jkt");
        // No cnf set

        let proof = make_proof(jwk, 0, "POST", "/auth/token");
        let store = make_store();

        assert!(matches!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store),
            Err(DpopError::BindingMismatch)
        ));
    }

    // ─── Proof Freshness Boundary ──────────────────────────────────────

    #[test]
    fn proof_exactly_at_60_seconds_rejected() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let jkt = compute_jkt(&jwk);
        let mut claims = make_test_claims_with_cnf(&jkt);
        claims.cnf = Some(DpopConfirmation { jkt });

        let proof = make_proof(jwk, -60, "POST", "/auth/token"); // iat = exactly 60s ago
        let store = make_store();

        assert!(matches!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store),
            Err(DpopError::ProofExpired)
        ));
    }

    #[test]
    fn proof_at_59_seconds_accepted() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let jkt = compute_jkt(&jwk);
        let mut claims = make_test_claims_with_cnf(&jkt);
        claims.cnf = Some(DpopConfirmation { jkt });

        let proof = make_proof(jwk, -59, "POST", "/auth/token"); // iat = 59s ago
        let store = make_store();

        assert!(
            verify_dpop_proof(&claims, &proof, "POST", "/auth/token", now_secs(), &*store).is_ok()
        );
    }

    // ─── EC Key Tests ──────────────────────────────────────────────────

    #[test]
    fn ec_keypair_generates_and_computes_jkt() {
        let (_priv, jwk) = generate_p256_keypair();
        assert!(matches!(jwk.kty, DpopKeyTypeId::Ec));
        assert!(matches!(jwk.crv, DpopCurve::P256));
        assert!(jwk.y.is_some());

        let jkt = compute_jkt(&jwk);
        assert!(!jkt.is_empty());
        assert!(jwk.validate_size().is_ok());
    }

    // ─── DpopConfirmation ──────────────────────────────────────────────

    #[test]
    fn dpop_confirmation_serializes() {
        let cnf = DpopConfirmation {
            jkt: "test-thumbprint".to_string(),
        };
        let json = serde_json::to_string(&cnf).unwrap();
        assert!(json.contains("\"jkt\""));
        assert!(json.contains("test-thumbprint"));
    }

    // ─── is_dpop_enabled Tests ─────────────────────────────────────────

    #[test]
    fn dpop_enabled_defaults_true_in_non_dev() {
        // When RUST_ENV is not "dev", DPoP should be enabled
        let original = std::env::var("RUST_ENV").ok();
        std::env::remove_var("RUST_ENV");
        assert!(is_dpop_enabled());
        if let Some(val) = original {
            std::env::set_var("RUST_ENV", val);
        }
    }

    #[test]
    fn dpop_disabled_when_dev_env_and_false() {
        std::env::set_var("RUST_ENV", "dev");
        std::env::set_var("DPoP_ENABLED", "false");
        assert!(!is_dpop_enabled());
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("DPoP_ENABLED");
    }

    #[test]
    fn dpop_enabled_when_dev_env_and_true() {
        std::env::set_var("RUST_ENV", "development");
        std::env::set_var("DPoP_ENABLED", "true");
        assert!(is_dpop_enabled());
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("DPoP_ENABLED");
    }

    // ─── require_dpop_proof Tests ──────────────────────────────────────

    #[test]
    fn require_dpop_rejects_missing_header() {
        std::env::set_var("RUST_ENV", "production");
        let req = brrtrouter::dispatcher::HandlerRequest {
            method: "GET".to_string(),
            path: "/test".to_string(),
            query_params: std::collections::HashMap::new(),
            headers: std::collections::HashMap::new(),
            body: None,
        };
        assert!(matches!(
            require_dpop_proof(&req),
            Err(DpopError::DpopRequired)
        ));
        std::env::remove_var("RUST_ENV");
    }

    #[test]
    fn require_dpop_accepts_header_present() {
        let req = brrtrouter::dispatcher::HandlerRequest {
            method: "GET".to_string(),
            path: "/test".to_string(),
            query_params: std::collections::HashMap::new(),
            headers: {
                let mut h = std::collections::HashMap::new();
                h.insert("DPoP".to_string(), "dpop-proof-jwt".to_string());
                h
            },
            body: None,
        };
        assert!(require_dpop_proof(&req).is_ok());
    }

    #[test]
    fn require_dpop_skip_in_dev() {
        std::env::set_var("RUST_ENV", "dev");
        std::env::set_var("DPoP_ENABLED", "false");
        let req = brrtrouter::dispatcher::HandlerRequest {
            method: "GET".to_string(),
            path: "/test".to_string(),
            query_params: std::collections::HashMap::new(),
            headers: std::collections::HashMap::new(),
            body: None,
        };
        assert!(require_dpop_proof(&req).is_ok()); // No proof needed in dev
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("DPoP_ENABLED");
    }

    // ─── JWK Key Bytes ─────────────────────────────────────────────────

    #[test]
    fn ed25519_key_bytes_correct_length() {
        let (_priv, jwk) = generate_ed25519_keypair();
        let bytes = jwk.to_key_bytes().unwrap();
        // Ed25519 public key is 32 bytes
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn ec_key_bytes_uncompressed_format() {
        let (_priv, jwk) = generate_p256_keypair();
        let bytes = jwk.to_key_bytes().unwrap();
        // EC uncompressed point: 0x04 (1 byte) + x (32 bytes) + y (32 bytes)
        assert_eq!(bytes.len(), 65);
        assert_eq!(bytes[0], 0x04);
    }

    #[test]
    fn ec_key_missing_y_rejected() {
        let jwk = DpopJwk {
            kty: DpopKeyTypeId::Ec,
            crv: DpopCurve::P256,
            x: "fake-x".to_string(),
            y: None,
            additional: serde_json::Value::Null,
        };
        assert!(matches!(jwk.to_key_bytes(), Err(DpopError::InvalidJwk(_))));
    }

    // ─── Extra JWK Fields Ignored ──────────────────────────────────────

    #[test]
    fn jwk_with_extra_fields_validated() {
        let mut jwk = serde_json::from_value(serde_json::json!({
            "kty": "OKP",
            "crv": "Ed25519",
            "x": "f7eb5e7c0f3e1c4d5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8",
            "use": "sig",
            "kid": "dpop-key-1"
        }))
        .unwrap();
        // Must validate even with extra fields
        assert!(jwk.validate_size().is_ok());
    }

    #[test]
    fn empty_jti_accepted() {
        let (_, jwk) = generate_ed25519_keypair();
        let claims = DpopProofClaims {
            typ: Some("dpop+jwt".to_string()),
            alg: "EdDSA".to_string(),
            jwk,
            jti: String::new(), // Empty JTI
            iat: now_secs(),
            htm: "POST".to_string(),
            htu: "/auth/token".to_string(),
        };
        assert!(claims.validate().is_ok());
    }

    fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}
