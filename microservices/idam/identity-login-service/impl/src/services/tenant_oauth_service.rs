//! Tenant OAuth metadata + K8s/env secret resolution.

use chrono::Utc;
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::tenant_oauth_provider::{
    Column as OauthColumn, Entity as OauthEntity, TenantOAuthProviderRecord,
};
use crate::services::oauth::config::ProviderCredentials;
use crate::services::tenant_service::TenantService;

/// Resolved credentials for an OAuth handshake.
#[derive(Debug, Clone)]
pub struct ResolvedTenantOAuth {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>,
    pub config_version: i32,
}

pub struct TenantOAuthService;

impl TenantOAuthService {
    /// Load enabled OAuth config for a registered tenant+provider.
    ///
    /// Secrets are read from the env var named in `secret_env_key` (K8s → pod).
    pub fn resolve<E: LifeExecutor>(
        tenant_slug: &str,
        provider: &str,
        exec: &E,
    ) -> Result<Option<ResolvedTenantOAuth>, LifeError> {
        // Caller must run `TenantService::require_active` first.
        let row = OauthEntity::find()
            .filter(OauthColumn::TenantSlug.eq(tenant_slug.to_string()))
            .filter(OauthColumn::Provider.eq(provider.to_string()))
            .filter(OauthColumn::Enabled.eq(true))
            .find_one(exec)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let client_id = if let Some(key) = row.client_id_env_key.as_deref() {
            std::env::var(key)
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| row.client_id.clone())
        } else {
            row.client_id.clone()
        };

        let client_secret = std::env::var(&row.secret_env_key)
            .map_err(|_| LifeError::Other("oauth_secret_unavailable".to_string()))?;

        if client_secret.trim().is_empty() {
            return Err(LifeError::Other("oauth_secret_unavailable".to_string()));
        }

        let redirect_uris = row
            .redirect_uris
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect();

        Ok(Some(ResolvedTenantOAuth {
            client_id,
            client_secret,
            redirect_uris,
            config_version: row.config_version,
        }))
    }

    /// Record a rotation event (secret updated in K8s out-of-band; version bumps here).
    pub fn record_rotation<E: LifeExecutor>(
        tenant_slug: &str,
        provider: &str,
        rotated_by: &str,
        exec: &E,
    ) -> Result<i32, LifeError> {
        let row = OauthEntity::find()
            .filter(OauthColumn::TenantSlug.eq(tenant_slug.to_string()))
            .filter(OauthColumn::Provider.eq(provider.to_string()))
            .find_one(exec)?
            .ok_or_else(|| LifeError::Other("oauth_config_not_found".to_string()))?;

        let new_version = row.config_version.saturating_add(1);
        let now = Utc::now();
        let mut record = TenantOAuthProviderRecord::new();
        record
            .set_id(row.id)
            .set_tenant_slug(row.tenant_slug.clone())
            .set_provider(row.provider.clone())
            .set_client_id(row.client_id.clone())
            .set_redirect_uris(row.redirect_uris.clone())
            .set_secret_env_key(row.secret_env_key.clone())
            .set_client_id_env_key(row.client_id_env_key.clone())
            .set_config_version(new_version)
            .set_last_rotated_at(Some(now))
            .set_last_rotated_by(Some(rotated_by.to_string()))
            .set_enabled(row.enabled)
            .set_created_at(row.created_at)
            .set_updated_at(now);
        record
            .update(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;
        Ok(new_version)
    }

    /// Upsert metadata row (platform admin / CLI). Does not store secret values.
    pub fn upsert_metadata<E: LifeExecutor>(
        tenant_slug: &str,
        provider: &str,
        client_id: &str,
        redirect_uris: &str,
        secret_env_key: &str,
        client_id_env_key: Option<&str>,
        exec: &E,
    ) -> Result<Uuid, LifeError> {
        match TenantService::find_by_slug(tenant_slug, exec) {
            Ok(Some(_)) => {}
            Ok(None) => {
                return Err(LifeError::Other("tenant_not_found".to_string()));
            }
            Err(e) => return Err(e),
        }

        let now = Utc::now();
        if let Some(existing) = OauthEntity::find()
            .filter(OauthColumn::TenantSlug.eq(tenant_slug.to_string()))
            .filter(OauthColumn::Provider.eq(provider.to_string()))
            .find_one(exec)?
        {
            let mut record = TenantOAuthProviderRecord::new();
            record
                .set_id(existing.id)
                .set_tenant_slug(existing.tenant_slug.clone())
                .set_provider(existing.provider.clone())
                .set_client_id(client_id.to_string())
                .set_redirect_uris(redirect_uris.to_string())
                .set_secret_env_key(secret_env_key.to_string())
                .set_client_id_env_key(client_id_env_key.map(ToString::to_string))
                .set_config_version(existing.config_version)
                .set_last_rotated_at(existing.last_rotated_at)
                .set_last_rotated_by(existing.last_rotated_by.clone())
                .set_enabled(existing.enabled)
                .set_created_at(existing.created_at)
                .set_updated_at(now);
            record
                .update(exec)
                .map_err(|e| LifeError::Other(e.to_string()))?;
            return Ok(existing.id);
        }

        let id = Uuid::new_v4();
        let mut record = TenantOAuthProviderRecord::new();
        record
            .set_id(id)
            .set_tenant_slug(tenant_slug.to_string())
            .set_provider(provider.to_string())
            .set_client_id(client_id.to_string())
            .set_redirect_uris(redirect_uris.to_string())
            .set_secret_env_key(secret_env_key.to_string())
            .set_client_id_env_key(client_id_env_key.map(ToString::to_string))
            .set_config_version(1)
            .set_last_rotated_at(None)
            .set_last_rotated_by(None)
            .set_enabled(true)
            .set_created_at(now)
            .set_updated_at(now);
        record
            .insert(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;
        Ok(id)
    }

    /// Serialize stored OAuth metadata (no secret values).
    pub fn metadata_json<E: LifeExecutor>(
        tenant_slug: &str,
        provider: &str,
        exec: &E,
    ) -> Result<Option<serde_json::Value>, LifeError> {
        let row = OauthEntity::find()
            .filter(OauthColumn::TenantSlug.eq(tenant_slug.to_string()))
            .filter(OauthColumn::Provider.eq(provider.to_string()))
            .find_one(exec)?;

        Ok(row.map(|row| {
            serde_json::json!({
                "provider": row.provider,
                "client_id": row.client_id,
                "redirect_uris": row.redirect_uris.split(',').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>(),
                "secret_env_key": row.secret_env_key,
                "client_id_env_key": row.client_id_env_key,
                "config_version": row.config_version,
                "enabled": row.enabled,
            })
        }))
    }
}

impl ResolvedTenantOAuth {
    #[must_use]
    pub fn credentials(&self) -> ProviderCredentials {
        ProviderCredentials {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
        }
    }

    #[must_use]
    pub fn redirect_uri_allowed(&self, redirect_uri: &str) -> bool {
        if self.redirect_uris.is_empty() {
            return super::oauth::config::is_dev_redirect_uri(redirect_uri);
        }
        self.redirect_uris.iter().any(|u| u == redirect_uri)
    }
}
