//! One-time recovery codes: the way back in when the authenticator app or passkey is lost (ADR-008 §6).
//!
//! Ten codes, shown once when the first factor is enrolled or when the user regenerates them (which invalidates the
//! old set). Each is 16 characters of Crockford base32 — 80 random bits — so the stored SHA-256 (salted with the
//! tenant and user) cannot be reversed by guessing, and no key is needed to check one. A code works once.
//!
//! Using one is a recovery event: audited and announced by email ([`super::notify`]). It gives `aal2`, never the
//! phishing-resistant class, so a member of an organisation that requires passkeys must enrol a new passkey before a
//! token is issued (the gate in [`super::gate`]).

use anyhow::Result;
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};

/// Codes per set.
pub const CODES_PER_SET: usize = 10;
/// Characters per code (5 bits each).
pub const CODE_CHARS: usize = 16;

/// Crockford base32: no I, L, O or U, so a code read aloud or retyped from paper survives.
const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// A fresh set of codes, formatted for display (`xxxx-xxxx-xxxx-xxxx`).
///
/// # Errors
/// The system RNG failed.
pub fn generate() -> Result<Vec<String>> {
    let rng = SystemRandom::new();
    (0..CODES_PER_SET)
        .map(|_| {
            let mut raw = [0u8; CODE_CHARS];
            rng.fill(&mut raw)
                .map_err(|_| anyhow::anyhow!("recovery codes: RNG failure"))?;
            let chars: String = raw
                .iter()
                .map(|b| ALPHABET[usize::from(b & 31)] as char)
                .collect();
            Ok(chars
                .as_bytes()
                .chunks(4)
                .map(|c| std::str::from_utf8(c).unwrap_or_default())
                .collect::<Vec<_>>()
                .join("-"))
        })
        .collect()
}

/// A typed code reduced to its 16 significant characters: case, spaces and hyphens ignored, and the letters people
/// confuse with digits read as those digits (Crockford). `None` when it cannot be a code.
#[must_use]
pub fn normalize(input: &str) -> Option<String> {
    let mut out = String::with_capacity(CODE_CHARS);
    for c in input.chars() {
        let c = c.to_ascii_lowercase();
        let mapped = match c {
            ' ' | '-' | '\t' => continue,
            'i' | 'l' => '1',
            'o' => '0',
            c if ALPHABET.contains(&(c as u8)) && c.is_ascii() => c,
            _ => return None,
        };
        out.push(mapped);
    }
    (out.len() == CODE_CHARS).then_some(out)
}

/// The stored form of a code: SHA-256 over a domain label, the tenant, the user and the normalized code.
#[must_use]
pub fn hash(tenant_id: &str, user_id: &uuid::Uuid, normalized: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"sesame-idam/recovery-code/v1\0");
    h.update(tenant_id.as_bytes());
    h.update(b"\0");
    h.update(user_id.as_bytes());
    h.update(b"\0");
    h.update(normalized.as_bytes());
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_is_ten_distinct_well_formed_codes() {
        let codes = generate().unwrap();
        assert_eq!(codes.len(), CODES_PER_SET);
        let mut seen = std::collections::HashSet::new();
        for code in &codes {
            assert_eq!(code.len(), CODE_CHARS + 3, "{code}");
            assert_eq!(code.matches('-').count(), 3);
            let n = normalize(code).expect("own codes normalize");
            assert_eq!(n.len(), CODE_CHARS);
            assert!(seen.insert(n));
        }
    }

    #[test]
    fn retyped_codes_still_match() {
        assert_eq!(
            normalize("ABCD-EFGH-JKMN-PQRS").as_deref(),
            Some("abcdefghjkmnpqrs")
        );
        assert_eq!(
            normalize(" abcd efgh jkmn pqrs ").as_deref(),
            Some("abcdefghjkmnpqrs")
        );
        // I/L read as 1, O as 0.
        assert_eq!(
            normalize("ilo0-0000-0000-0000").as_deref(),
            Some("1100000000000000")
        );
    }

    #[test]
    fn rejects_what_cannot_be_a_code() {
        for bad in [
            "",
            "abcd",
            "abcd-efgh-jkmn-pqrs-t",
            "abcd-efgh-jkmn-pqr!",
            "äbcd-efgh-jkmn-pqrs",
            "uuuu-uuuu-uuuu-uuuu",
        ] {
            assert_eq!(normalize(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn hashes_are_bound_to_tenant_and_user() {
        let u = uuid::Uuid::new_v4();
        let h = hash("acme", &u, "abcdefghjkmnpqrs");
        assert_eq!(h.len(), 64);
        assert_eq!(h, hash("acme", &u, "abcdefghjkmnpqrs"));
        assert_ne!(h, hash("other", &u, "abcdefghjkmnpqrs"));
        assert_ne!(h, hash("acme", &uuid::Uuid::new_v4(), "abcdefghjkmnpqrs"));
    }
}
