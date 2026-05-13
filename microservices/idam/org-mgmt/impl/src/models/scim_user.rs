use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// SCIM user provisioned for an organization.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "scim_users"]
#[schema_name = "sesame_idam"]
pub struct ScimUser {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.organizations(id) ON DELETE CASCADE"]
    pub org_id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub external_id: String,

    #[column_type = "VARCHAR(255)"]
    pub username: String,

    #[column_type = "VARCHAR(255)"]
    pub email: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
