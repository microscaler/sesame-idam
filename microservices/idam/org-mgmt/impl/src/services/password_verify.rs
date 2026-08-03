//! Local argon2id password verification for sensitive org actions.
//!
//! Mirrors identity-login-service hashing so org-mgmt can re-auth the caller
//! without a cross-service call on the Owner-transfer path.

use argon2::password_hash::{PasswordHash, PasswordVerifier};
use argon2::Argon2;
use lifeguard::LifeExecutor;
use uuid::Uuid;

/// Verify a password against a stored PHC-format hash.
#[must_use]
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Load `users.password_hash` for tenant-scoped user, or `None` if unavailable.
pub fn lookup_password_hash<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    user_id: Uuid,
) -> Option<String> {
    let values = sea_query::Values(vec![
        sea_query::Value::Uuid(Some(user_id)),
        sea_query::Value::String(Some(tenant_id.to_string())),
    ]);
    match exec.query_one_values(
        "SELECT password_hash FROM sesame_idam.users WHERE id = $1 AND tenant_id = $2",
        &values,
    ) {
        Ok(row) => row.try_get::<usize, String>(0).ok().filter(|h| !h.is_empty()),
        Err(error) => {
            tracing::warn!(%error, %user_id, tenant_id = %tenant_id, "password_hash lookup failed");
            None
        }
    }
}

/// True when the supplied password matches the caller's stored hash.
#[must_use]
pub fn verify_caller_password<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    user_id: Uuid,
    password: &str,
) -> bool {
    let password = password.trim();
    if password.is_empty() {
        return false;
    }
    lookup_password_hash(exec, tenant_id, user_id)
        .is_some_and(|hash| verify_password(password, &hash))
}

#[cfg(test)]
mod tests {
    use super::verify_password;
    use argon2::password_hash::rand_core::OsRng;
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::Argon2;

    #[test]
    fn verify_password_round_trip() {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(b"SecureP@ss123!", &salt)
            .unwrap()
            .to_string();
        assert!(verify_password("SecureP@ss123!", &hash));
        assert!(!verify_password("wrong", &hash));
        assert!(!verify_password("SecureP@ss123!", "not-a-hash"));
    }
}
