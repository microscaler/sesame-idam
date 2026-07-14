use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Per-tenant social OAuth **metadata** (SaaS-of-SaaS platform layer).
///
/// Client secrets live in Kubernetes secrets / env vars — never in this table.
/// `secret_env_key` names the env var the runtime reads after rotation.
/// `config_version` increments on each rotation for audit and cache busting.
///
/// Distinct from org-mgmt `Application` (org-scoped OIDC clients for B2B
/// customers inside a tenant).
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "tenant_oauth_providers"]
#[schema_name = "sesame_idam"]
#[composite_unique = "tenant_slug, provider"]
pub struct TenantOAuthProvider {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    /// Matches `tenants.slug` / `X-Tenant-ID`.
    #[column_type = "VARCHAR(64)"]
    pub tenant_slug: String,

    /// `google` | `microsoft` | …
    #[column_type = "VARCHAR(32)"]
    pub provider: String,

    #[column_type = "TEXT"]
    pub client_id: String,

    /// Comma-separated allowed OAuth callback URIs for this tenant+provider.
    #[column_type = "TEXT"]
    pub redirect_uris: String,

    /// Env var name for client secret (K8s secret → pod env). Not the secret value.
    #[column_type = "VARCHAR(255)"]
    pub secret_env_key: String,

    /// Env var name for client id when also injected via K8s (optional override).
    #[column_type = "VARCHAR(255)"]
    #[nullable]
    pub client_id_env_key: Option<String>,

    #[column_type = "INTEGER"]
    pub config_version: i32,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    #[nullable]
    pub last_rotated_at: Option<chrono::DateTime<chrono::Utc>>,

    #[column_type = "VARCHAR(255)"]
    #[nullable]
    pub last_rotated_by: Option<String>,

    #[column_type = "BOOLEAN"]
    pub enabled: bool,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
