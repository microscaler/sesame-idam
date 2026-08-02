use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "relying_party_client_redirect_uris"]
#[schema_name = "sesame_idam"]
pub struct RelyingPartyClientRedirectUri {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.relying_party_clients(id) ON DELETE CASCADE"]
    pub relying_party_client_id: uuid::Uuid,

    #[column_type = "VARCHAR(32)"]
    pub kind: String,

    #[column_type = "TEXT"]
    pub uri: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,
}
