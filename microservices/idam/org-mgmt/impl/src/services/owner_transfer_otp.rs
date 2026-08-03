//! Email OTP gate for product-path Owner transfer.
//!
//! Codes are hashed in Redis (never stored cleartext), bound to
//! `tenant + org + caller`, TTL'd, attempt-capped, and single-use.
//! Same threat model as login email OTP: walk-away console abuse needs a
//! secret that lands outside the unlocked browser.

use anyhow::{Context, Result};
use rand::Rng;
use redis::Commands;
use sha2::{Digest, Sha256};

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// OTP lifetime in seconds (also returned to challenge callers).
#[must_use]
pub fn ttl_secs() -> u64 {
    env_u64("OWNER_TRANSFER_OTP_TTL_SECS", 300)
}

fn max_attempts() -> u64 {
    env_u64("OWNER_TRANSFER_OTP_MAX_ATTEMPTS", 5)
}

fn connection() -> Result<redis::Connection> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let client = redis::Client::open(redis_url.as_str())?;
    Ok(client.get_connection()?)
}

fn sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut hex = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Constant-shape comparison of two equal-length hex digests.
fn digests_match(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

fn otp_key(tenant: &str, org_id: &str, user_id: &str) -> String {
    format!(
        "otp:owner_transfer:{}:{}:{}",
        tenant.trim(),
        org_id.trim(),
        user_id.trim()
    )
}

fn attempts_key(tenant: &str, org_id: &str, user_id: &str) -> String {
    format!(
        "otp:owner_transfer:attempts:{}:{}:{}",
        tenant.trim(),
        org_id.trim(),
        user_id.trim()
    )
}

/// Mint + store a 6-digit OTP for the Owner transfer challenge.
///
/// # Errors
///
/// Returns when Redis is unavailable.
pub fn create(tenant: &str, org_id: &str, user_id: &str) -> Result<String> {
    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let mut conn = connection().context("owner_transfer_otp: redis")?;
    let ttl = ttl_secs();
    conn.set_ex::<_, _, ()>(otp_key(tenant, org_id, user_id), sha256_hex(&code), ttl)
        .context("owner_transfer_otp: store")?;
    let _: Result<(), _> = conn.del(attempts_key(tenant, org_id, user_id));
    Ok(code)
}

/// Verify and consume the OTP. Returns `true` only on exact match within budget.
#[must_use]
pub fn verify_and_consume(tenant: &str, org_id: &str, user_id: &str, code: &str) -> bool {
    let code = code.trim();
    if code.is_empty() {
        return false;
    }
    let mut conn = match connection() {
        Ok(c) => c,
        Err(_) => return false,
    };

    let ak = attempts_key(tenant, org_id, user_id);
    let attempts: u64 = match conn.incr(&ak, 1u64) {
        Ok(n) => n,
        Err(_) => return false,
    };
    let _: Result<(), _> = conn.expire(&ak, i64::try_from(ttl_secs()).unwrap_or(300));
    if attempts > max_attempts() {
        let _: Result<(), _> = conn.del(otp_key(tenant, org_id, user_id));
        return false;
    }

    let stored: String = match conn.get(otp_key(tenant, org_id, user_id)) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if !digests_match(&stored, &sha256_hex(code)) {
        return false;
    }
    let _: Result<(), _> = conn.del(otp_key(tenant, org_id, user_id));
    let _: Result<(), _> = conn.del(&ak);
    true
}

#[cfg(test)]
mod tests {
    use super::{create, digests_match, sha256_hex, verify_and_consume};

    #[test]
    fn digests_match_positive_and_negative() {
        let a = sha256_hex("123456");
        assert!(digests_match(&a, &sha256_hex("123456")));
        assert!(!digests_match(&a, &sha256_hex("000000")));
        assert!(!digests_match(&a, "short"));
    }

    #[test]
    fn redis_round_trip_when_available() {
        let Ok(_) = super::connection() else {
            println!("SKIP: Redis not available");
            return;
        };
        // Unique keys: lib + bin test targets may run this in parallel.
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tenant = format!("otp-test-tenant-{nonce}");
        let org = format!("a1000001-0001-4000-8000-{nonce:012x}");
        let user = format!("a1000001-0001-4000-8001-{nonce:012x}");
        let code = create(&tenant, &org, &user).expect("mint");
        assert!(!verify_and_consume(&tenant, &org, &user, "000000"));
        assert!(verify_and_consume(&tenant, &org, &user, &code));
        assert!(!verify_and_consume(&tenant, &org, &user, &code));
    }
}
