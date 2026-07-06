//! Password hashing and verification (argon2id).
//!
//! Hashes are stored in PHC string format (`$argon2id$v=19$...`) in
//! `users.password_hash`. Verification is constant-time via the argon2 crate.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

/// Minimum accepted password length (F: signup validation).
pub const MIN_PASSWORD_LENGTH: usize = 8;

/// Validate password strength. Returns a user-facing error message on failure.
pub fn validate_password_strength(password: &str) -> Result<(), &'static str> {
    if password.chars().count() < MIN_PASSWORD_LENGTH {
        return Err("password must be at least 8 characters");
    }
    Ok(())
}

/// Hash a password with argon2id and a fresh random salt.
///
/// # Errors
///
/// Returns an error string if hashing fails (should not happen with valid
/// parameters).
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("password hashing failed: {e}"))
}

/// Verify a password against a stored PHC-format hash.
///
/// Returns `false` for malformed hashes rather than erroring — a corrupt
/// hash must never allow a login.
#[must_use]
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_round_trip() {
        let hash = hash_password("SecureP@ss123!").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("SecureP@ss123!", &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn same_password_different_salts() {
        let h1 = hash_password("SecureP@ss123!").unwrap();
        let h2 = hash_password("SecureP@ss123!").unwrap();
        assert_ne!(h1, h2, "salts must differ per hash");
    }

    #[test]
    fn malformed_hash_never_verifies() {
        assert!(!verify_password("anything", "not-a-phc-hash"));
        assert!(!verify_password("anything", ""));
    }

    #[test]
    fn strength_validation() {
        assert!(validate_password_strength("short").is_err());
        assert!(validate_password_strength("longenough").is_ok());
    }
}
