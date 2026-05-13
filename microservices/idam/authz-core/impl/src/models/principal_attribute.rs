use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Custom attributes attached to a principal.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "principal_attributes"]
#[schema_name = "sesame_idam"]
pub struct PrincipalAttribute {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub principal_id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub key: String,

    #[column_type = "TEXT"]
    pub value: String,

    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
