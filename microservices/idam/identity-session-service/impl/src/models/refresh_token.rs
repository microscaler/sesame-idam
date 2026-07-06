//! Refresh token rotation support.
//!
//! Implements rotating refresh tokens where each `/refresh` call validates the old token,
//! invalidates it, issues a new refresh token with a new `jti`, and stores the old `jti`
//! in the denylist cache for family TTL.

use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Redis key constants
// ---------------------------------------------------------------------------

/// Prefix for refresh-token metadata: `refresh:{jti}` (hash, 30d TTL)
pub const REFIRESH_TOKEN_KEY_PREFIX: &str = "refresh";

/// Prefix for token family sets: `family:{family_id}` (set, 24h TTL)
pub const FAMILY_SET_KEY_PREFIX: &str = "family";

/// Prefix for denylist entries: `denylist:{jti}` (string, 24h TTL)
pub const DENYLIST_KEY_PREFIX: &str = "denylist";

/// Prefix for session state: `session:{sid}` (hash, 30d TTL)
pub const SESSION_KEY_PREFIX: &str = "session";

/// TTL for refresh-token metadata (30 days) in seconds
pub const REFRESH_TOKEN_TTL: u32 = 2_592_000;

/// TTL for family sets and denylist entries (24 hours) in seconds
pub const FAMILY_TTL: u32 = 86_400;

/// Maximum denylist size per user before oldest entries are evicted
pub const MAX_DENYLIST_SIZE: usize = 1000;

/// Sentinel value to mark entire family as revoked
pub const FAMILY_REVOKED: &str = "__REVOKED__";

// ---------------------------------------------------------------------------
// RefreshToken data structure
// ---------------------------------------------------------------------------

/// Refresh token payload stored in Redis `refresh:{jti}` hash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefreshToken {
    /// Unique token ID (also in access token denylist)
    pub jti: String,
    /// User ID (subject)
    pub sub: String,
    /// Session ID
    pub sid: String,
    /// Token family identifier (for reuse detection)
    pub family_id: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
    /// Client application identifier
    pub client_id: String,
    /// Space-delimited scopes
    pub scopes: String,
}

impl RefreshToken {
    /// Create a new `RefreshToken` instance.
    pub fn new(
        jti: String,
        sub: String,
        sid: String,
        family_id: String,
        iat: i64,
        exp: i64,
        client_id: String,
        scopes: String,
    ) -> Self {
        Self {
            jti,
            sub,
            sid,
            family_id,
            iat,
            exp,
            client_id,
            scopes,
        }
    }

    /// Serialize to JSON for Redis storage.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON (from Redis).
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }
}

// ---------------------------------------------------------------------------
// Lifeguard entity for persistent token tracking (optional / audit)
// ---------------------------------------------------------------------------

/// Persistent token record for audit trail.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "refresh_tokens"]
#[schema_name = "sesame_idam"]
pub struct RefreshTokenRecord {
    /// Refresh token identifier (matches Redis jti)
    #[primary_key]
    #[column_type = "VARCHAR(32)"]
    pub id: String,

    /// Token type: "access" or "refresh"
    #[column_type = "VARCHAR(16)"]
    pub type_field: String,

    /// The JWT / opaque token string (refresh token payload or access token)
    #[column_type = "TEXT"]
    pub token: String,

    /// Associated user ID
    #[column_type = "UUID"]
    pub user_id: uuid::Uuid,

    /// Associated session ID
    #[column_type = "UUID"]
    #[nullable]
    pub session_id: Option<uuid::Uuid>,

    /// Token version at time of issue
    #[column_type = "INTEGER"]
    pub token_version: i32,

    /// Expiration timestamp
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub expires_at: chrono::DateTime<chrono::Utc>,

    /// Issued at timestamp
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub issued_at: chrono::DateTime<chrono::Utc>,

    /// Rotation sequence counter (monotonic within a family)
    #[column_type = "INTEGER"]
    pub rotation_seq: i32,

    /// Whether this token has been revoked
    #[column_type = "BOOLEAN"]
    pub revoked: bool,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
