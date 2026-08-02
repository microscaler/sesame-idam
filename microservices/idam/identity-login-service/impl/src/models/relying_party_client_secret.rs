use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "relying_party_client_secrets"]
#[schema_name = "sesame_idam"]
pub struct RelyingPartyClientSecret {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.relying_party_clients(id) ON DELETE CASCADE"]
    pub relying_party_client_id: uuid::Uuid,

    #[column_type = "TEXT"]
    pub secret_hash: String,

    #[column_type = "VARCHAR(32)"]
    pub status: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub valid_from: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[nullable]
    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[nullable]
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}
