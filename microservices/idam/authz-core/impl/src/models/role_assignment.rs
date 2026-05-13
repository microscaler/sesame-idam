use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Role assignment for a principal within a tenant.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "role_assignments"]
#[schema_name = "sesame_idam"]
pub struct RoleAssignment {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub principal_id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub role_name: String,

    #[column_type = "VARCHAR(255)"]
    pub resource_type: String,

    #[column_type = "UUID"]
    #[nullable]
    #[foreign_key = "sesame_idam.organizations(id) ON DELETE CASCADE"]
    pub resource_id: Option<uuid::Uuid>,

    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
