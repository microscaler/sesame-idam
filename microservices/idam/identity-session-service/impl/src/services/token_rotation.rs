//! Token rotation service — the core business logic for refresh token rotation.
//!
//! Implements the rotation flow per Story 3.1:
//!   1. Validate the refresh token exists in Redis
//!   2. Check denylist for reuse detection
//!   3. If reused → revoke entire family → 401
//!   4. If clean → rotate → issue new tokens
//!
//! Also provides metrics counters for telemetry.

use anyhow::Result;
use prometheus::{register_int_counter, register_int_counter_vec, IntCounter, IntCounterVec};
use uuid::Uuid;

use crate::models::refresh_token::RefreshToken;
use crate::redis;

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

lazy_static::lazy_static! {
    /// Total number of refresh attempts, labeled by result and subreason.
    pub static ref TOKEN_REFRESH_TOTAL: IntCounterVec = register_int_counter_vec!(
        "token_refresh_total",
        "Total number of /auth/refresh requests",
        &["result", "subreason"]
    ).unwrap();

    /// Number of refreshes where reuse was detected (family revoked).
    pub static ref REFRESH_REUSE_DETECTED_TOTAL: IntCounter = register_int_counter!(
        "refresh_reuse_detected_total",
        "Total number of refresh token reuse detections (family revocations)",
    ).unwrap();

    /// Number of rotation failures (invalid token, Redis error, etc.).
    pub static ref REFRESH_ROTATION_FAILURES_TOTAL: IntCounter = register_int_counter!(
        "refresh_rotation_failures_total",
        "Total number of refresh rotation failures",
    ).unwrap();
}

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

// ---------------------------------------------------------------------------
// Core rotation logic
// ---------------------------------------------------------------------------

/// Rotate a refresh token.
///
/// Implements the rotation flow from Story 3.1:
/// 1. Look up the refresh token in Redis by its JTI
/// 2. Check if the JTI is in the denylist (reuse detection)
/// 3. If found in denylist → revoke family → return ReuseDetected
/// 4. If clean → delete old token, issue new tokens, add old JTI to denylist
///
/// # Parameters
/// - `refresh_token_value`: The refresh token string from the client
/// - `family_id`: The token family ID (used for reuse detection)
/// - `user_id`: The user ID (for metrics and logging)
///
/// # Returns
/// - `RotationOutcome::Rotated` with new tokens on success
/// - `RotationOutcome::ReuseDetected` if the old JTI was in the denylist
/// - `RotationOutcome::InvalidToken` if the token is not in Redis
/// - `RotationOutcome::RedisUnavailable` if Redis is down
pub fn rotate_refresh_token(
    refresh_token_value: &str,
    family_id: &str,
    user_id: &str,
) -> RotationOutcome {
    let parts: Vec<&str> = refresh_token_value.split('.').collect();
    if parts.len() != 3 {
        TOKEN_REFRESH_TOTAL
            .with_label_values(&["failure", "invalid_format"])
            .inc();
        REFRESH_ROTATION_FAILURES_TOTAL.inc();
        return RotationOutcome::InvalidToken;
    }

    // Decode the JTI from the refresh token (payload portion)
    let jti = match decode_refresh_token_jti(refresh_token_value) {
        Some(j) => j,
        None => {
            TOKEN_REFRESH_TOTAL
                .with_label_values(&["failure", "decode_error"])
                .inc();
            REFRESH_ROTATION_FAILURES_TOTAL.inc();
            return RotationOutcome::InvalidToken;
        }
    };

    // Step 1: Look up the refresh token in Redis
    let token = match redis::lookup_refresh_token(&jti) {
        Ok(Some(t)) => t,
        Ok(None) => {
            TOKEN_REFRESH_TOTAL
                .with_label_values(&["failure", "not_found"])
                .inc();
            REFRESH_ROTATION_FAILURES_TOTAL.inc();
            return RotationOutcome::InvalidToken;
        }
        Err(_) => {
            TOKEN_REFRESH_TOTAL
                .with_label_values(&["failure", "redis_error"])
                .inc();
            REFRESH_ROTATION_FAILURES_TOTAL.inc();
            return RotationOutcome::RedisUnavailable;
        }
    };

    // Step 2: Check if this JTI is in the denylist (reuse detection)
    // This detects the "tear" scenario where an attacker used the token
    // after the legitimate user rotated it
    if token.jti == jti && redis::is_in_denylist(&jti).unwrap_or(false) {
        TOKEN_REFRESH_TOTAL
            .with_label_values(&["failure", "reuse_detected"])
            .inc();
        REFRESH_REUSE_DETECTED_TOTAL.inc();

        // Revoke the entire family (F-005: cross-session notification)
        if let Err(e) = redis::delete_family_tokens(family_id) {
            tracing::error!(
                event = "family_revocation_failed",
                family_id = family_id,
                error = e.to_string(),
                "Failed to revoke token family after reuse detection"
            );
        }

        return RotationOutcome::ReuseDetected {
            reused_jti: jti,
            family_id: family_id.to_string(),
        };
    }

    // Step 3: Normal rotation — delete old token
    if let Err(e) = redis::delete_refresh_token(&jti) {
        tracing::warn!(
            event = "rotation_old_token_delete_failed",
            jti = jti,
            error = e.to_string(),
            "Failed to delete old refresh token during rotation"
        );
    }

    // Step 4: Issue new refresh token (new JTI, same family)
    let new_jti = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    let new_exp = now + (crate::models::refresh_token::REFRESH_TOKEN_TTL as i64);

    let new_token = RefreshToken::new(
        new_jti.clone(),
        token.sub.clone(),
        token.sid.clone(),
        family_id.to_string(),
        now,
        new_exp,
        token.client_id.clone(),
        token.scopes.clone(),
    );

    // Store new refresh token in Redis
    if let Err(e) = redis::store_refresh_token(&new_token) {
        tracing::error!(
            event = "rotation_new_token_store_failed",
            new_jti = new_jti,
            error = e.to_string(),
            "Failed to store new refresh token"
        );
        return RotationOutcome::RedisUnavailable;
    }

    // Step 5: Add old JTI to denylist (24h TTL — prevents replay)
    if let Err(e) = redis::add_to_denylist(&jti) {
        tracing::warn!(
            event = "rotation_denylist_add_failed",
            old_jti = jti,
            error = e.to_string(),
            "Failed to add JTI to denylist during rotation"
        );
    }

    // Emit metrics
    TOKEN_REFRESH_TOTAL
        .with_label_values(&["success", "rotated"])
        .inc();

    // Step 6: Generate new tokens (access + refresh)
    let access_expires_in = 300; // 5 minutes
    let refresh_expires_in = crate::models::refresh_token::REFRESH_TOKEN_TTL as i32;

    let new_access_token = generate_access_token(
        &token.sub,
        &token.client_id,
        &token.scopes,
        family_id,
        &jti, // old JTI as version indicator
    );

    let new_refresh = new_token.to_json().unwrap_or_default();

    RotationOutcome::Rotated {
        new_access_token,
        new_refresh_token: new_refresh,
        access_expires_in,
        refresh_expires_in,
    }
}

/// Decode the JTI from a refresh token string.
///
/// The refresh token is a JWT-like structure (header.payload.signature).
/// The JTI is stored in the payload as the `jti` field.
fn decode_refresh_token_jti(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    // Decode the payload (base64url)
    let payload = parts[1];
    let decoded = base64_decode_url(payload)?;

    // Parse as JSON and extract `jti`
    let value: serde_json::Value = serde_json::from_str(&decoded).ok()?;
    value.get("jti")?.as_str().map(|s| s.to_string())
}

/// Base64url decode a string.
fn base64_decode_url(input: &str) -> Option<String> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let bytes = engine.decode(input).ok()?;
    String::from_utf8(bytes).ok()
}

/// Generate a new access token for the refreshed session.
///
/// In production, this would use the real JWT signing key (RS256/ES256).
/// For now, we generate a placeholder token.
fn generate_access_token(
    user_id: &str,
    client_id: &str,
    scopes: &str,
    family_id: &str,
    version_hint: &str,
) -> String {
    use base64::Engine;
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    // Header
    let header = serde_json::json!({
        "alg": "RS256",
        "typ": "JWT",
        "kid": "default-key",
    });
    let header_b64 = engine.encode(serde_json::to_string(&header).unwrap());

    // Payload
    let now = chrono::Utc::now().timestamp();
    let payload = serde_json::json!({
        "iss": "https://idam.example.com",
        "sub": user_id,
        "aud": ["identity-session-service"],
        "iat": now,
        "exp": now + 300,
        "jti": Uuid::new_v4().to_string(),
        "sid": family_id,
        "client_id": client_id,
        "scope": scopes,
        "ver": 1,
    });
    let payload_b64 = engine.encode(serde_json::to_string(&payload).unwrap());

    // Signature (placeholder — replace with real signing in production)
    format!("{}.{}.placeholder_signature", header_b64, payload_b64)
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
