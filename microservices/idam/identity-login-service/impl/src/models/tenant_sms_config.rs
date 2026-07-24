use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Per-`(tenant, environment)` SMS sender configuration (ADR-009 §7).
///
/// # What is and is not stored here
///
/// NEVER a plaintext credential. Two custody modes are representable:
///
/// - `connect` (PREFERRED, required for external tenants): only a revocable
///   `connected_account_sid`. Twilio bills the tenant directly and Sesame
///   holds no secret at all — the least-liability option.
/// - `envelope` (dogfood tenants only): the tenant's auth token encrypted
///   with a per-tenant data key (AES-256-GCM); the data key itself is stored
///   WRAPPED by a KEK held in the secret backend. Decryption is in-process at
///   send time and never logged.
///
/// A row per `(tenant_id, environment)` rather than one row with an
/// environment column: staging and prod are genuinely different senders with
/// different numbers and separate A2P/10DLC campaign registration, and this
/// keeps their ceilings and revocation independent.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "tenant_sms_config"]
#[schema_name = "sesame_idam"]
#[composite_unique = "tenant_id, environment"]
pub struct TenantSmsConfig {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    /// Tenant slug (`X-Tenant-ID`).
    #[column_type = "VARCHAR(64)"]
    pub tenant_id: String,

    /// `dev` | `staging` | `prod` …
    #[column_type = "VARCHAR(32)"]
    pub environment: String,

    /// `twilio` today; the provider trait allows others later.
    #[column_type = "VARCHAR(32)"]
    pub provider: String,

    /// `connect` | `envelope`
    #[column_type = "VARCHAR(16)"]
    pub custody_mode: String,

    /// Twilio Connect: the tenant's connected account. Not a secret.
    #[column_type = "VARCHAR(64)"]
    #[nullable]
    pub connected_account_sid: Option<String>,

    /// Envelope custody: the tenant's account SID (not secret) …
    #[column_type = "VARCHAR(64)"]
    #[nullable]
    pub account_sid: Option<String>,

    /// … the auth token, AES-256-GCM ciphertext (base64url).
    #[column_type = "TEXT"]
    #[nullable]
    pub auth_token_ciphertext: Option<String>,

    /// … its nonce (base64url).
    #[column_type = "TEXT"]
    #[nullable]
    pub auth_token_nonce: Option<String>,

    /// … and the per-tenant data key, wrapped by the backend KEK
    /// (base64url). Never the raw DEK.
    #[column_type = "TEXT"]
    #[nullable]
    pub dek_wrapped: Option<String>,

    /// Preferred sender identity (number pools, compliance, geo-routing).
    #[column_type = "VARCHAR(64)"]
    #[nullable]
    pub messaging_service_sid: Option<String>,

    /// Fallback sender identity, E.164.
    #[column_type = "VARCHAR(32)"]
    #[nullable]
    pub from_number: Option<String>,

    /// A2P/10DLC (or regional) campaign registration — compliance follows the
    /// sending BRAND, so a tenant under its own brand owns this even under
    /// Connect custody.
    #[column_type = "VARCHAR(64)"]
    #[nullable]
    pub campaign_ref: Option<String>,

    /// This tenant's own daily spend ceiling (ADR-009 §2.4) — independent of
    /// the platform's and of every other tenant's.
    #[column_type = "INTEGER"]
    pub daily_spend_ceiling_cents: i32,

    /// `active` | `pending_validation` | `revoked`. Credentials are NOT
    /// trusted until a live validation moves them to `active`.
    #[column_type = "VARCHAR(32)"]
    pub status: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[nullable]
    pub last_validated_at: Option<chrono::DateTime<chrono::Utc>>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Custody modes.
pub const CUSTODY_CONNECT: &str = "connect";
pub const CUSTODY_ENVELOPE: &str = "envelope";

/// Config lifecycle states.
pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_PENDING_VALIDATION: &str = "pending_validation";
pub const STATUS_REVOKED: &str = "revoked";
