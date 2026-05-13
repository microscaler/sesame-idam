use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Records an admin impersonation session.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "impersonations"]
#[schema_name = "sesame_idam"]
pub struct Impersonation {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub impersonator_id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.sessions(id) ON DELETE CASCADE"]
    pub session_id: uuid::Uuid,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub restored_at: chrono::DateTime<chrono::Utc>,
}
