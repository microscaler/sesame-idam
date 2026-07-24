//! One-time authorization codes for cross-origin session handoff (ADR-010).
//!
//! # Why this exists
//!
//! The hosted auth surface (`sesame-auth…`) and a tenant's app
//! (`app.tenant.com`) are DIFFERENT origins. Browser storage is
//! origin-scoped, so the app cannot read the tokens the auth surface just
//! obtained. Something has to cross the boundary — and it must not be the
//! tokens themselves, because the only cross-origin channel is a URL, and
//! URLs leak through history, `Referer` headers, proxy logs and screen
//! shares.
//!
//! So we hand over a **one-time code**: high-entropy, short-lived
//! (60s default), single-use, and **bound to the `redirect_uri`** it was
//! minted for. A stolen code is worthless at a different destination, and
//! worthless twice.
//!
//! Only the code's SHA-256 hash is used as the Redis key, matching the
//! magic-link / reset-token pattern: a Redis dump never yields usable codes.
//!
//! Env:
//! - `AUTH_CODE_TTL_SECS` (60) — deliberately short; the redirect is instant.

use anyhow::{Context, Result};
use base64::Engine;
use rand::Rng;
use redis::Commands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The session material parked against a code.
#[derive(Debug, Serialize, Deserialize)]
pub struct CodePayload {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// The redirect_uri this code is bound to — checked on redemption.
    pub redirect_uri: String,
    pub tenant_id: String,
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

fn code_key(code: &str) -> String {
    format!("authcode:{}", sha256_hex(code))
}

/// TTL for a freshly minted code.
#[must_use]
pub fn ttl_secs() -> u64 {
    std::env::var("AUTH_CODE_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60)
}

/// Mint a one-time code for the given session material.
///
/// # Errors
///
/// Returns an error when Redis is unavailable or the payload fails to
/// serialize.
pub fn mint(payload: &CodePayload) -> Result<String> {
    let mut raw = [0u8; 32];
    rand::thread_rng().fill(&mut raw);
    let code = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);

    let json = serde_json::to_string(payload).context("authcode: serialize")?;
    let mut conn = connection().context("authcode: redis")?;
    conn.set_ex::<_, _, ()>(code_key(&code), json, ttl_secs())
        .context("authcode: store")?;
    Ok(code)
}

/// Redeem a code (single use) and return its payload, but ONLY if the
/// presented `redirect_uri` matches the one it was minted for.
///
/// The code is burned on ANY redemption attempt that finds it — including a
/// redirect_uri mismatch — so a leaked code cannot be probed against
/// candidate destinations.
#[must_use]
pub fn redeem(code: &str, redirect_uri: &str, tenant_id: &str) -> Option<CodePayload> {
    let mut conn = connection().ok()?;
    // GETDEL: atomically fetch-and-burn.
    let json: Option<String> = redis::cmd("GETDEL")
        .arg(code_key(code))
        .query(&mut conn)
        .ok()?;
    let payload: CodePayload = serde_json::from_str(&json?).ok()?;

    if payload.redirect_uri != redirect_uri {
        tracing::warn!("authcode: redirect_uri mismatch on redemption — code burned");
        return None;
    }
    if payload.tenant_id != tenant_id {
        tracing::warn!("authcode: tenant mismatch on redemption — code burned");
        return None;
    }
    Some(payload)
}
