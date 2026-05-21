/// HMAC signing for tamper-evident audit log entries.
///
/// Each audit log entry can be signed with an HMAC-SHA256 key so that
/// downstream consumers can verify the entry has not been tampered with.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Signs an audit log entry.
///
/// The signature covers the canonical log JSON (fields in order) plus the
/// event timestamp. This allows verification without needing to know the
/// signing key at query time.
pub fn sign_entry(key: &[u8], log_json: &str, timestamp: &str) -> String {
    let message = format!("{}:{}", log_json, timestamp);
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Verifies an HMAC signature on an audit log entry.
///
/// Returns true if the signature is valid.
pub fn verify_entry(key: &[u8], log_json: &str, timestamp: &str, signature: &str) -> bool {
    let expected = sign_entry(key, log_json, timestamp);
    // Constant-time comparison to prevent timing attacks
    let expected_bytes = hex::decode(&expected).unwrap_or_default();
    let input_bytes = hex::decode(signature).ok().unwrap_or_default();
    if expected_bytes.len() != input_bytes.len() {
        return false;
    }
    hmac::mac::generic_array::GenericArray::from_slice(&expected_bytes)
        .const_eq(&hmac::mac::generic_array::GenericArray::from_slice(&input_bytes))
}

/// Generates a random HMAC key (32 bytes / 256 bits).
pub fn generate_key() -> Vec<u8> {
    use rand::{thread_rng, Rng};
    let mut key = vec![0u8; 32];
    thread_rng().fill(&mut key[..]);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let key = generate_key();
        let log_json = r#"{"event":"jwt_issued","service":"test"}"#;
        let timestamp = "2026-01-01T00:00:00Z";

        let sig = sign_entry(&key, log_json, timestamp);
        assert_eq!(sig.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
        assert!(verify_entry(&key, log_json, timestamp, &sig));
    }

    #[test]
    fn test_verify_fails_with_wrong_key() {
        let key1 = generate_key();
        let key2 = generate_key();
        let log_json = r#"{"event":"jwt_issued"}"#;
        let timestamp = "2026-01-01T00:00:00Z";

        let sig = sign_entry(&key1, log_json, timestamp);
        assert!(!verify_entry(&key2, log_json, timestamp, &sig));
    }

    #[test]
    fn test_verify_fails_with_tampered_json() {
        let key = generate_key();
        let log_json = r#"{"event":"jwt_issued"}"#;
        let tampered = r#"{"event":"jwt_issued","tampered":true}"#;
        let timestamp = "2026-01-01T00:00:00Z";

        let sig = sign_entry(&key, log_json, timestamp);
        assert!(!verify_entry(&key, tampered, timestamp, &sig));
    }

    #[test]
    fn test_generate_key_length() {
        let key = generate_key();
        assert_eq!(key.len(), 32);
    }
}
