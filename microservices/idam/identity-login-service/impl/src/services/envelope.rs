//! Envelope encryption for tenant-supplied credentials (ADR-009 Tier 2).
//!
//! # Why envelopes rather than one key or one Secret per tenant
//!
//! - **Not one shared key**: a single compromise would expose every tenant.
//! - **Not a k8s Secret per tenant**: thousands of Secrets is unworkable, and
//!   it drags tenant data into cluster config.
//!
//! So: each credential gets its own random **data key (DEK)** which encrypts
//! it with AES-256-GCM. The DEK is then itself encrypted ("wrapped") by a
//! long-lived **key encryption key (KEK)** held in the secret backend. Only
//! the ciphertext and the *wrapped* DEK go in the database.
//!
//! That buys three things: the DB alone is useless without the KEK; rotating
//! one tenant's credential touches only that row; and rotating the KEK
//! re-wraps DEKs without re-encrypting every credential.
//!
//! Step 1 (here) sources the KEK from env — `SMS_CREDENTIAL_KEK` (base64url,
//! 32 bytes), delivered like every other platform secret (SOPS → Secret →
//! env). Step 2 moves wrap/unwrap into the backend itself (OpenBao transit /
//! GCP KMS) so the KEK never enters process memory; the stored shape does not
//! change.
//!
//! Plaintext exists only inside [`decrypt`]'s return value — never logged,
//! never persisted, never echoed to an API.

use anyhow::{bail, Context, Result};
use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// A credential sealed for storage. Every field is safe to put in the DB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    /// AES-256-GCM ciphertext of the secret (base64url).
    pub ciphertext: String,
    /// Nonce used for the secret (base64url).
    pub nonce: String,
    /// The data key, encrypted under the KEK (base64url). Never the raw DEK.
    pub dek_wrapped: String,
}

fn kek() -> Result<[u8; 32]> {
    let raw = std::env::var("SMS_CREDENTIAL_KEK")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .context("SMS_CREDENTIAL_KEK not set — tenant credential custody is unavailable")?;
    let bytes = B64
        .decode(raw.trim())
        .context("SMS_CREDENTIAL_KEK is not base64url")?;
    if bytes.len() != 32 {
        bail!("SMS_CREDENTIAL_KEK must be 32 bytes (got {})", bytes.len());
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn seal_with(
    key_bytes: &[u8; 32],
    plaintext: &[u8],
    nonce_bytes: [u8; NONCE_LEN],
) -> Result<Vec<u8>> {
    let unbound = UnboundKey::new(&AES_256_GCM, key_bytes)
        .map_err(|_| anyhow::anyhow!("envelope: invalid key"))?;
    let key = LessSafeKey::new(unbound);
    let mut buf = plaintext.to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce_bytes),
        Aad::empty(),
        &mut buf,
    )
    .map_err(|_| anyhow::anyhow!("envelope: seal failed"))?;
    Ok(buf)
}

fn open_with(
    key_bytes: &[u8; 32],
    ciphertext: &[u8],
    nonce_bytes: [u8; NONCE_LEN],
) -> Result<Vec<u8>> {
    let unbound = UnboundKey::new(&AES_256_GCM, key_bytes)
        .map_err(|_| anyhow::anyhow!("envelope: invalid key"))?;
    let key = LessSafeKey::new(unbound);
    let mut buf = ciphertext.to_vec();
    let plain = key
        .open_in_place(
            Nonce::assume_unique_for_key(nonce_bytes),
            Aad::empty(),
            &mut buf,
        )
        .map_err(|_| anyhow::anyhow!("envelope: decryption failed (wrong key or tampered data)"))?;
    Ok(plain.to_vec())
}

fn random_bytes<const N: usize>() -> Result<[u8; N]> {
    let mut out = [0u8; N];
    SystemRandom::new()
        .fill(&mut out)
        .map_err(|_| anyhow::anyhow!("envelope: RNG failure"))?;
    Ok(out)
}

/// Seal a secret for storage: fresh DEK → encrypt secret → wrap DEK with KEK.
///
/// # Errors
///
/// Returns an error when the KEK is missing/invalid or the RNG fails.
pub fn encrypt(plaintext: &str) -> Result<Sealed> {
    encrypt_with(&kek()?, plaintext)
}

/// [`encrypt`] against an explicit KEK.
///
/// Step 2 (backend-held KEK) will call this with a key it never stores, and
/// tests use it to avoid depending on process env.
///
/// # Errors
///
/// Returns an error when the RNG or AEAD fails.
pub fn encrypt_with(kek: &[u8; 32], plaintext: &str) -> Result<Sealed> {
    let dek: [u8; 32] = random_bytes()?;

    let data_nonce: [u8; NONCE_LEN] = random_bytes()?;
    let ciphertext = seal_with(&dek, plaintext.as_bytes(), data_nonce)?;

    // Wrap the DEK. Its nonce is prefixed onto the wrapped blob so the row
    // needs only one extra column.
    let dek_nonce: [u8; NONCE_LEN] = random_bytes()?;
    let wrapped = seal_with(kek, &dek, dek_nonce)?;
    let mut dek_blob = dek_nonce.to_vec();
    dek_blob.extend_from_slice(&wrapped);

    Ok(Sealed {
        ciphertext: B64.encode(ciphertext),
        nonce: B64.encode(data_nonce),
        dek_wrapped: B64.encode(dek_blob),
    })
}

/// Unseal a stored secret: unwrap DEK with KEK → decrypt secret.
///
/// # Errors
///
/// Returns an error when the KEK is missing, the material is malformed, or
/// authentication fails (wrong key or tampered ciphertext).
pub fn decrypt(sealed: &Sealed) -> Result<String> {
    decrypt_with(&kek()?, sealed)
}

/// [`decrypt`] against an explicit KEK.
///
/// # Errors
///
/// Returns an error when the material is malformed or authentication fails.
pub fn decrypt_with(kek: &[u8; 32], sealed: &Sealed) -> Result<String> {
    let dek_blob = B64
        .decode(&sealed.dek_wrapped)
        .context("envelope: dek_wrapped base64")?;
    if dek_blob.len() <= NONCE_LEN {
        bail!("envelope: dek_wrapped too short");
    }
    let (dek_nonce_bytes, wrapped) = dek_blob.split_at(NONCE_LEN);
    let mut dek_nonce = [0u8; NONCE_LEN];
    dek_nonce.copy_from_slice(dek_nonce_bytes);
    let dek_vec = open_with(kek, wrapped, dek_nonce)?;
    if dek_vec.len() != 32 {
        bail!("envelope: unwrapped DEK has wrong length");
    }
    let mut dek = [0u8; 32];
    dek.copy_from_slice(&dek_vec);

    let nonce_bytes = B64
        .decode(&sealed.nonce)
        .context("envelope: nonce base64")?;
    if nonce_bytes.len() != NONCE_LEN {
        bail!("envelope: nonce has wrong length");
    }
    let mut data_nonce = [0u8; NONCE_LEN];
    data_nonce.copy_from_slice(&nonce_bytes);

    let ciphertext = B64
        .decode(&sealed.ciphertext)
        .context("envelope: ciphertext base64")?;
    let plain = open_with(&dek, &ciphertext, data_nonce)?;
    String::from_utf8(plain).context("envelope: plaintext is not UTF-8")
}

/// Generate a fresh KEK, base64url-encoded — for populating
/// `SMS_CREDENTIAL_KEK` in a SOPS secret.
///
/// # Errors
///
/// Returns an error if the RNG fails.
pub fn generate_kek() -> Result<String> {
    let key: [u8; 32] = random_bytes()?;
    Ok(B64.encode(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parallel test runner shares process env, so only the test that
    /// deliberately exercises the env path may touch `SMS_CREDENTIAL_KEK` —
    /// and it serializes. Every other test passes its KEK explicitly (this
    /// was a real cross-test flake: one test rotated the KEK mid-round-trip).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn kek_bytes() -> [u8; 32] {
        random_bytes().expect("rng")
    }

    #[test]
    fn round_trips() {
        let kek = kek_bytes();
        let sealed = encrypt_with(&kek, "super-secret-auth-token").unwrap();
        assert_eq!(
            decrypt_with(&kek, &sealed).unwrap(),
            "super-secret-auth-token"
        );
    }

    /// Each sealing uses a fresh DEK and nonce, so identical plaintexts do
    /// not produce identical ciphertexts (no equality oracle across rows).
    #[test]
    fn identical_plaintexts_seal_differently() {
        let kek = kek_bytes();
        let a = encrypt_with(&kek, "same").unwrap();
        let b = encrypt_with(&kek, "same").unwrap();
        assert_ne!(a.ciphertext, b.ciphertext);
        assert_ne!(a.dek_wrapped, b.dek_wrapped);
        assert_eq!(
            decrypt_with(&kek, &a).unwrap(),
            decrypt_with(&kek, &b).unwrap()
        );
    }

    /// The database alone is useless: a different KEK cannot open the row.
    #[test]
    fn wrong_kek_cannot_decrypt() {
        let sealed = encrypt_with(&kek_bytes(), "tenant-token").unwrap();
        assert!(decrypt_with(&kek_bytes(), &sealed).is_err());
    }

    /// GCM is authenticated: tampering is detected, not silently decrypted.
    #[test]
    fn tampered_ciphertext_is_rejected() {
        let kek = kek_bytes();
        let mut sealed = encrypt_with(&kek, "tenant-token").unwrap();
        let mut raw = B64.decode(&sealed.ciphertext).unwrap();
        raw[0] ^= 0xff;
        sealed.ciphertext = B64.encode(raw);
        assert!(decrypt_with(&kek, &sealed).is_err());
    }

    /// A missing KEK must fail loudly, never silently fall back to a
    /// hard-coded or empty key.
    #[test]
    fn missing_kek_is_an_error_not_a_default() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let restore = std::env::var("SMS_CREDENTIAL_KEK").ok();
        std::env::remove_var("SMS_CREDENTIAL_KEK");
        assert!(encrypt("x").is_err());
        if let Some(v) = restore {
            std::env::set_var("SMS_CREDENTIAL_KEK", v);
        }
    }

    /// The env path itself still works end to end.
    #[test]
    fn env_kek_round_trips() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let restore = std::env::var("SMS_CREDENTIAL_KEK").ok();
        std::env::set_var("SMS_CREDENTIAL_KEK", generate_kek().unwrap());
        let sealed = encrypt("via-env").unwrap();
        assert_eq!(decrypt(&sealed).unwrap(), "via-env");
        match restore {
            Some(v) => std::env::set_var("SMS_CREDENTIAL_KEK", v),
            None => std::env::remove_var("SMS_CREDENTIAL_KEK"),
        }
    }
}
