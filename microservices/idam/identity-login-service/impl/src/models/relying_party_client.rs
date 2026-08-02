use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Registered north-south client used to derive tenant and application context.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "relying_party_clients"]
#[schema_name = "sesame_idam"]
pub struct RelyingPartyClient {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    /// Globally unique public identifier supplied by relying parties.
    #[column_type = "VARCHAR(128)"]
    #[unique]
    pub client_id: String,

    /// Hard-isolation tenant selected by this registered client.
    #[column_type = "VARCHAR(64)"]
    #[foreign_key = "sesame_idam.tenants(slug) ON DELETE CASCADE"]
    pub tenant_slug: String,

    /// Authz application/portal context embedded in issued tokens.
    #[column_type = "VARCHAR(64)"]
    pub portal: String,

    /// Stable application identifier used by protocol and authorization policy.
    #[column_type = "VARCHAR(64)"]
    pub application_id: String,

    /// `public` or `confidential`; immutable after registration.
    #[column_type = "VARCHAR(32)"]
    pub client_type: String,

    /// `none`, `client_secret_basic`, or `client_secret_post`.
    #[column_type = "VARCHAR(32)"]
    pub token_endpoint_auth_method: String,

    /// Public clients must require the S256 PKCE method.
    #[column_type = "BOOLEAN"]
    pub pkce_s256_required: bool,

    /// `tenant` or `platform`; tenant routes can never create platform clients.
    #[column_type = "VARCHAR(32)"]
    pub authority_class: String,

    /// `active`, `disabled`, or `deleted`.
    #[column_type = "VARCHAR(32)"]
    pub status: String,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
