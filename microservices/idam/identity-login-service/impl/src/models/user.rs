use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Represents a registered user in the system.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "users"]
#[schema_name = "sesame_idam"]
pub struct User {
    /// Primary key - UUID v4
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    /// User's email address (unique within tenant)
    #[column_type = "VARCHAR(255)"]
    pub email: String,

    /// Hashed password (empty if social-only or magic-link-only user)
    #[column_type = "TEXT"]
    pub password_hash: String,

    /// Tenant this user belongs to (hard isolation boundary)
    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    /// Email verification status
    #[column_type = "BOOLEAN"]
    pub email_verified: bool,

    /// Phone number (E.164 format, optional)
    #[column_type = "VARCHAR(64)"]
    #[nullable]
    pub phone: Option<String>,

    /// Phone verification status
    #[column_type = "BOOLEAN"]
    pub phone_verified: bool,

    /// Account status: active, disabled, deleted
    #[column_type = "VARCHAR(32)"]
    pub status: String,

    /// Timestamp of account creation
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Timestamp of last update
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
