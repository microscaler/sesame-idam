//! OTP + magic-link issuance and verification (Redis-backed).
//!
//! Secrets never touch Redis in the clear: only SHA-256 hashes of codes and
//! magic-link tokens are stored, keyed by the SHA-256-hashed recipient (same
//! no-PII-in-keys rule as `abuse_guard`). Both artefacts are single-use —
//! consumption is a GETDEL — and attempt-capped so a stored code cannot be
//! brute-forced within its TTL.
//!
//! Env (defaults in parentheses):
//! - `OTP_TTL_SECS` (300)            code lifetime
//! - `OTP_MAX_ATTEMPTS` (5)          verify attempts per issued code
//! - `MAGIC_LINK_TTL_SECS` (600)     link lifetime
//! - `MAGIC_LINK_BASE_URL` (`http://localhost:8080/auth/verify-magic`)
//!   base URL embedded in the emailed link (`?tenant=…&token=…`)

use anyhow::{Context, Result};
use base64::Engine;
use rand::Rng;
use redis::Commands;
use sha2::{Digest, Sha256};

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
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

fn ident_hash(identifier: &str) -> String {
    sha256_hex(&identifier.trim().to_lowercase())[..32].to_string()
}

/// Constant-shape comparison of two equal-length hex digests.
fn digests_match(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

// ── email OTP ───────────────────────────────────────────────────────────────

fn otp_key(channel: &str, tenant: &str, recipient: &str) -> String {
    format!("otp:{channel}:{tenant}:{}", ident_hash(recipient))
}

fn otp_attempts_key(channel: &str, tenant: &str, recipient: &str) -> String {
    format!("otp:attempts:{channel}:{tenant}:{}", ident_hash(recipient))
}

/// Generate + store a 6-digit OTP for a KNOWN user on a given channel
/// (`email` | `sms`). Returns the code for delivery. Re-issuing overwrites
/// any previous code (last-code-wins) and resets the attempt budget.
///
/// # Errors
///
/// Returns an error when Redis is unavailable — the caller keeps the HTTP
/// response generic and skips the send.
pub fn create_otp(channel: &str, tenant: &str, recipient: &str, user_id: &str) -> Result<String> {
    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let mut conn = connection().context("otp: redis")?;
    let ttl = env_u64("OTP_TTL_SECS", 300);
    let value = format!("{}:{user_id}", sha256_hex(&code));
    conn.set_ex::<_, _, ()>(otp_key(channel, tenant, recipient), value, ttl)
        .context("otp: store")?;
    let _: Result<(), _> = conn.del(otp_attempts_key(channel, tenant, recipient));
    Ok(code)
}

/// Verify an OTP on a given channel. Single-use; attempt-capped. Returns the
/// `user_id` bound at issuance on success, `None` on any failure (missing,
/// expired, wrong code, attempts exhausted, Redis down) — callers must map
/// every `None` to the same generic error.
#[must_use]
pub fn verify_otp(channel: &str, tenant: &str, recipient: &str, code: &str) -> Option<String> {
    let mut conn = connection().ok()?;

    // Attempt budget first: exhausting it burns the stored code.
    let ak = otp_attempts_key(channel, tenant, recipient);
    let attempts: u64 = conn.incr(&ak, 1u64).ok()?;
    let _: Result<(), _> = conn.expire(&ak, i64::try_from(env_u64("OTP_TTL_SECS", 300)).unwrap_or(300));
    if attempts > env_u64("OTP_MAX_ATTEMPTS", 5) {
        let _: Result<(), _> = conn.del(otp_key(channel, tenant, recipient));
        return None;
    }

    let stored: String = conn.get(otp_key(channel, tenant, recipient)).ok()?;
    let (stored_hash, user_id) = stored.split_once(':')?;
    if !digests_match(stored_hash, &sha256_hex(code)) {
        return None;
    }
    // Single use.
    let _: Result<(), _> = conn.del(otp_key(channel, tenant, recipient));
    let _: Result<(), _> = conn.del(&ak);
    Some(user_id.to_string())
}

/// Email-channel OTP issuance (thin wrapper over [`create_otp`]).
///
/// # Errors
///
/// Returns an error when Redis is unavailable.
pub fn create_email_otp(tenant: &str, email: &str, user_id: &str) -> Result<String> {
    create_otp("email", tenant, email, user_id)
}

/// Email-channel OTP verification (thin wrapper over [`verify_otp`]).
#[must_use]
pub fn verify_email_otp(tenant: &str, email: &str, code: &str) -> Option<String> {
    verify_otp("email", tenant, email, code)
}

/// SMS-channel OTP issuance (thin wrapper over [`create_otp`]).
///
/// # Errors
///
/// Returns an error when Redis is unavailable.
pub fn create_phone_otp(tenant: &str, phone: &str, user_id: &str) -> Result<String> {
    create_otp("sms", tenant, phone, user_id)
}

/// SMS-channel OTP verification (thin wrapper over [`verify_otp`]).
#[must_use]
pub fn verify_phone_otp(tenant: &str, phone: &str, code: &str) -> Option<String> {
    verify_otp("sms", tenant, phone, code)
}

// ── magic link ──────────────────────────────────────────────────────────────

fn magic_key(tenant: &str, token_hash: &str) -> String {
    format!("magiclink:{tenant}:{token_hash}")
}

/// Mint + store a single-use magic-link token for a KNOWN user. Returns the
/// full clickable URL for delivery.
///
/// # Errors
///
/// Returns an error when Redis is unavailable.
pub fn create_magic_link(tenant: &str, user_id: &str) -> Result<String> {
    let mut raw = [0u8; 32];
    rand::thread_rng().fill(&mut raw);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);

    let mut conn = connection().context("magiclink: redis")?;
    let ttl = env_u64("MAGIC_LINK_TTL_SECS", 600);
    conn.set_ex::<_, _, ()>(magic_key(tenant, &sha256_hex(&token)), user_id, ttl)
        .context("magiclink: store")?;

    let base = std::env::var("MAGIC_LINK_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080/auth/verify-magic".to_string());
    Ok(format!("{base}?tenant={tenant}&token={token}"))
}

// ── password reset ──────────────────────────────────────────────────────────

fn reset_key(tenant: &str, token_hash: &str) -> String {
    format!("pwreset:{tenant}:{token_hash}")
}

/// Mint + store a single-use password-reset token for a KNOWN user.
///
/// Same shape as a magic link (256-bit, only the hash stored, TTL'd,
/// consumed atomically) but a SEPARATE keyspace: a reset token must never be
/// usable as a login link, and vice versa.
///
/// # Errors
///
/// Returns an error when Redis is unavailable.
pub fn create_password_reset(tenant: &str, user_id: &str) -> Result<String> {
    let mut raw = [0u8; 32];
    rand::thread_rng().fill(&mut raw);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);

    let mut conn = connection().context("pwreset: redis")?;
    let ttl = env_u64("PASSWORD_RESET_TTL_SECS", 900);
    conn.set_ex::<_, _, ()>(reset_key(tenant, &sha256_hex(&token)), user_id, ttl)
        .context("pwreset: store")?;
    Ok(token)
}

/// Build the clickable reset URL delivered to the user.
#[must_use]
pub fn password_reset_url(tenant: &str, token: &str) -> String {
    let base = std::env::var("PASSWORD_RESET_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080/reset-password".to_string());
    format!("{base}?tenant={tenant}&token={token}")
}

/// Consume a password-reset token (single use). Returns the bound `user_id`,
/// or `None` for unknown/expired/reused tokens.
#[must_use]
pub fn consume_password_reset(tenant: &str, token: &str) -> Option<String> {
    let mut conn = connection().ok()?;
    let user_id: Option<String> = redis::cmd("GETDEL")
        .arg(reset_key(tenant, &sha256_hex(token)))
        .query(&mut conn)
        .ok()?;
    user_id
}

/// Consume a magic-link token (single use). Returns the bound `user_id`, or
/// `None` for unknown/expired/reused tokens.
#[must_use]
pub fn consume_magic_link(tenant: &str, token: &str) -> Option<String> {
    let mut conn = connection().ok()?;
    // GETDEL: atomically fetch-and-burn (reuse returns None).
    let user_id: Option<String> = redis::cmd("GETDEL")
        .arg(magic_key(tenant, &sha256_hex(token)))
        .query(&mut conn)
        .ok()?;
    user_id
}
