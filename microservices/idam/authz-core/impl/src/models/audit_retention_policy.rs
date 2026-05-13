use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Audit log retention policy per tenant.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "audit_retention_policies"]
#[schema_name = "sesame_idam"]
pub struct AuditRetentionPolicy {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    #[column_type = "INTEGER"]
    pub retention_days: i32,

    #[column_type = "BOOLEAN"]
    pub enabled: bool,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
