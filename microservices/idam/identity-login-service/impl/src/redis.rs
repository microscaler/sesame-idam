//! Redis helpers for seeding refresh-token state at login/register time.
//!
//! Key layout matches identity-session-service's rotation machinery
//! (`refresh:{jti}`, `family:{family_id}`) so tokens issued here can be
//! rotated by `POST /auth/refresh` on the session service.

use anyhow::Result;
use base64::Engine;
use redis::Commands;

use crate::models::refresh_token::{
    RefreshToken, FAMILY_SET_KEY_PREFIX, FAMILY_TTL, REFRESH_TOKEN_KEY_PREFIX, REFRESH_TOKEN_TTL,
};

fn get_redis_connection() -> Result<redis::Connection> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let client = redis::Client::open(redis_url.as_str())?;
    Ok(client.get_connection()?)
}

/// Store refresh-token metadata (`refresh:{jti}`, 30d TTL) and register the
/// jti in its family set (`family:{family_id}`, 24h TTL).
///
/// # Errors
///
/// Returns an error if Redis is unavailable or the write fails.
pub fn store_refresh_token(token: &RefreshToken) -> Result<()> {
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", REFRESH_TOKEN_KEY_PREFIX, token.jti);
    let json = token.to_json()?;
    let _: () = conn.set_ex(&key, json, u64::from(REFRESH_TOKEN_TTL))?;

    let family_key = format!("{}:{}", FAMILY_SET_KEY_PREFIX, token.family_id);
    let _: () = conn.sadd(&family_key, &token.jti)?;
    let _: () = conn.expire(&family_key, i64::from(FAMILY_TTL))?;

    Ok(())
}

/// Add an access-token `jti` to the denylist (`denylist:{jti}`) with a TTL that
/// matches the token's remaining lifetime.
///
/// This is the write half of access-token revocation: a logged-out access token
/// is recorded as revoked so `SesameTokenStatusChecker` (using the same `denylist:` key
/// scheme) rejects it until it would have expired anyway.
/// No-op for an empty jti or a zero/elapsed TTL.
///
/// # Errors
///
/// Returns an error when Redis is unavailable or the write fails.
pub fn deny_access_jti(jti: &str, ttl_secs: u64) -> Result<()> {
    if jti.is_empty() || ttl_secs == 0 {
        return Ok(());
    }
    let mut conn = get_redis_connection()?;
    let key = format!("denylist:{jti}");
    let _: () = conn.set_ex(&key, "revoked", ttl_secs)?;
    Ok(())
}

/// Decode the `jti` claim from a signed refresh token JWT.
fn decode_refresh_token_jti(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let bytes = engine.decode(parts[1]).ok()?;
    let decoded = String::from_utf8(bytes).ok()?;
    let value: serde_json::Value = serde_json::from_str(&decoded).ok()?;
    value
        .get("jti")?
        .as_str()
        .map(std::string::ToString::to_string)
}

/// Revoke a refresh token presented at logout.
///
/// Deletes the token metadata, adds the JTI to the denylist, and revokes the
/// entire token family so outstanding siblings cannot be rotated.
///
/// # Errors
///
/// Returns an error when Redis is unavailable. Missing/ malformed tokens are
/// treated as success (idempotent logout).
pub fn revoke_refresh_token(refresh_token_value: &str) -> Result<()> {
    let Some(jti) = decode_refresh_token_jti(refresh_token_value) else {
        return Ok(());
    };

    let mut conn = get_redis_connection()?;
    let key = format!("{}:{}", REFRESH_TOKEN_KEY_PREFIX, jti);
    let stored: Option<String> = conn.get(&key)?;

    let family_id = stored
        .as_ref()
        .and_then(|json| RefreshToken::from_json(json).ok())
        .map(|t| t.family_id);

    let _: () = conn.del(&key)?;

    let denylist_key = format!("denylist:{jti}");
    let _: () = conn.set_ex(&denylist_key, "revoked", u64::from(FAMILY_TTL))?;

    if let Some(family_id) = family_id {
        let family_key = format!("{}:{}", FAMILY_SET_KEY_PREFIX, family_id);
        let members: Vec<String> = conn.smembers(&family_key)?;
        for member in members {
            if member == "__REVOKED__" {
                continue;
            }
            let member_key = format!("{}:{}", REFRESH_TOKEN_KEY_PREFIX, member);
            let _: () = conn.del(&member_key)?;
            let member_denylist = format!("denylist:{member}");
            let _: () = conn.set_ex(&member_denylist, "revoked", u64::from(FAMILY_TTL))?;
        }
        let _: () = conn.del(&family_key)?;
    }

    Ok(())
}
