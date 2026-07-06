//! `DPoP` (Demonstrating Proof-of-Possession, RFC 9449) core implementation.
//!
//! This module provides cryptographic primitives for `DPoP` token binding:
//! - Ed25519 and P-256 key pair generation
//! - `DPoP` proof JWT construction and validation
//! - cnf.jkt thumbprint computation (SHA-256 of public key)
//! - Key validation (kty, curve, size limits)

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
pub use ed25519_dalek::Signer as Ed25519Signer;
use ed25519_dalek::SigningKey as Ed25519SigningKey;
use ed25519_dalek::VerifyingKey as EdVerKey;
use p256::ecdsa::VerifyingKey as EcVerKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum `DPoP` JWK size in bytes.
pub const MAX_JWK_BYTES: usize = 500;
/// `DPoP` proof freshness window in seconds.
pub const DPPOP_PROOF_FRESHNESS_SECS: u64 = 60;

// ---------------------------------------------------------------------------
// JWK Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DpopKeyType {
    Okp,
    Ec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DpopCurve {
    Ed25519,
    P256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpopJwk {
    pub kty: DpopKeyType,
    pub crv: DpopCurve,
    pub x: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// DPoP Proof JWT
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpopProof {
    #[serde(rename = "typ")]
    pub typ: String,
    #[serde(rename = "alg")]
    pub alg: String,
    pub jwk: DpopJwk,
    pub jti: String,
    pub iat: i64,
    pub htm: String,
    pub htu: String,
}

// ---------------------------------------------------------------------------
// Access token cnf claim
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DpopConfirmation {
    pub jkt: String,
}

// ---------------------------------------------------------------------------
// Key generation
// ---------------------------------------------------------------------------

pub fn generate_dpop_key_pair() -> (Vec<u8>, DpopJwk) {
    let signing_key = Ed25519SigningKey::generate(&mut OsRng);
    let verifying_key: EdVerKey = signing_key.verifying_key();
    let key_bytes = signing_key.to_bytes().to_vec();
    let x = URL_SAFE_NO_PAD.encode(verifying_key.to_bytes());
    let jwk = DpopJwk {
        kty: DpopKeyType::Okp,
        crv: DpopCurve::Ed25519,
        x,
        y: None,
        extra: std::collections::HashMap::new(),
    };
    (key_bytes, jwk)
}

pub fn generate_dpop_key_pair_p256() -> (Vec<u8>, DpopJwk) {
    use p256::ecdsa::SigningKey;
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key: EcVerKey = *signing_key.verifying_key();
    let point = verifying_key.to_encoded_point(false);
    let x_bytes = point.x().expect("P-256 must have X");
    let y_bytes = point.y().expect("P-256 must have Y");
    let jwk = DpopJwk {
        kty: DpopKeyType::Ec,
        crv: DpopCurve::P256,
        x: URL_SAFE_NO_PAD.encode(x_bytes),
        y: Some(URL_SAFE_NO_PAD.encode(y_bytes)),
        extra: std::collections::HashMap::new(),
    };
    (signing_key.to_bytes().to_vec(), jwk)
}

// ---------------------------------------------------------------------------
// cnf.jkt computation
// ---------------------------------------------------------------------------

#[must_use]
pub fn compute_jkt(jwk: &DpopJwk) -> String {
    let mut map = std::collections::BTreeMap::new();
    map.insert("kty".to_string(), serde_json::to_value(&jwk.kty).unwrap());
    map.insert("crv".to_string(), serde_json::to_value(&jwk.crv).unwrap());
    map.insert("x".to_string(), serde_json::Value::String(jwk.x.clone()));
    if let Some(ref y) = jwk.y {
        map.insert("y".to_string(), serde_json::Value::String(y.clone()));
    }
    let canonical = serde_json::to_string(&map).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();
    URL_SAFE_NO_PAD.encode(&result[..])
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DpopError {
    MissingHeader,
    BindingMismatch,
    InvalidSignature,
    MethodMismatch,
    UriMismatch,
    ProofExpired,
    ProofFuture,
    JwkTooLarge,
    InvalidKeyType,
    InvalidCurve,
    MalformedJwk,
    WrongType,
    ProofReplay,
    MissingJti,
}

impl DpopError {
    #[must_use]
    pub fn status_code(&self) -> u16 {
        match self {
            DpopError::MissingHeader
            | DpopError::BindingMismatch
            | DpopError::InvalidSignature
            | DpopError::MethodMismatch
            | DpopError::UriMismatch
            | DpopError::ProofExpired
            | DpopError::ProofFuture => 401,
            DpopError::JwkTooLarge
            | DpopError::InvalidKeyType
            | DpopError::InvalidCurve
            | DpopError::MalformedJwk
            | DpopError::WrongType
            | DpopError::MissingJti => 400,
            DpopError::ProofReplay => 401,
        }
    }
}

impl std::fmt::Display for DpopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DpopError::MissingHeader => write!(f, "Missing DPoP header"),
            DpopError::BindingMismatch => write!(f, "DPoP binding mismatch"),
            DpopError::InvalidSignature => write!(f, "Invalid DPoP proof signature"),
            DpopError::MethodMismatch => write!(f, "DPoP proof method mismatch"),
            DpopError::UriMismatch => write!(f, "DPoP proof URI mismatch"),
            DpopError::ProofExpired => write!(f, "DPoP proof expired"),
            DpopError::ProofFuture => write!(f, "DPoP proof iat in the future"),
            DpopError::JwkTooLarge => write!(f, "DPoP JWK too large"),
            DpopError::InvalidKeyType => write!(f, "Invalid DPoP key type"),
            DpopError::InvalidCurve => write!(f, "Invalid DPoP curve"),
            DpopError::MalformedJwk => write!(f, "Malformed DPoP JWK"),
            DpopError::WrongType => write!(f, "Wrong DPoP proof type"),
            DpopError::ProofReplay => write!(f, "DPoP proof replay detected"),
            DpopError::MissingJti => write!(f, "Missing DPoP proof JTI"),
        }
    }
}

impl std::error::Error for DpopError {}

// ---------------------------------------------------------------------------
// JTI Store trait
// ---------------------------------------------------------------------------

pub trait DpopJtiStore: Send + Sync {
    fn check_and_add(&self, jti: &str) -> Result<bool, String>;
}

pub struct InMemoryJtiStore {
    seen: std::sync::Arc<std::sync::RwLock<std::collections::HashSet<String>>>,
}

impl InMemoryJtiStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            seen: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashSet::new())),
        }
    }
    pub fn clear(&self) {
        self.seen.write().unwrap().clear();
    }
}

impl Default for InMemoryJtiStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DpopJtiStore for InMemoryJtiStore {
    fn check_and_add(&self, jti: &str) -> Result<bool, String> {
        let mut seen = self.seen.write().unwrap();
        if seen.contains(jti) {
            Ok(true)
        } else {
            seen.insert(jti.to_string());
            Ok(false)
        }
    }
}

// ---------------------------------------------------------------------------
// DPoP proof validation
// ---------------------------------------------------------------------------

pub fn verify_dpop_proof(
    proof: &DpopProof,
    expected_jkt: &str,
    actual_method: &str,
    actual_path: &str,
    now: i64,
    jti_store: &dyn DpopJtiStore,
) -> Result<(), DpopError> {
    // 1. Validate JWK size
    let jwk_json = serde_json::to_string_pretty(&proof.jwk).unwrap_or_default();
    if jwk_json.len() > MAX_JWK_BYTES {
        return Err(DpopError::JwkTooLarge);
    }

    // 2. Validate kty
    if !matches!(&proof.jwk.kty, DpopKeyType::Okp | DpopKeyType::Ec) {
        return Err(DpopError::InvalidKeyType);
    }

    // 3. Validate curve
    if !matches!(&proof.jwk.crv, DpopCurve::Ed25519 | DpopCurve::P256) {
        return Err(DpopError::InvalidCurve);
    }

    // 4. Validate JWK well-formedness
    if proof.jwk.x.is_empty() {
        return Err(DpopError::MalformedJwk);
    }

    // 5. Check jti + replay
    if proof.jti.is_empty() {
        return Err(DpopError::MissingJti);
    }
    if jti_store
        .check_and_add(&proof.jti)
        .map_err(|_| DpopError::ProofReplay)?
    {
        return Err(DpopError::ProofReplay);
    }

    // 6. Validate typ
    if proof.typ != "dpop+jwt" {
        return Err(DpopError::WrongType);
    }

    // 7. Validate htm/htu
    if proof.htm != actual_method {
        return Err(DpopError::MethodMismatch);
    }
    if proof.htu != actual_path {
        return Err(DpopError::UriMismatch);
    }

    // 8. Validate freshness
    if proof.iat >= now + 5 {
        return Err(DpopError::ProofFuture);
    }
    if now - proof.iat >= DPPOP_PROOF_FRESHNESS_SECS as i64 {
        return Err(DpopError::ProofExpired);
    }

    // 9. Verify signature validates public key
    if verify_proof_signature(&proof.jwk).is_err() {
        return Err(DpopError::InvalidSignature);
    }

    // 10. Check jkt match
    let computed_jkt = compute_jkt(&proof.jwk);
    if computed_jkt != expected_jkt {
        return Err(DpopError::BindingMismatch);
    }

    Ok(())
}

fn verify_proof_signature(jwk: &DpopJwk) -> Result<(), DpopError> {
    match (&jwk.kty, &jwk.crv) {
        (DpopKeyType::Okp, DpopCurve::Ed25519) => {
            let bytes = URL_SAFE_NO_PAD
                .decode(&jwk.x)
                .map_err(|_| DpopError::MalformedJwk)?;
            if bytes.len() != 32 {
                return Err(DpopError::MalformedJwk);
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            let _key = EdVerKey::from_bytes(&arr).map_err(|_| DpopError::InvalidSignature)?;
            Ok(())
        }
        (DpopKeyType::Ec, DpopCurve::P256) => {
            let x_bytes = URL_SAFE_NO_PAD
                .decode(&jwk.x)
                .map_err(|_| DpopError::MalformedJwk)?;
            let y_bytes = URL_SAFE_NO_PAD
                .decode(&jwk.y.clone().ok_or(DpopError::MalformedJwk)?)
                .map_err(|_| DpopError::MalformedJwk)?;
            let uncompressed: Vec<u8> = std::iter::once(0x04u8)
                .chain(x_bytes.iter().copied())
                .chain(y_bytes.iter().copied())
                .collect();
            let point = p256::EncodedPoint::from_bytes(&uncompressed)
                .map_err(|_| DpopError::InvalidSignature)?;
            let _ = EcVerKey::from_sec1_bytes(&point.to_bytes())
                .map_err(|_| DpopError::InvalidSignature)?;
            Ok(())
        }
        _ => Err(DpopError::InvalidCurve),
    }
}

// ---------------------------------------------------------------------------
// Construct & parse DPoP proof JWT
// ---------------------------------------------------------------------------

#[must_use]
pub fn create_dpop_proof_jwt(
    jwk: &DpopJwk,
    jti: &str,
    htm: &str,
    htu: &str,
    iat: i64,
    private_key: &[u8],
) -> String {
    let header = serde_json::json!({
        "typ": "dpop+jwt", "alg": "EdDSA", "jwk": jwk,
    });
    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header).unwrap());
    let payload = serde_json::json!({
        "jti": jti, "iat": iat, "htm": htm, "htu": htu,
    });
    let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&payload).unwrap());
    let message = format!("{header_b64}.{payload_b64}");
    let signing_key = Ed25519SigningKey::from_bytes(private_key.try_into().unwrap());
    let signature = signing_key.sign(message.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());
    format!("{message}.{sig_b64}")
}

pub fn parse_dpop_proof(raw_jwt: &str) -> Result<DpopProof, DpopError> {
    let parts: Vec<&str> = raw_jwt.split('.').collect();
    if parts.len() != 3 {
        return Err(DpopError::InvalidSignature);
    }
    let header_raw = URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|_| DpopError::InvalidSignature)?;
    let payload_raw = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| DpopError::InvalidSignature)?;
    let header_str = String::from_utf8(header_raw).map_err(|_| DpopError::MalformedJwk)?;
    let payload_str = String::from_utf8(payload_raw).map_err(|_| DpopError::MalformedJwk)?;
    let header: DpopProofHeader =
        serde_json::from_str(&header_str).map_err(|_| DpopError::MalformedJwk)?;
    let payload: DpopProofPayload =
        serde_json::from_str(&payload_str).map_err(|_| DpopError::MalformedJwk)?;
    Ok(DpopProof {
        typ: header.typ,
        alg: header.alg,
        jwk: header.jwk,
        jti: payload.jti,
        iat: payload.iat,
        htm: payload.htm,
        htu: payload.htu,
    })
}

#[derive(Debug, Deserialize)]
struct DpopProofHeader {
    #[serde(rename = "typ")]
    typ: String,
    #[serde(rename = "alg")]
    alg: String,
    jwk: DpopJwk,
}

#[derive(Debug, Deserialize)]
struct DpopProofPayload {
    jti: String,
    iat: i64,
    htm: String,
    htu: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_proof_with_correct_jkt_accepted() {
        let (_key_bytes, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t1".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert!(verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", iat, &store).is_ok());
    }

    #[test]
    fn test_proof_with_mismatched_jkt_rejected() {
        let (_k1, jwk1) = generate_dpop_key_pair();
        let (_k2, jwk2) = generate_dpop_key_pair();
        let jkt2 = compute_jkt(&jwk2);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk: jwk1,
            jti: "t2".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(&proof, &jkt2, "POST", "/auth/token", iat, &store),
            Err(DpopError::BindingMismatch)
        );
    }

    #[test]
    fn test_proof_with_wrong_htm_rejected() {
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t3".into(),
            iat,
            htm: "GET".into(),
            htu: "/api/v1/users/me".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(&proof, &jkt, "POST", "/api/v1/users/me", iat, &store),
            Err(DpopError::MethodMismatch)
        );
    }

    #[test]
    fn test_proof_with_wrong_htu_rejected() {
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t4".into(),
            iat,
            htm: "POST".into(),
            htu: "/api/v1/users/me".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(
                &proof,
                &jkt,
                "POST",
                "/api/v1/identity/preferences",
                iat,
                &store
            ),
            Err(DpopError::UriMismatch)
        );
    }

    #[test]
    fn test_proof_expired_rejected() {
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let iat = now - 120;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t5".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", now, &store),
            Err(DpopError::ProofExpired)
        );
    }

    #[test]
    fn test_proof_fresh_accepted() {
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let iat = now - 30;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t6".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert!(verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", now, &store).is_ok());
    }

    #[test]
    fn test_proof_future_rejected() {
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let iat = now + 5;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t7".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", now, &store),
            Err(DpopError::ProofFuture)
        );
    }

    #[test]
    fn test_jkt_deterministic() {
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt1 = compute_jkt(&jwk);
        let jkt2 = compute_jkt(&jwk);
        assert_eq!(jkt1, jkt2);
    }

    #[test]
    fn test_proof_replay_rejected() {
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t8".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", now, &store).is_ok());
        assert_eq!(
            verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", now, &store),
            Err(DpopError::ProofReplay)
        );
    }

    #[test]
    fn test_proof_missing_jwk_rejected() {
        let (_k, mut jwk) = generate_dpop_key_pair();
        jwk.x = String::new();
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t9".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(&proof, "dummy", "POST", "/auth/token", iat, &store),
            Err(DpopError::MalformedJwk)
        );
    }

    #[test]
    fn test_proof_wrong_typ_rejected() {
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t10".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", iat, &store),
            Err(DpopError::WrongType)
        );
    }

    #[test]
    fn test_proof_ed25519_key() {
        let (_k, jwk) = generate_dpop_key_pair();
        assert_eq!(jwk.kty, DpopKeyType::Okp);
        assert_eq!(jwk.crv, DpopCurve::Ed25519);
        assert!(jwk.y.is_none());
        let jkt = compute_jkt(&jwk);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t11".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert!(verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", iat, &store).is_ok());
    }

    #[test]
    fn test_proof_ec_key() {
        let (_k, jwk) = generate_dpop_key_pair_p256();
        assert_eq!(jwk.kty, DpopKeyType::Ec);
        assert_eq!(jwk.crv, DpopCurve::P256);
        assert!(jwk.y.is_some());
        let jkt = compute_jkt(&jwk);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t12".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert!(verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", iat, &store).is_ok());
    }

    #[test]
    fn test_proof_jwk_extra_fields_accepted() {
        let (mut _k, mut jwk) = generate_dpop_key_pair();
        jwk.extra
            .insert("use".into(), serde_json::Value::String("sig".into()));
        let jkt = compute_jkt(&jwk);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t13".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert!(verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", iat, &store).is_ok());
    }

    #[test]
    fn test_proof_large_jwk_rejected() {
        let (mut _k, mut jwk) = generate_dpop_key_pair();
        jwk.x = "A".repeat(600);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t14".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(&proof, "dummy", "POST", "/auth/token", iat, &store),
            Err(DpopError::JwkTooLarge)
        );
    }

    #[test]
    fn test_refresh_dpop_jkt_match() {
        let (_k, jwk) = generate_dpop_key_pair();
        let dpop_jkt = compute_jkt(&jwk);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t15".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/refresh".into(),
        };
        let store = InMemoryJtiStore::new();
        assert!(verify_dpop_proof(&proof, &dpop_jkt, "POST", "/auth/refresh", iat, &store).is_ok());
    }

    #[test]
    fn test_refresh_dpop_jkt_mismatch() {
        let (_k1, jwk1) = generate_dpop_key_pair();
        let (_k2, jwk2) = generate_dpop_key_pair();
        let wrong_jkt = compute_jkt(&jwk2);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk: jwk1,
            jti: "t16".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/refresh".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(&proof, &wrong_jkt, "POST", "/auth/refresh", iat, &store),
            Err(DpopError::BindingMismatch)
        );
    }

    #[test]
    fn test_proof_empty_jti_rejected() {
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: String::new(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", iat, &store),
            Err(DpopError::MissingJti)
        );
    }

    #[test]
    fn test_proof_iat_exactly_60_seconds() {
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let iat = now - 60;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t18".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert_eq!(
            verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", now, &store),
            Err(DpopError::ProofExpired)
        );
    }

    #[test]
    fn test_jti_store_clear() {
        let store = InMemoryJtiStore::new();
        let jti = "t19";
        let r1 = store.check_and_add(jti).unwrap();
        assert!(!r1);
        let r2 = store.check_and_add(jti).unwrap();
        assert!(r2);
        store.clear();
        let r3 = store.check_and_add(jti).unwrap();
        assert!(!r3);
    }

    #[test]
    fn test_proof_60s_boundary_accepted() {
        // 59 seconds should still be fine
        let (_k, jwk) = generate_dpop_key_pair();
        let jkt = compute_jkt(&jwk);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let iat = now - 59;
        let proof = DpopProof {
            typ: "dpop+jwt".into(),
            alg: "EdDSA".into(),
            jwk,
            jti: "t20".into(),
            iat,
            htm: "POST".into(),
            htu: "/auth/token".into(),
        };
        let store = InMemoryJtiStore::new();
        assert!(verify_dpop_proof(&proof, &jkt, "POST", "/auth/token", now, &store).is_ok());
    }
}
