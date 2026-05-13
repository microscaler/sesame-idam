use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Represents a social login credential linked to a user account.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "social_credentials"]
#[schema_name = "sesame_idam"]
pub struct SocialCredential {
    /// Primary key - UUID v4
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    /// Associated user
    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: uuid::Uuid,

    /// OAuth provider name (google, github, etc.)
    #[column_type = "VARCHAR(64)"]
    pub provider: String,

    /// User's ID on the provider's platform
    #[column_type = "VARCHAR(255)"]
    pub provider_user_id: String,

    /// OAuth access token
    #[column_type = "TEXT"]
    #[nullable]
    pub access_token: Option<String>,

    /// OAuth refresh token
    #[column_type = "TEXT"]
    #[nullable]
    pub refresh_token: Option<String>,

    /// Timestamp of credential creation
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Timestamp of last update
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
