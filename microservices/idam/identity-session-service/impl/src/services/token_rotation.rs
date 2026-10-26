//! Token rotation service — the core business logic for refresh token rotation.
//!
//! Implements the rotation flow per Story 3.1:
//!   1. Validate the refresh token exists in Redis
//!   2. Check denylist for reuse detection
//!   3. If reused → revoke entire family → 401
//!   4. If clean → rotate → issue new tokens
//!
//! Also provides metrics counters for telemetry.

use std::sync::LazyLock;

use anyhow::Result;
use prometheus::{register_int_counter, register_int_counter_vec, IntCounter, IntCounterVec};
use uuid::Uuid;

use crate::models::refresh_token::RefreshToken;
use crate::redis;

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

pub static TOKEN_REFRESH_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "token_refresh_total",
        "Total number of /auth/refresh requests",
        &["result", "subreason"]
    )
    .unwrap()
});

pub static REFRESH_REUSE_DETECTED_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!(
        "refresh_reuse_detected_total",
        "Total number of refresh token reuse detections (family revocations)",
    )
    .unwrap()
});

pub static REFRESH_ROTATION_FAILURES_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!(
        "refresh_rotation_failures_total",
        "Total number of refresh rotation failures",
    )
    .unwrap()
});

// ---------------------------------------------------------------------------
// Rotation result types
// ---------------------------------------------------------------------------

/// Result of a rotation attempt.
#[derive(Debug, Clone)]
pub enum RotationOutcome {
    /// Normal rotation — old token invalidated, new tokens issued.
    Rotated {
        new_access_token: String,
        new_refresh_token: String,
        access_expires_in: i32,
        refresh_expires_in: i32,
        user_id: String,
        scope: String,
    },
    /// Reuse detected — entire token family revoked.
    ReuseDetected {
        /// The JTI that triggered reuse detection
        reused_jti: String,
        /// The family that was revoked
        family_id: String,
    },
    /// Invalid token — not found in Redis.
    InvalidToken,
    /// Redis unavailable — rotation cannot proceed.
    RedisUnavailable,
}

/// Token rotation error types for structured handling.
#[derive(Debug, Clone)]
pub enum RotationError {
    /// Token not found in Redis (expired, malformed, or already rotated).
    TokenNotFound,
    /// Reuse detected — token was already used, family revoked.
    TokenReuseDetected(String, String),
    /// Redis connection or operation failed.
    RedisError(String),
}

impl From<anyhow::Error> for RotationError {
    fn from(e: anyhow::Error) -> Self {
        RotationError::RedisError(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Account state (refresh is refused for an account that is no longer active)
// ---------------------------------------------------------------------------

/// Whether the refresh token's subject may still be issued tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Active,
    /// Disabled, deleted, or not a user of this tenant.
    Inactive,
    /// The database could not say.
    Unknown,
}

/// `users.status` is `active` for a usable account; anything else (or no row) is not.
#[must_use]
pub fn account_state_of(status: Option<&str>) -> AccountState {
    match status {
        Some(s) if s == "active" => AccountState::Active,
        _ => AccountState::Inactive,
    }
}

fn account_state(tenant_id: &str, sub: &str) -> AccountState {
    let Ok(user_id) = sub.parse::<Uuid>() else {
        return AccountState::Inactive;
    };
    match crate::tenant_db::load_profile(tenant_id, user_id) {
        Ok(crate::tenant_db::ProfileLoad::Found(user, _)) => {
            account_state_of(Some(user.status.as_str()))
        }
        Ok(crate::tenant_db::ProfileLoad::NotFound) => AccountState::Inactive,
        Err(e) => {
            tracing::error!(error = %e, "refresh: account state lookup failed");
            AccountState::Unknown
        }
    }
}

// ---------------------------------------------------------------------------
// Core rotation logic
// ---------------------------------------------------------------------------

/// Rotate a refresh token.
///
/// Validates the old token, checks for reuse, and issues new access + refresh
/// tokens signed with the shared Ed25519 key.
pub fn rotate_refresh_token(refresh_token_value: &str, tenant_id: &str) -> RotationOutcome {
    rotate_refresh_token_bound(refresh_token_value, Some(tenant_id), None)
}

/// Rotate a standards-endpoint refresh token, deriving tenant from the
/// server-side token metadata and enforcing the registered client binding.
pub fn rotate_refresh_token_for_client(
    refresh_token_value: &str,
    client_id: &str,
) -> RotationOutcome {
    rotate_refresh_token_bound(refresh_token_value, None, Some(client_id))
}

fn rotate_refresh_token_bound(
    refresh_token_value: &str,
    expected_tenant_id: Option<&str>,
    expected_client_id: Option<&str>,
) -> RotationOutcome {
    use sesame_common::refresh_session::{self as refresh_store, Advance, Policy, Presented};

    let policy = Policy::from_env();

    let fail = |subreason: &str| {
        TOKEN_REFRESH_TOTAL
            .with_label_values(&["failure", subreason])
            .inc();
        REFRESH_ROTATION_FAILURES_TOTAL.inc();
    };
    if super::token_issuer::SIGNER
        .verify(refresh_token_value)
        .is_err()
    {
        fail("invalid_signature");
        return RotationOutcome::InvalidToken;
    }
    // The payload is signed (verified above): jti, subject and family come from the token itself.
    let Some(claims) = decode_refresh_token_claims(refresh_token_value) else {
        fail("decode_error");
        return RotationOutcome::InvalidToken;
    };
    let now = chrono::Utc::now().timestamp();

    // Round trip 1: the session, any pre-change record, the grace receipt and the token version.
    let (presented, token_version) =
        match refresh_store::inspect(&claims.family_id, &claims.jti, &claims.sub, now) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(event = "refresh_store_unavailable", error = %e);
                fail("redis_error");
                return RotationOutcome::RedisUnavailable;
            }
        };
    let (token, legacy, absolute_expiry) = match presented {
        Presented::Current(session) => {
            let abs = session.abs;
            (session.current(), None, abs)
        }
        Presented::Legacy(record) => {
            // A token issued before session records: its own expiry is the session's absolute cap.
            if record.exp <= now {
                fail("not_found");
                return RotationOutcome::InvalidToken;
            }
            let abs = record.exp;
            (record.clone(), Some(record), abs)
        }
        Presented::Replay(receipt) => {
            // The token rotated out moments ago (a reloaded tab, a racing second tab, a lost response): the same
            // successor again, never a second pair. See redis::ROTATION_REPLAY_GRACE_SECS.
            if let Some(outcome) =
                rotated_from_receipt(&receipt, expected_tenant_id, expected_client_id)
            {
                TOKEN_REFRESH_TOTAL
                    .with_label_values(&["success", "replayed"])
                    .inc();
                tracing::info!(
                    event = "refresh_replayed_within_grace",
                    old_jti = claims.jti,
                    "Rotated-out refresh token presented again within the grace window; successor returned"
                );
                return outcome;
            }
            fail("not_found");
            return RotationOutcome::InvalidToken;
        }
        Presented::Superseded(session) => {
            return reuse_detected(&session.family_id, &claims.jti);
        }
        Presented::Unknown => {
            fail("not_found");
            return RotationOutcome::InvalidToken;
        }
    };
    if token.sub != claims.sub {
        fail("subject_mismatch");
        return RotationOutcome::InvalidToken;
    }
    if expected_client_id.is_some_and(|expected| expected != token.client_id) {
        return RotationOutcome::InvalidToken;
    }
    let tenant_id = match expected_tenant_id {
        Some(expected) if token.tenant_id.is_empty() || token.tenant_id == expected => expected,
        Some(_) => return RotationOutcome::InvalidToken,
        None if !token.tenant_id.is_empty() => token.tenant_id.as_str(),
        None => return RotationOutcome::InvalidToken,
    };

    // The account must still be active. A refresh token outlives an access token by days; without this a disabled
    // (or deleted) user rotated their way to fresh access tokens indefinitely, roles and all. The session is revoked
    // so it ends here. A database failure refuses the refresh (503) without touching the session.
    match account_state(tenant_id, &token.sub) {
        AccountState::Active => {}
        AccountState::Inactive => {
            fail("account_inactive");
            if let Err(e) = refresh_store::revoke(&claims.family_id, &claims.jti) {
                tracing::error!(
                    event = "family_revocation_failed",
                    family_id = claims.family_id,
                    error = %e,
                    "Failed to revoke the session of an inactive account"
                );
            }
            return RotationOutcome::InvalidToken;
        }
        AccountState::Unknown => return RotationOutcome::RedisUnavailable,
    }

    // The successor: new jti, same session; it expires with the idle lifetime, capped at the session's absolute
    // expiry. Signed before the swap, so the swap can store it as the grace receipt in the same atomic step.
    let new_jti = Uuid::new_v4().to_string();
    let new_exp = policy.token_exp(now, absolute_expiry);
    let new_token = RefreshToken::new_with_tenant(
        new_jti.clone(),
        token.sub.clone(),
        token.sid.clone(),
        claims.family_id.clone(),
        now,
        new_exp,
        token.client_id.clone(),
        token.tenant_id.clone(),
        token.scopes.clone(),
    );
    let issued =
        match super::token_issuer::issue_rotated_tokens(&new_token, tenant_id, token_version) {
            Ok(issued) => issued,
            Err(e) => {
                tracing::error!(
                    event = "rotation_signing_failed",
                    error = e.to_string(),
                    "Failed to sign rotated tokens"
                );
                fail("signing_error");
                return RotationOutcome::RedisUnavailable;
            }
        };
    let receipt = serde_json::json!({
        "access_token": issued.access_token,
        "refresh_token": issued.refresh_token,
        "access_expires_in": issued.access_expires_in,
        "refresh_expires_in": issued.refresh_expires_in,
        "user_id": issued.user_id,
        "scope": issued.scope,
        "tenant_id": tenant_id,
        "client_id": token.client_id,
        "rotated_at": now,
    });

    // Round trip 2: compare-and-swap. Only one request can move the session off this jti.
    match refresh_store::advance(
        &claims.family_id,
        &claims.jti,
        &new_jti,
        legacy.as_ref(),
        absolute_expiry,
        now,
        &receipt,
        policy,
    ) {
        Ok(Advance::Rotated) => {
            TOKEN_REFRESH_TOTAL
                .with_label_values(&["success", "rotated"])
                .inc();
            RotationOutcome::Rotated {
                new_access_token: issued.access_token,
                new_refresh_token: issued.refresh_token,
                access_expires_in: issued.access_expires_in,
                refresh_expires_in: issued.refresh_expires_in,
                user_id: issued.user_id,
                scope: issued.scope,
            }
        }
        Ok(Advance::Replay(winner)) => {
            // A concurrent request with the same token won the swap: hand back its successor.
            TOKEN_REFRESH_TOTAL
                .with_label_values(&["success", "replayed"])
                .inc();
            rotated_from_receipt(&winner, expected_tenant_id, expected_client_id)
                .unwrap_or(RotationOutcome::InvalidToken)
        }
        Ok(Advance::Reuse) => {
            REFRESH_REUSE_DETECTED_TOTAL.inc();
            fail("reuse_detected");
            RotationOutcome::ReuseDetected {
                reused_jti: claims.jti,
                family_id: claims.family_id,
            }
        }
        Ok(Advance::Gone) => {
            fail("not_found");
            RotationOutcome::InvalidToken
        }
        Err(e) => {
            tracing::error!(event = "refresh_store_unavailable", error = %e);
            fail("redis_error");
            RotationOutcome::RedisUnavailable
        }
    }
}

/// A superseded token of a live session was presented: revoke the session (OAuth refresh-token reuse detection).
fn reuse_detected(family_id: &str, jti: &str) -> RotationOutcome {
    TOKEN_REFRESH_TOTAL
        .with_label_values(&["failure", "reuse_detected"])
        .inc();
    REFRESH_ROTATION_FAILURES_TOTAL.inc();
    REFRESH_REUSE_DETECTED_TOTAL.inc();
    if let Err(e) = sesame_common::refresh_session::revoke(family_id, jti) {
        tracing::error!(
            event = "family_revocation_failed",
            family_id = family_id,
            error = %e,
            "Failed to revoke token family after reuse detection"
        );
    }
    RotationOutcome::ReuseDetected {
        reused_jti: jti.to_string(),
        family_id: family_id.to_string(),
    }
}

/// The signed claims a refresh token carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshClaims {
    pub jti: String,
    pub sub: String,
    pub family_id: String,
}

/// Decode `jti`, `sub` and `family_id` from a refresh token (call only after its signature verified).
#[must_use]
pub fn decode_refresh_token_claims(token: &str) -> Option<RefreshClaims> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(&base64_decode_url(parts[1])?).ok()?;
    let s = |k: &str| {
        value
            .get(k)
            .and_then(serde_json::Value::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    if value.get("typ").and_then(serde_json::Value::as_str) != Some("refresh") {
        return None;
    }
    Some(RefreshClaims {
        jti: s("jti")?,
        sub: s("sub")?,
        family_id: s("family_id")?,
    })
}

/// End the session of a presented refresh token (logout): verifies the signature first, so only a holder of one of
/// the session's tokens can end it. Malformed or forged tokens are ignored (idempotent logout).
///
/// # Errors
///
/// Returns the Redis error.
pub fn revoke_session_of(refresh_token_value: &str) -> Result<()> {
    if super::token_issuer::SIGNER
        .verify(refresh_token_value)
        .is_err()
    {
        return Ok(());
    }
    let Some(claims) = decode_refresh_token_claims(refresh_token_value) else {
        return Ok(());
    };
    sesame_common::refresh_session::revoke(&claims.family_id, &claims.jti)?;
    Ok(())
}

/// A `Rotated` outcome rebuilt from a rotation receipt, or None when the
/// receipt does not match the caller's tenant / client binding (then the
/// strict not-found path applies). The successor's remaining lifetimes are
/// reported as issued minus the seconds elapsed since rotation.
fn rotated_from_receipt(
    receipt: &serde_json::Value,
    expected_tenant_id: Option<&str>,
    expected_client_id: Option<&str>,
) -> Option<RotationOutcome> {
    let s = |k: &str| receipt.get(k).and_then(|v| v.as_str()).map(str::to_string);
    let i = |k: &str| receipt.get(k).and_then(|v| v.as_i64());
    let tenant = s("tenant_id").unwrap_or_default();
    let client = s("client_id").unwrap_or_default();
    if let Some(t) = expected_tenant_id {
        if !tenant.is_empty() && tenant != t {
            return None;
        }
    }
    if let Some(c) = expected_client_id {
        if client != c {
            return None;
        }
    }
    let elapsed = (chrono::Utc::now().timestamp() - i("rotated_at").unwrap_or(0)).max(0);
    let remaining = |k: &str| (i(k).unwrap_or(0) - elapsed).max(0) as i32;
    Some(RotationOutcome::Rotated {
        new_access_token: s("access_token")?,
        new_refresh_token: s("refresh_token")?,
        access_expires_in: remaining("access_expires_in"),
        refresh_expires_in: remaining("refresh_expires_in"),
        user_id: s("user_id").unwrap_or_default(),
        scope: s("scope").unwrap_or_default(),
    })
}

/// Base64url decode a string.
fn base64_decode_url(input: &str) -> Option<String> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let bytes = engine.decode(input).ok()?;
    String::from_utf8(bytes).ok()
}

/// Check if a token has been reused (for reuse detection).
///
/// Returns true if the token's JTI is in the denylist.
pub fn check_token_reuse(jti: &str, family_id: &str) -> Result<bool> {
    let is_in_denylist = redis::is_in_denylist(jti)?;
    if is_in_denylist {
        return Ok(true);
    }

    // Also check family revocation sentinel
    let is_revoked = redis::is_family_revoked(family_id)?;
    Ok(is_revoked)
}

#[cfg(test)]
mod account_state_tests {
    use super::{account_state_of, AccountState};

    #[test]
    fn only_an_active_account_may_refresh() {
        assert_eq!(account_state_of(Some("active")), AccountState::Active);
        for s in [
            Some("disabled"),
            Some("deleted"),
            Some("Active"),
            Some(""),
            None,
        ] {
            assert_eq!(account_state_of(s), AccountState::Inactive, "{s:?}");
        }
    }
}
