use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Organization entity.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "organizations"]
#[schema_name = "sesame_idam"]
pub struct Org {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub name: String,

    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    #[column_type = "VARCHAR(32)"]
    pub status: String,

    /// Opaque tenant-scoped product metadata (e.g. Hauliage `hauliage_profile_type`
    /// SHIPPER/HAULIER per ADR-002 §3.3). Sesame stores and echoes it without
    /// interpreting persona semantics.
    #[column_type = "JSONB"]
    pub metadata: Option<serde_json::Value>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
