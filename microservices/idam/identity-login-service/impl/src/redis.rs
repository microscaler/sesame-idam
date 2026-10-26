//! Redis helpers for refresh-token state at login/register and logout time.
//!
//! Sessions are identity-session-service's `rfam:{family_id}` records (`services::refresh_store`), so tokens issued
//! here are rotated by `POST /session/refresh` there.

use anyhow::Result;
use redis::Commands;

use crate::models::refresh_token::RefreshToken;

/// A Redis connection from the process-wide pool (`sesame_common::redis_pool`): reused, not opened per call.
fn get_redis_connection() -> Result<sesame_common::redis_pool::PooledConnection<'static>> {
    Ok(sesame_common::redis_pool::pooled_connection()?)
}

/// Start the session of a freshly issued refresh token: its `rfam:{family_id}` record with this token current
/// (`sesame_common::refresh_session`, the layout identity-session-service rotates; one atomic round trip). It lives for
/// the idle window, renewed on every rotation, and never past `absolute_expiry`.
///
/// # Errors
///
/// Returns an error if Redis is unavailable or the write fails.
pub fn store_refresh_token(token: &RefreshToken, absolute_expiry: i64) -> Result<()> {
    use sesame_common::refresh_session::{create, Policy, SessionToken};

    let session = SessionToken {
        jti: token.jti.clone(),
        sub: token.sub.clone(),
        sid: token.sid.clone(),
        family_id: token.family_id.clone(),
        iat: token.iat,
        exp: token.exp,
        client_id: token.client_id.clone(),
        tenant_id: token.tenant_id.clone(),
        scopes: token.scopes.clone(),
    };
    create(&session, absolute_expiry, token.iat, Policy::from_env())?;
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

/// Revoke a refresh token presented at logout: the whole session ends (its `rfam:` record, and for tokens issued
/// before session records, the old per-token records). The token's signature is verified first, so only a holder of
/// one of the session's tokens can end it; a malformed or forged token is ignored (idempotent logout).
///
/// # Errors
///
/// Returns an error when Redis is unavailable.
pub fn revoke_refresh_token(refresh_token_value: &str) -> Result<()> {
    sesame_idam_identity_session_service::services::token_rotation::revoke_session_of(
        refresh_token_value,
    )
}
