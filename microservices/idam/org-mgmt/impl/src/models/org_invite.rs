use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Pending invitation to join an organization.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "org_invites"]
#[schema_name = "sesame_idam"]
pub struct OrgInvite {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.organizations(id) ON DELETE CASCADE"]
    pub org_id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub email: String,

    #[column_type = "VARCHAR(255)"]
    pub role: String,

    #[column_type = "VARCHAR(64)"]
    pub token: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub expires_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub accepted_at: chrono::DateTime<chrono::Utc>,
}
