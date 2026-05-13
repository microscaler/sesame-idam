use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Audit event stored by authz-core.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "audit_events"]
#[schema_name = "sesame_idam"]
pub struct AuditEvent {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    #[column_type = "VARCHAR(64)"]
    pub event_type: String,

    #[column_type = "VARCHAR(32)"]
    pub severity: String,

    #[column_type = "VARCHAR(32)"]
    pub actor: String,

    #[column_type = "TEXT"]
    #[nullable]
    pub data: Option<String>,

    #[column_type = "VARCHAR(64)"]
    #[nullable]
    pub ip: Option<String>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,
}
