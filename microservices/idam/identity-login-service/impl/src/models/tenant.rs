use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Platform tenant registry — canonical definition for identity-login-service.
///
/// A tenant is a SaaS product partition (`hauliage`, `pricewhisperer`, …).
/// Slug matches `X-Tenant-ID`. Tenants must be provisioned before any auth
/// traffic is accepted (no implicit / magic tenants).
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "tenants"]
#[schema_name = "sesame_idam"]
pub struct Tenant {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    /// External identifier (`X-Tenant-ID`). Unique across the platform.
    #[column_type = "VARCHAR(64)"]
    #[unique]
    pub slug: String,

    #[column_type = "VARCHAR(255)"]
    pub display_name: String,

    /// `active` | `suspended` | `provisioning`
    #[column_type = "VARCHAR(32)"]
    pub status: String,

    /// `platform` (ops minted) | `self_service` (SaaS signup)
    #[column_type = "VARCHAR(32)"]
    pub provisioning_mode: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
