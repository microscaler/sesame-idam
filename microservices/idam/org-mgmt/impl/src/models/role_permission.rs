use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Mapping of roles to permissions.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "role_permissions"]
#[schema_name = "sesame_idam"]
pub struct RolePermission {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.roles(id) ON DELETE CASCADE"]
    pub role_id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.permissions(id) ON DELETE CASCADE"]
    pub permission_id: uuid::Uuid,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,
}
