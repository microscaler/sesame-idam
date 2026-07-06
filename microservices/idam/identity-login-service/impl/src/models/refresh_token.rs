//! Refresh token metadata stored in Redis at login time.
//!
//! The JSON shape and Redis key layout MUST match
//! `identity-session-service/impl/src/models/refresh_token.rs` — the session
//! service's `/auth/refresh` rotation reads tokens this service writes.
//! (Redis-only struct: no Lifeguard entity here, so no duplicate migration.)

use serde::{Deserialize, Serialize};

/// Prefix for refresh-token metadata: `refresh:{jti}` (30d TTL).
pub const REFRESH_TOKEN_KEY_PREFIX: &str = "refresh";

/// Prefix for token family sets: `family:{family_id}` (24h TTL).
pub const FAMILY_SET_KEY_PREFIX: &str = "family";

/// TTL for refresh-token metadata (30 days) in seconds.
pub const REFRESH_TOKEN_TTL: u32 = 2_592_000;

/// TTL for family sets (24 hours) in seconds.
pub const FAMILY_TTL: u32 = 86_400;

/// Refresh token payload stored in Redis `refresh:{jti}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefreshToken {
    /// Unique token ID (also embedded as `jti` in the refresh JWT).
    pub jti: String,
    /// User ID (subject).
    pub sub: String,
    /// Session ID.
    pub sid: String,
    /// Token family identifier (for reuse detection).
    pub family_id: String,
    /// Issued at (Unix timestamp).
    pub iat: i64,
    /// Expiration (Unix timestamp).
    pub exp: i64,
    /// Client application identifier.
    pub client_id: String,
    /// Space-delimited scopes.
    pub scopes: String,
}

impl RefreshToken {
    /// Serialize to JSON for Redis storage.
    ///
    /// # Errors
    ///
    /// Returns a `serde_json::Error` if serialization fails.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}
