use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Application registered within an organization.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "applications"]
#[schema_name = "sesame_idam"]
pub struct Application {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.organizations(id) ON DELETE CASCADE"]
    pub org_id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub name: String,

    #[column_type = "VARCHAR(64)"]
    pub client_id: String,

    #[column_type = "TEXT"]
    #[nullable]
    pub client_secret: Option<String>,

    #[column_type = "TEXT"]
    #[nullable]
    pub redirect_uris: Option<String>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
