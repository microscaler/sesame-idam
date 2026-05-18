use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Active or expired authentication session with refresh tokens and impersonation metadata.
#[allow(clippy::pub_underscore_fields)]
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "sessions"]
#[schema_name = "sesame_idam"]
pub struct Session {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: uuid::Uuid,

    #[column_type = "TEXT"]
    pub token: String,

    #[column_type = "TEXT"]
    pub refresh_token: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub expires_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "VARCHAR(64)"]
    #[nullable]
    pub ip: Option<String>,

    #[column_type = "TEXT"]
    #[nullable]
    pub user_agent: Option<String>,

    /// Whether MFA was verified in this session.
    #[column_type = "BOOLEAN"]
    pub mfa_verified: bool,

    /// If impersonated, the admin user id who performed impersonation.
    #[column_type = "UUID"]
    #[nullable]
    pub impersonated_by: Option<uuid::Uuid>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
