use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// API key usage tracking.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "api_key_usage"]
#[schema_name = "sesame_idam"]
pub struct ApiKeyUsage {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.api_keys(id) ON DELETE CASCADE"]
    pub key_id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub endpoint: String,

    #[column_type = "VARCHAR(16)"]
    pub method: String,

    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    #[column_type = "VARCHAR(64)"]
    pub ip: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,
}
