//! API key creation and validation (M2M keys / service accounts).
//!
//! Key material: `sk_<64 hex chars>`. Only a SHA-256 hash is stored
//! (`api_keys.key_hash`); the plaintext key is returned exactly once from
//! the create endpoint. `key_prefix` keeps the first characters for display
//! ("`sk_ab12`…"). Validation is a tenant-scoped hash lookup plus
//! active/expiry checks.

use chrono::{Duration, Utc};
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::api_key::{ApiKeyModel, ApiKeyRecord, Column, Entity};

/// Prefix identifying sesame-idam secret keys.
pub const KEY_PREFIX: &str = "sk_";

/// Length of the stored display prefix (fits `key_prefix VARCHAR(16)`).
pub const DISPLAY_PREFIX_LEN: usize = 10;

/// Outcome of a validation lookup.
#[derive(Debug)]
pub enum ValidationOutcome {
    /// Key exists, is active, matches the requested type, and is not expired.
    Valid(ApiKeyModel),
    /// Key exists but is expired (spec reports `valid: false, is_expired: true`).
    Expired(ApiKeyModel),
    /// Unknown, inactive, wrong tenant, or wrong key type.
    Invalid,
}

/// A freshly created key: the model plus the plaintext secret (returned once).
pub struct CreatedKey {
    pub model: ApiKeyModel,
    pub plaintext: String,
}

/// Parameters for key creation.
pub struct NewApiKey {
    pub tenant_id: String,
    pub name: String,
    pub user_id: Option<Uuid>,
    pub org_id: Option<Uuid>,
    /// JSON-encoded list of permission strings.
    pub permissions: Option<Vec<String>>,
    pub expires_in_days: Option<i64>,
}

pub struct ApiKeyService;

impl ApiKeyService {
    /// SHA-256 hex digest of a plaintext key.
    #[must_use]
    pub fn hash_key(plaintext: &str) -> String {
        hex::encode(Sha256::digest(plaintext.as_bytes()))
    }

    /// Generate a new `sk_`-prefixed secret (32 random bytes, hex-encoded).
    #[must_use]
    pub fn generate_plaintext() -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        format!("{KEY_PREFIX}{}", hex::encode(bytes))
    }

    /// Create and persist a new API key. Returns the model + plaintext.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on insert failure.
    pub fn create<E: LifeExecutor>(params: NewApiKey, exec: &E) -> Result<CreatedKey, LifeError> {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let plaintext = Self::generate_plaintext();
        let display_prefix: String = plaintext.chars().take(DISPLAY_PREFIX_LEN).collect();
        let expires_at = params.expires_in_days.map(|d| now + Duration::days(d));

        let permissions_json = match &params.permissions {
            Some(perms) => {
                Some(serde_json::to_string(perms).map_err(|e| LifeError::Other(e.to_string()))?)
            }
            None => None,
        };

        let mut record = ApiKeyRecord::new();
        record
            .set_id(id)
            .set_key_hash(Self::hash_key(&plaintext))
            .set_key_prefix(display_prefix)
            .set_name(params.name)
            .set_tenant_id(params.tenant_id)
            .set_user_id(params.user_id)
            .set_org_id(params.org_id)
            .set_permissions(permissions_json)
            .set_expires_at(expires_at)
            .set_active(true)
            .set_created_at(now)
            .set_updated_at(now);

        record
            .insert(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;

        let model = Entity::find()
            .filter(Column::Id.eq(id))
            .find_one(exec)?
            .ok_or_else(|| LifeError::Other("api key vanished after insert".into()))?;

        Ok(CreatedKey { model, plaintext })
    }

    /// Validate a plaintext key within a tenant.
    ///
    /// `key_type`: `"any"` (default), `"personal"` (user-bound keys only),
    /// or `"org"` (org-bound keys only) — a mismatch is `Invalid` per spec.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn validate<E: LifeExecutor>(
        tenant_id: &str,
        plaintext: &str,
        key_type: &str,
        exec: &E,
    ) -> Result<ValidationOutcome, LifeError> {
        let hash = Self::hash_key(plaintext);

        let Some(key) = Entity::find()
            .filter(Column::TenantId.eq(tenant_id.to_string()))
            .filter(Column::KeyHash.eq(hash))
            .find_one(exec)?
        else {
            return Ok(ValidationOutcome::Invalid);
        };

        if !key.active {
            return Ok(ValidationOutcome::Invalid);
        }

        let type_matches = match key_type {
            "personal" => key.user_id.is_some(),
            "org" => key.org_id.is_some(),
            _ => true,
        };
        if !type_matches {
            return Ok(ValidationOutcome::Invalid);
        }

        if let Some(expires_at) = key.expires_at {
            if expires_at <= Utc::now() {
                return Ok(ValidationOutcome::Expired(key));
            }
        }

        Ok(ValidationOutcome::Valid(key))
    }
}

/// Decode the stored JSON permissions column into a list.
#[must_use]
pub fn decode_permissions(key: &ApiKeyModel) -> Vec<String> {
    key.permissions
        .as_deref()
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_are_unique_and_prefixed() {
        let a = ApiKeyService::generate_plaintext();
        let b = ApiKeyService::generate_plaintext();
        assert_ne!(a, b);
        assert!(a.starts_with(KEY_PREFIX));
        assert_eq!(a.len(), KEY_PREFIX.len() + 64);
    }

    #[test]
    fn hash_is_deterministic_and_not_reversible_shaped() {
        let key = "sk_test";
        assert_eq!(ApiKeyService::hash_key(key), ApiKeyService::hash_key(key));
        assert_eq!(ApiKeyService::hash_key(key).len(), 64);
        assert_ne!(ApiKeyService::hash_key(key), key);
    }
}
