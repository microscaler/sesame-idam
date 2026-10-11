//! RS256 JWTs for ID tokens (ADR-015).
//!
//! Access and refresh tokens stay Ed25519 ([`super::Ed25519Signer`]); ID tokens are RS256, as `OpenID` Connect Core
//! §15.1 requires. Keys come from one of two backends, chosen by the JSON file named in [`ID_TOKEN_KEYS_ENV`]:
//!
//! ```json
//! {"backend": "gcp-kms",
//!  "key_versions": ["projects/p/locations/l/keyRings/r/cryptoKeys/id-token/cryptoKeyVersions/2",
//!                   "projects/p/locations/l/keyRings/r/cryptoKeys/id-token/cryptoKeyVersions/1"]}
//! {"backend": "file", "keys": ["/run/sesame/id-token/current.pem", "/run/sesame/id-token/previous.pem"]}
//! ```
//!
//! The first key signs; every listed key is published in the JWKS until removed. With Cloud KMS the private key
//! stays in an HSM: signing is `asymmetricSign` over the SHA-256 digest and the public key comes from
//! `getPublicKey`, so the JWKS publisher needs only `roles/cloudkms.publicKeyViewer`. The file backend (PKCS#8
//! PEM, signed with ring) is for development and tests. Each key's `kid` is its RFC 7638 thumbprint.

use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use ring::rand::SystemRandom;
use ring::signature::{RsaKeyPair, RSA_PKCS1_SHA256};

use crate::gcp_kms::{is_key_version_name, KmsClient, KmsError, KmsSettings};

/// Environment variable naming the ID-token key configuration file.
pub const ID_TOKEN_KEYS_ENV: &str = "SESAME_ID_TOKEN_KEYS";
/// Smallest RSA modulus accepted (bits).
const MIN_RSA_BITS: usize = 2048;
/// KMS algorithms accepted for RS256.
const KMS_RS256_ALGORITHMS: &[&str] = &[
    "RSA_SIGN_PKCS1_2048_SHA256",
    "RSA_SIGN_PKCS1_3072_SHA256",
    "RSA_SIGN_PKCS1_4096_SHA256",
];
/// After a failed load (KMS unreachable at start-up), wait this long before trying again.
const RETRY_AFTER: Duration = Duration::from_secs(30);
/// Largest configuration or key file accepted.
const MAX_FILE: u64 = 64 * 1024;

/// Why RS256 keys could not be loaded or used. Never contains key material or file contents.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Rs256Error {
    /// A file could not be read (the I/O error kind only).
    #[error("ID token key file unreadable: {0}")]
    Unreadable(String),
    /// Configuration or key not the expected shape.
    #[error("ID token key configuration malformed: {0}")]
    Malformed(&'static str),
    /// RSA key shorter than 2048 bits.
    #[error("RSA key shorter than 2048 bits")]
    TooShort,
    /// KMS failure.
    #[error(transparent)]
    Kms(#[from] KmsError),
    /// Local signing failed.
    #[error("RS256 signing failed")]
    Signing,
}

// ---------------------------------------------------------------------------
// Public keys
// ---------------------------------------------------------------------------

/// An RSA public key as published in the JWKS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RsaPublicJwk {
    /// RFC 7638 thumbprint.
    pub kid: String,
    /// Modulus, base64url without padding.
    pub n: String,
    /// Public exponent, base64url without padding.
    pub e: String,
}

impl RsaPublicJwk {
    /// From big-endian modulus and exponent (leading zeros stripped here).
    #[must_use]
    pub fn from_components(n: &[u8], e: &[u8]) -> Self {
        let strip = |b: &[u8]| {
            let first = b.iter().position(|&x| x != 0).unwrap_or(b.len());
            b[first..].to_vec()
        };
        let n = URL_SAFE_NO_PAD.encode(strip(n));
        let e = URL_SAFE_NO_PAD.encode(strip(e));
        Self {
            kid: rfc7638_rsa_thumbprint(&n, &e),
            n,
            e,
        }
    }

    /// The JWKS member: `{"kty":"RSA","use":"sig","alg":"RS256","kid","n","e"}`.
    #[must_use]
    pub fn to_jwk(&self) -> serde_json::Value {
        serde_json::json!({
            "kty": "RSA",
            "use": "sig",
            "alg": "RS256",
            "kid": self.kid,
            "n": self.n,
            "e": self.e,
        })
    }
}

/// RFC 7638 thumbprint of an RSA JWK: base64url(SHA-256(`{"e":…,"kty":"RSA","n":…}`)).
#[must_use]
pub fn rfc7638_rsa_thumbprint(n_b64: &str, e_b64: &str) -> String {
    let canonical = format!(r#"{{"e":"{e_b64}","kty":"RSA","n":"{n_b64}"}}"#);
    URL_SAFE_NO_PAD.encode(ring::digest::digest(
        &ring::digest::SHA256,
        canonical.as_bytes(),
    ))
}

/// One DER element: (tag, contents, rest).
fn der_element(input: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = input.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (len, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > 4 || rest.len() < count {
            return None;
        }
        let len = rest[..count]
            .iter()
            .fold(0usize, |acc, &b| (acc << 8) | usize::from(b));
        (len, &rest[count..])
    };
    if rest.len() < len {
        return None;
    }
    Some((tag, &rest[..len], &rest[len..]))
}

/// Modulus and exponent from a DER `RSAPublicKey` (`SEQUENCE { INTEGER n, INTEGER e }`, RFC 8017).
///
/// # Errors
///
/// [`Rs256Error::Malformed`] or [`Rs256Error::TooShort`].
pub fn parse_rsa_public_key_der(der: &[u8]) -> Result<RsaPublicJwk, Rs256Error> {
    let bad = Rs256Error::Malformed("RSAPublicKey");
    let (tag, seq, rest) = der_element(der).ok_or(bad.clone())?;
    if tag != 0x30 || !rest.is_empty() {
        return Err(bad);
    }
    let (tag_n, n, seq) = der_element(seq).ok_or(bad.clone())?;
    let (tag_e, e, seq) = der_element(seq).ok_or(bad.clone())?;
    if tag_n != 0x02 || tag_e != 0x02 || !seq.is_empty() {
        return Err(bad);
    }
    let jwk = RsaPublicJwk::from_components(n, e);
    let n_bits = URL_SAFE_NO_PAD.decode(&jwk.n).map_or(0, |b| {
        b.len() * 8 - b.first().map_or(8, |x| x.leading_zeros() as usize)
    });
    if n_bits < MIN_RSA_BITS {
        return Err(Rs256Error::TooShort);
    }
    Ok(jwk)
}

/// rsaEncryption, 1.2.840.113549.1.1.1.
const RSA_ENCRYPTION_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

/// Modulus and exponent from a PEM `SubjectPublicKeyInfo` of an RSA key (as KMS `getPublicKey` returns).
///
/// # Errors
///
/// [`Rs256Error::Malformed`] or [`Rs256Error::TooShort`].
pub fn parse_rsa_spki_pem(pem: &str) -> Result<RsaPublicJwk, Rs256Error> {
    let der = pem_body(pem, "PUBLIC KEY")?;
    let bad = Rs256Error::Malformed("SubjectPublicKeyInfo");
    let (tag, spki, rest) = der_element(&der).ok_or(bad.clone())?;
    if tag != 0x30 || !rest.is_empty() {
        return Err(bad);
    }
    let (tag_alg, alg, spki) = der_element(spki).ok_or(bad.clone())?;
    let (tag_oid, oid, _) = der_element(alg).ok_or(bad.clone())?;
    if tag_alg != 0x30 || tag_oid != 0x06 || oid != RSA_ENCRYPTION_OID {
        return Err(Rs256Error::Malformed("not an RSA key"));
    }
    let (tag_bits, bits, spki) = der_element(spki).ok_or(bad.clone())?;
    if tag_bits != 0x03 || !spki.is_empty() || bits.first() != Some(&0) {
        return Err(bad);
    }
    parse_rsa_public_key_der(&bits[1..])
}

fn pem_body(pem: &str, label: &str) -> Result<Vec<u8>, Rs256Error> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let body = pem
        .trim()
        .strip_prefix(&begin)
        .and_then(|s| s.strip_suffix(&end))
        .ok_or(Rs256Error::Malformed("PEM label"))?;
    let joined: String = body.split_whitespace().collect();
    STANDARD
        .decode(joined)
        .map_err(|_| Rs256Error::Malformed("PEM base64"))
}

// ---------------------------------------------------------------------------
// Signers
// ---------------------------------------------------------------------------

/// Signs RS256 (RSASSA-PKCS1-v1_5 with SHA-256).
pub trait Rs256Signer: Send + Sync {
    /// The signing key's public JWK.
    fn public_jwk(&self) -> &RsaPublicJwk;
    /// Signature over `message`.
    ///
    /// # Errors
    ///
    /// [`Rs256Error`].
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Rs256Error>;
}

/// A local RSA key (PKCS#8), signed with ring. Development and tests.
pub struct LocalRs256Key {
    pair: RsaKeyPair,
    jwk: RsaPublicJwk,
}

impl fmt::Debug for LocalRs256Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalRs256Key")
            .field("kid", &self.jwk.kid)
            .finish_non_exhaustive()
    }
}

impl LocalRs256Key {
    /// From PKCS#8 DER.
    ///
    /// # Errors
    ///
    /// [`Rs256Error::Malformed`] or [`Rs256Error::TooShort`].
    pub fn from_pkcs8_der(der: &[u8]) -> Result<Self, Rs256Error> {
        let pair =
            RsaKeyPair::from_pkcs8(der).map_err(|_| Rs256Error::Malformed("PKCS#8 RSA key"))?;
        let jwk = parse_rsa_public_key_der(pair.public().as_ref())?;
        Ok(Self { pair, jwk })
    }

    /// From a PEM `PRIVATE KEY` (PKCS#8), as `openssl genpkey -algorithm RSA` writes it.
    ///
    /// # Errors
    ///
    /// [`Rs256Error::Malformed`] or [`Rs256Error::TooShort`].
    pub fn from_pkcs8_pem(pem: &str) -> Result<Self, Rs256Error> {
        Self::from_pkcs8_der(&pem_body(pem, "PRIVATE KEY")?)
    }
}

impl Rs256Signer for LocalRs256Key {
    fn public_jwk(&self) -> &RsaPublicJwk {
        &self.jwk
    }

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Rs256Error> {
        let mut sig = vec![0u8; self.pair.public().modulus_len()];
        self.pair
            .sign(&RSA_PKCS1_SHA256, &SystemRandom::new(), message, &mut sig)
            .map_err(|_| Rs256Error::Signing)?;
        Ok(sig)
    }
}

/// An RSA key version in Cloud KMS (HSM). Signs through `asymmetricSign`.
#[derive(Debug)]
pub struct KmsRs256Key {
    version_name: String,
    jwk: RsaPublicJwk,
    client: Arc<KmsClient>,
}

impl KmsRs256Key {
    /// Fetch the version's public key and check its algorithm is RS256.
    ///
    /// # Errors
    ///
    /// [`Rs256Error::Kms`] when KMS fails, [`Rs256Error::Malformed`] for a non-RS256 key.
    pub fn new(client: Arc<KmsClient>, version_name: &str) -> Result<Self, Rs256Error> {
        if !is_key_version_name(version_name) {
            return Err(Rs256Error::Malformed("KMS key version name"));
        }
        let public = client.public_key(version_name)?;
        if !KMS_RS256_ALGORITHMS.contains(&public.algorithm.as_str()) {
            return Err(Rs256Error::Malformed("KMS key is not RSA PKCS#1 SHA-256"));
        }
        Ok(Self {
            version_name: version_name.to_string(),
            jwk: parse_rsa_spki_pem(&public.pem)?,
            client,
        })
    }
}

impl Rs256Signer for KmsRs256Key {
    fn public_jwk(&self) -> &RsaPublicJwk {
        &self.jwk
    }

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Rs256Error> {
        let digest = ring::digest::digest(&ring::digest::SHA256, message);
        let mut d = [0u8; 32];
        d.copy_from_slice(digest.as_ref());
        Ok(self.client.asymmetric_sign_sha256(&self.version_name, &d)?)
    }
}

// ---------------------------------------------------------------------------
// Key sets and configuration
// ---------------------------------------------------------------------------

/// Where ID-token keys come from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "backend", rename_all = "kebab-case", deny_unknown_fields)]
pub enum IdTokenKeysConfig {
    /// PKCS#8 PEM files; the first signs. Development and tests.
    File {
        /// Key files, current first.
        keys: Vec<PathBuf>,
    },
    /// Cloud KMS RSA key versions (HSM); the first signs.
    GcpKms {
        /// Key version names, current first.
        key_versions: Vec<String>,
        /// Connection settings (defaults: public endpoint, metadata server, HSM required).
        #[serde(default)]
        kms: Option<KmsSettings>,
    },
}

fn read_small(path: &std::path::Path) -> Result<Vec<u8>, Rs256Error> {
    let meta = std::fs::metadata(path).map_err(|e| Rs256Error::Unreadable(e.kind().to_string()))?;
    if meta.len() > MAX_FILE {
        return Err(Rs256Error::Malformed("file too large"));
    }
    std::fs::read(path).map_err(|e| Rs256Error::Unreadable(e.kind().to_string()))
}

impl IdTokenKeysConfig {
    /// Parse the configuration JSON.
    ///
    /// # Errors
    ///
    /// [`Rs256Error::Malformed`].
    pub fn from_json(bytes: &[u8]) -> Result<Self, Rs256Error> {
        serde_json::from_slice(bytes)
            .map_err(|_| Rs256Error::Malformed("ID token key configuration"))
    }

    /// The configuration named by [`ID_TOKEN_KEYS_ENV`], or `None` when unset.
    ///
    /// # Errors
    ///
    /// [`Rs256Error`] when the variable is set but the file is unreadable or malformed.
    pub fn from_env() -> Result<Option<Self>, Rs256Error> {
        match std::env::var(ID_TOKEN_KEYS_ENV) {
            Ok(path) if !path.trim().is_empty() => {
                Self::from_json(&read_small(std::path::Path::new(path.trim()))?).map(Some)
            }
            _ => Ok(None),
        }
    }

    /// Load the keys (for KMS: fetch every listed version's public key).
    ///
    /// # Errors
    ///
    /// [`Rs256Error`].
    pub fn load(&self) -> Result<IdTokenKeys, Rs256Error> {
        let signers: Vec<Box<dyn Rs256Signer>> = match self {
            Self::File { keys } => keys
                .iter()
                .map(|p| {
                    let pem = String::from_utf8(read_small(p)?)
                        .map_err(|_| Rs256Error::Malformed("PEM not UTF-8"))?;
                    Ok(Box::new(LocalRs256Key::from_pkcs8_pem(&pem)?) as Box<dyn Rs256Signer>)
                })
                .collect::<Result<_, Rs256Error>>()?,
            Self::GcpKms { key_versions, kms } => {
                let client = Arc::new(KmsClient::new(kms.clone().unwrap_or_default())?);
                key_versions
                    .iter()
                    .map(|v| {
                        Ok(Box::new(KmsRs256Key::new(client.clone(), v)?) as Box<dyn Rs256Signer>)
                    })
                    .collect::<Result<_, Rs256Error>>()?
            }
        };
        IdTokenKeys::new(signers)
    }
}

/// The ID-token keys: the first signs, all are published.
pub struct IdTokenKeys {
    keys: Vec<Box<dyn Rs256Signer>>,
}

impl fmt::Debug for IdTokenKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kids: Vec<&str> = self
            .keys
            .iter()
            .map(|k| k.public_jwk().kid.as_str())
            .collect();
        f.debug_struct("IdTokenKeys").field("kids", &kids).finish()
    }
}

impl IdTokenKeys {
    /// From signers, current first.
    ///
    /// # Errors
    ///
    /// [`Rs256Error::Malformed`] when empty or two entries are the same key.
    pub fn new(keys: Vec<Box<dyn Rs256Signer>>) -> Result<Self, Rs256Error> {
        if keys.is_empty() {
            return Err(Rs256Error::Malformed("no ID token keys"));
        }
        let mut kids: Vec<&str> = keys.iter().map(|k| k.public_jwk().kid.as_str()).collect();
        kids.sort_unstable();
        kids.dedup();
        if kids.len() != keys.len() {
            return Err(Rs256Error::Malformed("duplicate ID token key"));
        }
        Ok(Self { keys })
    }

    /// The `kid` new ID tokens carry.
    #[must_use]
    pub fn signing_kid(&self) -> &str {
        &self.keys[0].public_jwk().kid
    }

    /// Sign `claims` as a compact JWS: header `{"alg":"RS256","typ":"JWT","kid":…}`.
    ///
    /// # Errors
    ///
    /// [`Rs256Error`] when signing fails (for KMS: unreachable or failed checks).
    pub fn sign_jwt(&self, claims: &serde_json::Value) -> Result<String, Rs256Error> {
        let signer = &self.keys[0];
        let header =
            serde_json::json!({"alg": "RS256", "typ": "JWT", "kid": signer.public_jwk().kid});
        let signing_input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header.to_string()),
            URL_SAFE_NO_PAD.encode(claims.to_string())
        );
        let sig = signer.sign(signing_input.as_bytes())?;
        Ok(format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(sig)))
    }

    /// The JWKS members for every key.
    #[must_use]
    pub fn jwks(&self) -> Vec<serde_json::Value> {
        self.keys.iter().map(|k| k.public_jwk().to_jwk()).collect()
    }
}

/// The deployment's ID-token keys, from [`ID_TOKEN_KEYS_ENV`]. Not configured: ID tokens fall back to `EdDSA`
/// (development), and discovery says so.
///
/// # Panics
///
/// On first use, when [`ID_TOKEN_KEYS_ENV`] names an unreadable or malformed configuration: a deployment must not
/// start with a broken key configuration. Services force it at start-up.
#[allow(clippy::expect_used)]
pub static ID_TOKEN_KEYS: std::sync::LazyLock<LazyIdTokenKeys> =
    std::sync::LazyLock::new(|| {
        LazyIdTokenKeys::new(IdTokenKeysConfig::from_env().expect(
            "SESAME_ID_TOKEN_KEYS names an unreadable or malformed ID token key configuration",
        ))
    });

/// ID-token keys loaded on first use and retried after a failure (KMS unreachable at start-up must not leave a
/// service without keys for good).
pub struct LazyIdTokenKeys {
    config: Option<IdTokenKeysConfig>,
    state: Mutex<(Option<Arc<IdTokenKeys>>, Option<Instant>)>,
}

impl fmt::Debug for LazyIdTokenKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LazyIdTokenKeys")
            .field("configured", &self.config.is_some())
            .finish_non_exhaustive()
    }
}

impl LazyIdTokenKeys {
    /// From an optional configuration (`None`: RS256 ID tokens not configured).
    #[must_use]
    pub fn new(config: Option<IdTokenKeysConfig>) -> Self {
        Self {
            config,
            state: Mutex::new((None, None)),
        }
    }

    /// Whether RS256 ID tokens are configured (for discovery metadata).
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.config.is_some()
    }

    /// The keys: `Ok(None)` when not configured; loaded on first use, retried at most every 30 s after a failure.
    ///
    /// # Errors
    ///
    /// [`Rs256Error`] from the last load attempt.
    pub fn get(&self) -> Result<Option<Arc<IdTokenKeys>>, Rs256Error> {
        let Some(config) = &self.config else {
            return Ok(None);
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| Rs256Error::Malformed("ID token key state poisoned"))?;
        if let Some(keys) = &state.0 {
            return Ok(Some(keys.clone()));
        }
        if let Some(last) = state.1 {
            if last.elapsed() < RETRY_AFTER {
                return Err(Rs256Error::Kms(KmsError::Unavailable(
                    "ID token keys not loaded yet".to_string(),
                )));
            }
        }
        state.1 = Some(Instant::now());
        let keys = Arc::new(config.load()?);
        state.0 = Some(keys.clone());
        Ok(Some(keys))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gcp_kms::fake;
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    use std::sync::LazyLock;

    /// Test-only RSA keys, generated per test run (nothing private is committed).
    static TEST_KEYS: LazyLock<Vec<rsa::RsaPrivateKey>> = LazyLock::new(|| {
        let mut rng = rand::thread_rng();
        (0..2)
            .map(|_| rsa::RsaPrivateKey::new(&mut rng, 2048).expect("keygen"))
            .collect()
    });

    fn pkcs8_pem(i: usize) -> String {
        TEST_KEYS[i]
            .to_pkcs8_pem(LineEnding::LF)
            .expect("pem")
            .to_string()
    }

    fn spki_pem(i: usize) -> String {
        TEST_KEYS[i]
            .to_public_key()
            .to_public_key_pem(LineEnding::LF)
            .expect("pem")
    }

    fn verify_with_jsonwebtoken(token: &str, jwk: &RsaPublicJwk) -> serde_json::Value {
        let key = jsonwebtoken::DecodingKey::from_rsa_components(&jwk.n, &jwk.e).expect("key");
        let mut v = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        v.validate_exp = false;
        v.validate_aud = false;
        v.required_spec_claims.clear();
        let data = jsonwebtoken::decode::<serde_json::Value>(token, &key, &v).expect("verifies");
        assert_eq!(data.header.kid.as_deref(), Some(jwk.kid.as_str()));
        assert_eq!(data.header.typ.as_deref(), Some("JWT"));
        data.claims
    }

    #[test]
    fn rfc7638_example_thumbprint() {
        // RFC 7638 §3.1.
        let n = "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw";
        assert_eq!(
            rfc7638_rsa_thumbprint(n, "AQAB"),
            "NzbLsXh8uDCcd-6MNwXF4W_7noWXFZAfHkxZsRGC9Xs"
        );
    }

    #[test]
    fn local_key_signs_rs256_that_jsonwebtoken_accepts() {
        let key = LocalRs256Key::from_pkcs8_pem(&pkcs8_pem(0)).expect("key");
        let keys = IdTokenKeys::new(vec![Box::new(key)]).expect("keys");
        let claims = serde_json::json!({"iss": "https://idp.example", "sub": "u1", "nonce": "n"});
        let token = keys.sign_jwt(&claims).expect("signed");
        let jwk = parse_rsa_spki_pem(&spki_pem(0)).expect("spki");
        assert_eq!(jwk.kid, keys.signing_kid());
        assert_eq!(verify_with_jsonwebtoken(&token, &jwk), claims);
        let published = keys.jwks();
        assert_eq!(published[0]["kty"], "RSA");
        assert_eq!(published[0]["alg"], "RS256");
        assert_eq!(published[0]["kid"], keys.signing_kid());
    }

    #[test]
    fn kms_key_signs_through_asymmetric_sign() {
        let local = LocalRs256Key::from_pkcs8_pem(&pkcs8_pem(1)).expect("key");
        let version =
            "projects/sesame-test/locations/europe-west1/keyRings/sesame/cryptoKeys/id-token/cryptoKeyVersions/1";
        let claims = serde_json::json!({"sub": "u2"});
        // The fake signs by looking up the digest: precompute what KMS would return.
        let header =
            serde_json::json!({"alg": "RS256", "typ": "JWT", "kid": local.public_jwk().kid});
        let input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header.to_string()),
            URL_SAFE_NO_PAD.encode(claims.to_string())
        );
        let digest = ring::digest::digest(&ring::digest::SHA256, input.as_bytes())
            .as_ref()
            .to_vec();
        let mut script = fake::Script::default();
        script
            .signatures
            .insert(digest, local.sign(input.as_bytes()).expect("sig"));
        script.public_keys.insert(
            version.to_string(),
            (spki_pem(1), "RSA_SIGN_PKCS1_2048_SHA256".to_string()),
        );
        let (base, _) = fake::start(fake::Mode::Good, script);
        let config = IdTokenKeysConfig::GcpKms {
            key_versions: vec![version.to_string()],
            kms: Some(fake::settings(&base)),
        };
        let keys = config.load().expect("loads");
        assert_eq!(keys.signing_kid(), local.public_jwk().kid);
        let token = keys.sign_jwt(&claims).expect("signed");
        assert_eq!(verify_with_jsonwebtoken(&token, local.public_jwk()), claims);
    }

    #[test]
    fn kms_key_of_the_wrong_algorithm_is_refused() {
        let version =
            "projects/sesame-test/locations/europe-west1/keyRings/sesame/cryptoKeys/id-token/cryptoKeyVersions/1";
        let mut script = fake::Script::default();
        script.public_keys.insert(
            version.to_string(),
            (spki_pem(0), "EC_SIGN_P256_SHA256".to_string()),
        );
        let (base, _) = fake::start(fake::Mode::Good, script);
        let config = IdTokenKeysConfig::GcpKms {
            key_versions: vec![version.to_string()],
            kms: Some(fake::settings(&base)),
        };
        assert_eq!(
            config.load().err(),
            Some(Rs256Error::Malformed("KMS key is not RSA PKCS#1 SHA-256"))
        );
    }

    #[test]
    fn rotation_publishes_both_and_signs_with_the_first() {
        let a = LocalRs256Key::from_pkcs8_pem(&pkcs8_pem(0)).expect("key");
        let b = LocalRs256Key::from_pkcs8_pem(&pkcs8_pem(1)).expect("key");
        let (kid_a, kid_b) = (a.public_jwk().kid.clone(), b.public_jwk().kid.clone());
        let keys = IdTokenKeys::new(vec![Box::new(b), Box::new(a)]).expect("keys");
        assert_eq!(keys.signing_kid(), kid_b);
        let kids: Vec<String> = keys
            .jwks()
            .iter()
            .map(|j| j["kid"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(kids, vec![kid_b, kid_a]);
        let dup = IdTokenKeys::new(vec![
            Box::new(LocalRs256Key::from_pkcs8_pem(&pkcs8_pem(0)).expect("key")),
            Box::new(LocalRs256Key::from_pkcs8_pem(&pkcs8_pem(0)).expect("key")),
        ]);
        assert!(dup.is_err());
        assert!(IdTokenKeys::new(Vec::new()).is_err());
    }

    #[test]
    fn parsers_refuse_malformed_input() {
        assert!(parse_rsa_spki_pem("not pem").is_err());
        assert!(parse_rsa_public_key_der(&[0x30, 0x03, 0x02, 0x01, 0x01]).is_err());
        assert!(LocalRs256Key::from_pkcs8_pem(&spki_pem(0)).is_err());
        // A 1024-bit key is refused.
        let small = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 1024).expect("keygen");
        let pem = small
            .to_public_key()
            .to_public_key_pem(LineEnding::LF)
            .expect("pem");
        assert_eq!(parse_rsa_spki_pem(&pem).err(), Some(Rs256Error::TooShort));
    }

    #[test]
    fn configuration_parses_and_unset_means_none() {
        let cfg = IdTokenKeysConfig::from_json(
            br#"{"backend":"gcp-kms","key_versions":["projects/p1234/locations/l/keyRings/r/cryptoKeys/k/cryptoKeyVersions/1"]}"#,
        )
        .expect("parses");
        assert!(matches!(cfg, IdTokenKeysConfig::GcpKms { .. }));
        assert!(
            IdTokenKeysConfig::from_json(br#"{"backend":"file","keys":["/a.pem"],"x":1}"#).is_err()
        );
        let lazy = LazyIdTokenKeys::new(None);
        assert!(!lazy.is_configured());
        assert!(matches!(lazy.get(), Ok(None)));
    }

    #[test]
    fn lazy_keys_retry_after_a_failed_load() {
        let lazy = LazyIdTokenKeys::new(Some(IdTokenKeysConfig::File {
            keys: vec![PathBuf::from("/nonexistent/id-token.pem")],
        }));
        assert!(matches!(lazy.get(), Err(Rs256Error::Unreadable(_))));
        // Within the retry window the error is immediate and does not touch the file system again.
        assert!(matches!(
            lazy.get(),
            Err(Rs256Error::Kms(KmsError::Unavailable(_)))
        ));
    }
}
