//! TOTP (RFC 6238 over RFC 4226): HMAC-SHA1, 30-second steps, 6 digits — what every authenticator app implements.
//!
//! A code is accepted for the current step and one step either side (clock skew), and only for a step later than the
//! last one accepted for the authenticator ([`super::factors::advance_totp_step`] makes that check and the update in
//! one statement), so an intercepted code cannot be replayed, not even within its 90-second window.
//!
//! The shared secret is sealed before it is stored: AES-256-GCM under a key derived (HKDF-SHA256) from the
//! deployment's key-encryption key, with the tenant, user and factor id as associated data, so a sealed secret copied
//! to another row does not open. The KEK is `MFA_SECRET_KEK`, or the tenant-credential custody KEK
//! (`SMS_CREDENTIAL_KEK`, ADR-009) when no dedicated one is configured; the HKDF label keeps the two uses apart.

use anyhow::{bail, Context, Result};
use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::hkdf;
use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};

/// Seconds per step.
pub const STEP_SECS: i64 = 30;
/// Code length.
pub const DIGITS: u32 = 6;
/// Secret length (RFC 4226 §4 recommends 160 bits).
pub const SECRET_LEN: usize = 20;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// A fresh random secret.
///
/// # Errors
/// The system RNG failed.
pub fn generate_secret() -> Result<[u8; SECRET_LEN]> {
    let mut secret = [0u8; SECRET_LEN];
    SystemRandom::new()
        .fill(&mut secret)
        .map_err(|_| anyhow::anyhow!("totp: RNG failure"))?;
    Ok(secret)
}

/// RFC 4648 base32 without padding: the form authenticator apps take secrets in.
#[must_use]
pub fn base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

/// The HOTP value for `counter` (RFC 4226 §5.3), `DIGITS` long.
#[must_use]
pub fn hotp(secret: &[u8], counter: u64) -> u32 {
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret);
    let tag = hmac::sign(&key, &counter.to_be_bytes());
    let digest = tag.as_ref();
    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    let binary = (u32::from(digest[offset]) & 0x7f) << 24
        | u32::from(digest[offset + 1]) << 16
        | u32::from(digest[offset + 2]) << 8
        | u32::from(digest[offset + 3]);
    binary % 10_u32.pow(DIGITS)
}

/// The time step of unix time `t`.
#[must_use]
pub fn step_at(t: i64) -> i64 {
    t.div_euclid(STEP_SECS)
}

/// The step `code` is valid for at `now` (now, or one step either side), if any, and only if it is later than
/// `last_step`. Comparison does not short-circuit on the code's digits.
#[must_use]
pub fn matching_step(secret: &[u8], code: &str, now: i64, last_step: Option<i64>) -> Option<i64> {
    let code = code.trim();
    if code.len() != DIGITS as usize || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let presented: u32 = code.parse().ok()?;
    let current = step_at(now);
    let mut found = None;
    for step in [current - 1, current, current + 1] {
        let Ok(counter) = u64::try_from(step) else {
            continue;
        };
        let expected = hotp(secret, counter);
        // Constant-time on the value: XOR, then compare to zero.
        if (expected ^ presented) == 0 && last_step.is_none_or(|last| step > last) {
            found = Some(step);
        }
    }
    found
}

/// The `otpauth://` URI authenticator apps scan (Key Uri Format): issuer and account name label the entry.
#[must_use]
pub fn otpauth_uri(issuer: &str, account: &str, secret_b32: &str) -> String {
    let enc = |s: &str| url::form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>();
    let label = format!("{}:{}", enc(issuer), enc(account)).replace('+', "%20");
    format!(
        "otpauth://totp/{label}?secret={secret_b32}&issuer={}&algorithm=SHA1&digits={DIGITS}&period={STEP_SECS}",
        enc(issuer).replace('+', "%20")
    )
}

// ── sealing ─────────────────────────────────────────────────────────────────

/// The deployment KEK: `MFA_SECRET_KEK`, else `SMS_CREDENTIAL_KEK` (base64url, 32 bytes either way).
fn kek() -> Result<[u8; 32]> {
    let raw = ["MFA_SECRET_KEK", "SMS_CREDENTIAL_KEK"]
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|v| !v.trim().is_empty()))
        .context(
            "no key-encryption key (MFA_SECRET_KEK or SMS_CREDENTIAL_KEK): TOTP is unavailable",
        )?;
    let bytes = B64
        .decode(raw.trim())
        .context("the MFA key-encryption key is not base64url")?;
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("the MFA key-encryption key must be 32 bytes"))?;
    Ok(key)
}

/// Whether TOTP secrets can be sealed here (a KEK is configured).
#[must_use]
pub fn available() -> bool {
    kek().is_ok()
}

struct KeyLen;
impl hkdf::KeyType for KeyLen {
    fn len(&self) -> usize {
        32
    }
}

fn sealing_key(kek: &[u8; 32]) -> Result<LessSafeKey> {
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"sesame-idam/mfa");
    let prk = salt.extract(kek);
    let okm = prk
        .expand(&[b"totp-secret/v1"], KeyLen)
        .map_err(|_| anyhow::anyhow!("totp: key derivation failed"))?;
    let mut key = [0u8; 32];
    okm.fill(&mut key)
        .map_err(|_| anyhow::anyhow!("totp: key derivation failed"))?;
    let unbound =
        UnboundKey::new(&AES_256_GCM, &key).map_err(|_| anyhow::anyhow!("totp: bad key"))?;
    Ok(LessSafeKey::new(unbound))
}

fn aad(tenant_id: &str, user_id: &uuid::Uuid, factor_id: &uuid::Uuid) -> String {
    format!("totp:{tenant_id}:{user_id}:{factor_id}")
}

/// Seal a secret for storage (`v1.<nonce>.<ciphertext>`), bound to its tenant, user and factor.
///
/// # Errors
/// No KEK, or the RNG or AEAD failed.
pub fn seal(
    secret: &[u8],
    tenant_id: &str,
    user_id: &uuid::Uuid,
    factor_id: &uuid::Uuid,
) -> Result<String> {
    seal_with(&kek()?, secret, tenant_id, user_id, factor_id)
}

/// [`seal`] under an explicit KEK (tests).
///
/// # Errors
/// The RNG or AEAD failed.
pub fn seal_with(
    kek: &[u8; 32],
    secret: &[u8],
    tenant_id: &str,
    user_id: &uuid::Uuid,
    factor_id: &uuid::Uuid,
) -> Result<String> {
    let key = sealing_key(kek)?;
    let mut nonce = [0u8; NONCE_LEN];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| anyhow::anyhow!("totp: RNG failure"))?;
    let mut buf = secret.to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::from(aad(tenant_id, user_id, factor_id).as_bytes()),
        &mut buf,
    )
    .map_err(|_| anyhow::anyhow!("totp: seal failed"))?;
    Ok(format!("v1.{}.{}", B64.encode(nonce), B64.encode(buf)))
}

/// Open a sealed secret.
///
/// # Errors
/// No KEK, malformed input, or authentication failed (another key, another row, or tampering).
pub fn open(
    sealed: &str,
    tenant_id: &str,
    user_id: &uuid::Uuid,
    factor_id: &uuid::Uuid,
) -> Result<Vec<u8>> {
    open_with(&kek()?, sealed, tenant_id, user_id, factor_id)
}

/// [`open`] under an explicit KEK (tests).
///
/// # Errors
/// Malformed input or authentication failed.
pub fn open_with(
    kek: &[u8; 32],
    sealed: &str,
    tenant_id: &str,
    user_id: &uuid::Uuid,
    factor_id: &uuid::Uuid,
) -> Result<Vec<u8>> {
    let mut parts = sealed.split('.');
    let (Some("v1"), Some(nonce), Some(ct), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        bail!("totp: unrecognised sealed secret");
    };
    let nonce: [u8; NONCE_LEN] = B64
        .decode(nonce)
        .context("totp: nonce")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("totp: nonce length"))?;
    let mut buf = B64.decode(ct).context("totp: ciphertext")?;
    let key = sealing_key(kek)?;
    let plain = key
        .open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(aad(tenant_id, user_id, factor_id).as_bytes()),
            &mut buf,
        )
        .map_err(|_| anyhow::anyhow!("totp: sealed secret does not open"))?;
    Ok(plain.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC_SECRET: &[u8] = b"12345678901234567890";

    /// RFC 6238 Appendix B (SHA-1), last six digits.
    #[test]
    fn rfc6238_test_vectors() {
        for (t, code) in [
            (59, 287_082),
            (1_111_111_109, 81_804),
            (1_111_111_111, 50_471),
            (1_234_567_890, 5_924),
            (2_000_000_000, 279_037),
        ] {
            let counter = u64::try_from(step_at(t)).unwrap();
            assert_eq!(hotp(RFC_SECRET, counter), code, "t={t}");
        }
    }

    #[test]
    fn rfc4648_base32() {
        assert_eq!(base32(b""), "");
        assert_eq!(base32(b"f"), "MY");
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI");
        assert_eq!(base32(RFC_SECRET), "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
    }

    #[test]
    fn accepts_adjacent_steps_and_refuses_replay() {
        let now = 1_111_111_111;
        let code = format!(
            "{:06}",
            hotp(RFC_SECRET, u64::try_from(step_at(now)).unwrap())
        );
        let step = matching_step(RFC_SECRET, &code, now, None).expect("current step");
        assert_eq!(step, step_at(now));
        // 30 s later the same code still matches (previous step), once.
        assert_eq!(matching_step(RFC_SECRET, &code, now + 30, None), Some(step));
        assert_eq!(
            matching_step(RFC_SECRET, &code, now + 30, Some(step)),
            None,
            "a code is accepted once"
        );
        // 90 s later it is gone.
        assert_eq!(matching_step(RFC_SECRET, &code, now + 90, None), None);
    }

    #[test]
    fn refuses_malformed_codes() {
        for bad in ["", "12345", "1234567", "12a456", " 12 456"] {
            assert_eq!(matching_step(RFC_SECRET, bad, 59, None), None, "{bad:?}");
        }
    }

    #[test]
    fn otpauth_uri_is_scannable() {
        let uri = otpauth_uri("Price Whisperer", "a@b.co", "ABC");
        assert_eq!(
            uri,
            "otpauth://totp/Price%20Whisperer:a%40b.co?secret=ABC&issuer=Price%20Whisperer&algorithm=SHA1&digits=6&period=30"
        );
    }

    #[test]
    fn sealed_secret_opens_only_for_its_row() {
        let kek = [7u8; 32];
        let (u, f) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        let sealed = seal_with(&kek, RFC_SECRET, "acme", &u, &f).unwrap();
        assert!(sealed.starts_with("v1."));
        assert!(!sealed.contains("GEZDGNBV"));
        assert_eq!(
            open_with(&kek, &sealed, "acme", &u, &f).unwrap(),
            RFC_SECRET
        );
        assert!(open_with(&kek, &sealed, "other", &u, &f).is_err());
        assert!(open_with(&kek, &sealed, "acme", &uuid::Uuid::new_v4(), &f).is_err());
        assert!(open_with(&[8u8; 32], &sealed, "acme", &u, &f).is_err());
    }
}
