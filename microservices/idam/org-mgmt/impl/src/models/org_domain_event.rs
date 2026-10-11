use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Append-only history of a domain claim: one row per state change (story 17.3). The service login may insert and
/// read, never update or delete (trigger and RLS, migrations/org-mgmt). `detail` never carries the token.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "org_domain_events"]
#[schema_name = "sesame_idam"]
pub struct OrgDomainEvent {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "VARCHAR(64)"]
    pub tenant_id: String,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.org_domains(id) ON DELETE CASCADE"]
    pub org_domain_id: uuid::Uuid,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[default_value = "now()"]
    pub at: chrono::DateTime<chrono::Utc>,

    /// created checked verified grace revoked expired conflict deleted auto_join
    #[column_type = "VARCHAR(32)"]
    pub event: String,

    #[column_type = "JSONB"]
    #[default_value = "'{}'"]
    pub detail: serde_json::Value,
}
