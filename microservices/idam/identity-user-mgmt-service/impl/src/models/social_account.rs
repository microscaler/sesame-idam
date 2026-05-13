use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Social account linking stored by user-mgmt.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "social_accounts"]
#[schema_name = "sesame_idam"]
pub struct SocialAccount {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: uuid::Uuid,

    #[column_type = "VARCHAR(64)"]
    pub provider: String,

    #[column_type = "VARCHAR(255)"]
    pub provider_user_id: String,

    #[column_type = "TEXT"]
    #[nullable]
    pub access_token: Option<String>,

    #[column_type = "TEXT"]
    #[nullable]
    pub refresh_token: Option<String>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
