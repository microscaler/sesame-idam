use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Issued tokens (access/refresh) tracked per session.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "tokens"]
#[schema_name = "sesame_idam"]
pub struct Token {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: uuid::Uuid,

    #[column_type = "UUID"]
    #[nullable]
    #[foreign_key = "sesame_idam.sessions(id) ON DELETE CASCADE"]
    pub session_id: Option<uuid::Uuid>,

    #[column_type = "VARCHAR(32)"]
    pub type_field: String,

    #[column_type = "TEXT"]
    pub token: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub expires_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
