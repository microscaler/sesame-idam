use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// User entity — duplicated from identity-user-mgmt-service (which owns the
/// canonical definition) so login can verify credentials against the shared
/// `sesame_idam.users` table. Shared entities are duplicated per service by
/// convention (same as `OpenAPI` schema duplication); the migrator merges
/// identical table definitions by name.
///
/// Keep in sync with
/// `identity-user-mgmt-service/impl/src/models/user.rs`.
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
