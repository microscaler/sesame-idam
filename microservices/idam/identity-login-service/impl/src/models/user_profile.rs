// Duplicated from identity-session-service (which owns the canonical
// definition) so login can seed user_profiles at registration. Shared
// entities are duplicated per service by convention; the migrator merges
// identical table definitions by name. Keep in sync with
// identity-session-service/impl/src/models/user_profile.rs.
use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Extended user profile metadata separate from auth credentials.
#[allow(clippy::pub_underscore_fields)]
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "user_profiles"]
#[schema_name = "sesame_idam"]
pub struct UserProfile {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    #[nullable]
    pub first_name: Option<String>,

    #[column_type = "VARCHAR(255)"]
    #[nullable]
    pub last_name: Option<String>,

    #[column_type = "TEXT"]
    #[nullable]
    pub avatar_url: Option<String>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
