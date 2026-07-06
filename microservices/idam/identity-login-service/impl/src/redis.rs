//! Redis helpers for seeding refresh-token state at login/register time.
//!
//! Key layout matches identity-session-service's rotation machinery
//! (`refresh:{jti}`, `family:{family_id}`) so tokens issued here can be
//! rotated by `POST /auth/refresh` on the session service.

use anyhow::Result;
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
