use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// API key stored by api-keys service.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "api_keys"]
#[schema_name = "sesame_idam"]
pub struct ApiKey {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "TEXT"]
    pub key_hash: String,

    #[column_type = "VARCHAR(16)"]
    pub key_prefix: String,

    #[column_type = "VARCHAR(255)"]
    pub name: String,

    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    #[column_type = "UUID"]
    #[nullable]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: Option<uuid::Uuid>,

    #[column_type = "UUID"]
    #[nullable]
    #[foreign_key = "sesame_idam.organizations(id) ON DELETE CASCADE"]
    pub org_id: Option<uuid::Uuid>,

    #[column_type = "TEXT"]
    #[nullable]
    pub permissions: Option<String>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[nullable]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,

    #[column_type = "BOOLEAN"]
    pub active: bool,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
