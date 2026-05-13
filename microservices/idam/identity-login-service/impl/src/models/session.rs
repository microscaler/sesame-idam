use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Represents an active or expired authentication session.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "sessions"]
#[schema_name = "sesame_idam"]
pub struct Session {
    /// Primary key - UUID v4
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    /// Associated user
    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: uuid::Uuid,

    /// Access token
    #[column_type = "TEXT"]
    pub token: String,

    /// Refresh token (hash for security)
    #[column_type = "TEXT"]
    pub refresh_token: String,

    /// Session expiration timestamp
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub expires_at: chrono::DateTime<chrono::Utc>,

    /// Client IP address
    #[column_type = "VARCHAR(64)"]
    #[nullable]
    pub ip: Option<String>,

    /// Client user agent string
    #[column_type = "TEXT"]
    #[nullable]
    pub user_agent: Option<String>,

    /// Timestamp of session creation
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Timestamp of last activity
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
