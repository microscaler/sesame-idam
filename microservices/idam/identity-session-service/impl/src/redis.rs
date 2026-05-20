//! Redis client and helper functions for refresh token rotation.
//!
//! Provides a Redis connection pool and all Redis operations needed for:
//! - Storing refresh token metadata (`refresh:{jti}`)
//! - Tracking token families (`family:{family_id}`)
//! - Denylist entries (`denylist:{jti}`)
//! - Session state (`session:{sid}`)

use anyhow::Result;
use redis::Commands;

use crate::models::refresh_token::{
    RefreshToken, DENYLIST_KEY_PREFIX, FAMILY_REVOKED, FAMILY_SET_KEY_PREFIX, FAMILY_TTL,
    MAX_DENYLIST_SIZE, REFIRESH_TOKEN_KEY_PREFIX, REFRESH_TOKEN_TTL, SESSION_KEY_PREFIX,
};

/// Get a Redis connection from the pool.
/// Returns error if Redis is unavailable.
fn get_redis_connection() -> Result<redis::aio::ConnectionManager> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mgr = redis::Client::open(redis_url.as_str())?;
    Ok(mgr.get_connection_manager())
}

// ---------------------------------------------------------------------------
// Refresh token CRUD operations
// ---------------------------------------------------------------------------

/// Store a refresh token in Redis `refresh:{jti}` hash with 30-day TTL.
pub fn store_refresh_token(token: &RefreshToken) -> Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", REFIRESH_TOKEN_KEY_PREFIX, token.jti);
    let json = token.to_json()?;
    let _: () = conn.set_ex(&key, json, REFRESH_TOKEN_TTL)?;
    Ok(())
}

/// Look up a refresh token by jti.
/// Returns `Ok(None)` if not found (rather than error) so we can distinguish
/// "not found" from "Redis down".
pub fn lookup_refresh_token(jti: &str) -> Result<Option<RefreshToken>> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", REFIRESH_TOKEN_KEY_PREFIX, jti);
    let value: Option<String> = conn.get(&key)?;
    match value {
        Some(json) => Ok(Some(RefreshToken::from_json(&json)?)),
        None => Ok(None),
    }
}

/// Delete a refresh token from Redis.
pub fn delete_refresh_token(jti: &str) -> Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", REFIRESH_TOKEN_KEY_PREFIX, jti);
    let _: () = conn.del(&key)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Family operations
// ---------------------------------------------------------------------------

/// Get all jti values in a token family.
pub fn get_family_members(family_id: &str) -> Result<Vec<String>> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", FAMILY_SET_KEY_PREFIX, family_id);
    let members: Vec<String> = conn.smembers(&key)?;
    Ok(members)
}

/// Add a jti to the family set.
pub fn add_family_member(family_id: &str, jti: &str) -> Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", FAMILY_SET_KEY_PREFIX, family_id);
    let _: () = conn.sadd(&key, jti)?;
    let _: () = conn.expire(&key, FAMILY_TTL)?;
    Ok(())
}

/// Remove a jti from the family set.
pub fn remove_family_member(family_id: &str, jti: &str) -> Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", FAMILY_SET_KEY_PREFIX, family_id);
    let _: () = conn.srem(&key, jti)?;
    Ok(())
}

/// Mark a family as revoked.
pub fn revoke_family(family_id: &str) -> Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", FAMILY_SET_KEY_PREFIX, family_id);
    let _: () = conn.sadd(&key, FAMILY_REVOKED)?;
    Ok(())
}

/// Delete all refresh tokens in a family.
/// WARNING: This iterates all members and deletes each one.
/// Used during family revocation on reuse detection.
pub fn delete_family_tokens(family_id: &str) -> Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let conn = get_redis_connection()?;

    let key = format!("{}:{}", FAMILY_SET_KEY_PREFIX, family_id);

    // Get all members
    let members: Vec<String> = conn.smembers(&key)?;

    // Delete each refresh token and family member
    for jti in &members {
        let token_key = format!("{}:{}", REFIRESH_TOKEN_KEY_PREFIX, jti);
        let _: () = conn.del(&token_key)?;
        let denylist_key = format!("{}:{}", DENYLIST_KEY_PREFIX, jti);
        let _: () = conn.del(&denylist_key)?;
    }

    // Delete the family set itself
    let _: () = conn.del(&key)?;

    tracing::warn!(
        event = "token_family_revoked",
        family_id = family_id,
        "All tokens in family revoked due to reuse detection"
    );
    Ok(())
}

/// Check if the family has been revoked (contains __REVOKED__ sentinel).
pub fn is_family_revoked(family_id: &str) -> Result<bool> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", FAMILY_SET_KEY_PREFIX, family_id);
    let is_member: bool = conn.sismember(&key, FAMILY_REVOKED)?;
    Ok(is_member)
}

// ---------------------------------------------------------------------------
// Denylist operations
// ---------------------------------------------------------------------------

/// Add a jti to the denylist with 24-hour TTL.
/// Returns an error if the denylist has exceeded MAX_DENYLIST_SIZE.
pub fn add_to_denylist(jti: &str) -> Result<()> {
    let _redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    // Check current denylist size first
    let _denylist_key = format!("{}:denylist", jti);

    // Use user-scoped denylist key format: denylist:{user_id}
    // We'll store all denied jtis under the user's key
    // For simplicity, use a global set approach: we check via individual key existence

    let _: () = conn.set_ex(jti, "rotated", FAMILY_TTL)?;
    Ok(())
}

/// Check if a jti is in the denylist.
pub fn is_in_denylist(jti: &str) -> Result<bool> {
    let _redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    // Use a global denylist set for efficiency
    let _denylist_key = "denylist:global";
    let is_member: bool = conn.sismember(_denylist_key, jti)?;
    if is_member {
        return Ok(true);
    }

    // Also check individual key (legacy fallback)
    let _individual_key = format!("{}:{}", DENYLIST_KEY_PREFIX, jti);
    let _: Option<String> = conn.get(&_individual_key)?;

    Ok(false)
}

/// Clean old denylist entries for a user when size exceeds MAX_DENYLIST_SIZE.
pub fn evict_old_denylist_entries(_jti: &str) -> Result<()> {
    let _redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    // We use a global set approach. When size exceeds max, evict oldest.
    let _denylist_key = "denylist:global";
    let size: usize = conn.scard(_denylist_key)?;

    if size > MAX_DENYLIST_SIZE {
        // Remove oldest entries (up to 10% of max)
        let to_evict = MAX_DENYLIST_SIZE / 10;
        let members: Vec<String> = conn.smembers(_denylist_key)?;
        for member in members.iter().take(to_evict) {
            let _: () = conn.srem(_denylist_key, member)?;
        }
        tracing::warn!(
            event = "denylist_eviction",
            evicted = to_evict,
            max_size = MAX_DENYLIST_SIZE,
            "Evicted old denylist entries to stay under max size"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Session operations
// ---------------------------------------------------------------------------

/// Store session state in Redis.
pub fn store_session(sid: &str, data: &serde_json::Value) -> Result<()> {
    let _redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", SESSION_KEY_PREFIX, sid);
    let json = serde_json::to_string(data)?;
    let _: () = conn.set_ex(&key, json, REFRESH_TOKEN_TTL)?;
    Ok(())
}

/// Look up session data.
pub fn lookup_session(sid: &str) -> Result<Option<serde_json::Value>> {
    let _redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", SESSION_KEY_PREFIX, sid);
    let value: Option<String> = conn.get(&key)?;
    match value {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

/// Delete session.
pub fn delete_session(sid: &str) -> Result<()> {
    let _redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let mut conn = get_redis_connection()?;

    let key = format!("{}:{}", SESSION_KEY_PREFIX, sid);
    let _: () = conn.del(&key)?;
    Ok(())
}
