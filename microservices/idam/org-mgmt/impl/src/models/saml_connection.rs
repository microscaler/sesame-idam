use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// SAML connection for an organization.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "saml_connections"]
#[schema_name = "sesame_idam"]
pub struct SamlConnection {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.organizations(id) ON DELETE CASCADE"]
    pub org_id: uuid::Uuid,

    #[column_type = "VARCHAR(255)"]
    pub issuer: String,

    #[column_type = "TEXT"]
    #[nullable]
    pub metadata_url: Option<String>,

    #[column_type = "TEXT"]
    #[nullable]
    pub sso_url: Option<String>,

    #[column_type = "TEXT"]
    #[nullable]
    pub signing_cert: Option<String>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
