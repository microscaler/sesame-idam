use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Represents an OTP token used for email/phone/dual-step verification.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "otp_tokens"]
#[schema_name = "sesame_idam"]
pub struct OTPToken {
    /// Primary key - UUID v4
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    /// Associated user
    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: uuid::Uuid,

    /// OTP type: email, phone, dual
    #[column_type = "VARCHAR(32)"]
    pub type_field: String,

    /// The OTP code (hashed for security)
    #[column_type = "VARCHAR(64)"]
    pub code: String,

    /// Expiration timestamp
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub expires_at: chrono::DateTime<chrono::Utc>,

    /// Number of failed verification attempts
    #[column_type = "INTEGER"]
    pub attempts: i32,

    /// Maximum allowed attempts before token is invalidated
    #[column_type = "INTEGER"]
    pub max_attempts: i32,

    /// Timestamp of token creation
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Timestamp of last update
    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
