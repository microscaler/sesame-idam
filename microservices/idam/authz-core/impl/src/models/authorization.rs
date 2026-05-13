use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Authorization record (ABAC-style).
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "authorizations"]
#[schema_name = "sesame_idam"]
pub struct Authorization {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub principal_id: uuid::Uuid,

    #[column_type = "VARCHAR(128)"]
    pub action: String,

    #[column_type = "VARCHAR(255)"]
    pub resource: String,

    #[column_type = "VARCHAR(16)"]
    pub effect: String, // allow / deny

    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
