use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Archived (revoked) API keys.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "archived_api_keys"]
#[schema_name = "sesame_idam"]
pub struct ArchivedApiKey {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "TEXT"]
    pub key_hash: String,

    #[column_type = "VARCHAR(16)"]
    pub key_prefix: String,

    #[column_type = "VARCHAR(255)"]
    pub name: String,

    #[column_type = "TEXT"]
    pub reason: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub archived_at: chrono::DateTime<chrono::Utc>,
}
