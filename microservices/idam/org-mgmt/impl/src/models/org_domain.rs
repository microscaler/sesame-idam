use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// An organisation's claim on an email domain (DESIGN-org-domain-verification.md §2, §8; story 17.3).
///
/// Never marked verified by a request: only `sesame_common::domain_verify::lifecycle::apply` moves `state`, from a
/// DNS verdict, time, or an admin withdrawing the claim. One holder per tenant: a partial unique index allows a single
/// claim per (`tenant_id`, `domain`) in VERIFIED or GRACE. Tenant RLS (ADR-005).
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "org_domains"]
#[schema_name = "sesame_idam"]
pub struct OrgDomain {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.organizations(id) ON DELETE CASCADE"]
    pub org_id: uuid::Uuid,

    /// Normalised: lower-case IDNA A-label, no trailing dot.
    #[column_type = "VARCHAR(255)"]
    pub domain: String,

    #[column_type = "VARCHAR(64)"]
    pub tenant_id: String,

    /// `exact` | `subdomains`.
    #[column_type = "VARCHAR(16)"]
    #[default_value = "'exact'"]
    pub scope: String,

    /// PENDING | VERIFIED | GRACE | REVOKED | EXPIRED | CONFLICT.
    #[column_type = "VARCHAR(16)"]
    #[default_value = "'PENDING'"]
    pub state: String,

    /// `K_enroll` epoch of the last match.
    #[column_type = "INTEGER"]
    #[default_value = "1"]
    pub epoch: i32,

    /// The last match was DNSSEC-validated by every provider (false until one is). Not nullable: Lifeguard binds a
    /// NULL `Option<bool>` with the wrong type on insert.
    #[column_type = "BOOLEAN"]
    #[default_value = "false"]
    pub dnssec: bool,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[nullable]
    pub pending_until: Option<chrono::DateTime<chrono::Utc>>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[nullable]
    pub verified_at: Option<chrono::DateTime<chrono::Utc>>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[nullable]
    pub last_checked_at: Option<chrono::DateTime<chrono::Utc>>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[nullable]
    pub last_verified_at: Option<chrono::DateTime<chrono::Utc>>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[nullable]
    pub grace_until: Option<chrono::DateTime<chrono::Utc>>,

    #[column_type = "BOOLEAN"]
    #[default_value = "false"]
    pub auto_join: bool,

    #[column_type = "UUID"]
    #[nullable]
    pub created_by: Option<uuid::Uuid>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
