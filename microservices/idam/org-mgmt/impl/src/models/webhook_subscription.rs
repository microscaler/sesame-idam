use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Webhook subscription for an organization.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "webhook_subscriptions"]
#[schema_name = "sesame_idam"]
pub struct WebhookSubscription {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.organizations(id) ON DELETE CASCADE"]
    pub org_id: uuid::Uuid,

    #[column_type = "TEXT"]
    pub url: String,

    #[column_type = "TEXT"]
    pub events: String,

    #[column_type = "TEXT"]
    #[nullable]
    pub secret: Option<String>,

    #[column_type = "BOOLEAN"]
    pub active: bool,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
