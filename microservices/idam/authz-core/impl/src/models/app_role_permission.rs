use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Tenant/application role → permission mapping for JWT enrichment.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "app_role_permissions"]
#[schema_name = "sesame_idam"]
pub struct AppRolePermission {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    #[column_type = "VARCHAR(255)"]
    pub app_id: String,

    #[column_type = "VARCHAR(255)"]
    pub role_name: String,

    #[column_type = "VARCHAR(255)"]
    pub permission: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,
}
