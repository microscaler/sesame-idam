use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Represents a magic link token for passwordless authentication.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "magic_link_tokens"]
#[schema_name = "sesame_idam"]
pub struct MagicLinkToken {
    /// Primary key - UUID v4
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    /// Associated user
    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: uuid::Uuid,

    /// The magic link (encoded token)
    #[column_type = "TEXT"]
    pub link: String,

    /// Expiration timestamp
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub expires_at: chrono::DateTime<chrono::Utc>,

    /// Whether the link has been used
    #[column_type = "BOOLEAN"]
    pub used: bool,

    /// Timestamp of token creation
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Timestamp of last update
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
