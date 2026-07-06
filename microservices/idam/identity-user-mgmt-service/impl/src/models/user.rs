use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// User entity for identity-user-mgmt-service.
///
/// NOTE: identity-login-service duplicates this entity (same table) for
/// credential verification at login — keep the definitions in sync.
/// `UNIQUE(tenant_id, email)`: the same email on different tenants is a
/// different, unrelated user (hard-segment tenancy).
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "users"]
#[schema_name = "sesame_idam"]
#[composite_unique = "tenant_id, email"]
pub struct User {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub email: String,

    #[column_type = "TEXT"]
    pub password_hash: String,

    #[column_type = "VARCHAR(255)"]
    pub tenant_id: String,

    #[column_type = "VARCHAR(32)"]
    pub status: String,

    #[column_type = "BOOLEAN"]
    pub email_verified: bool,

    #[column_type = "VARCHAR(64)"]
    #[nullable]
    pub phone: Option<String>,

    #[column_type = "BOOLEAN"]
    pub phone_verified: bool,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
