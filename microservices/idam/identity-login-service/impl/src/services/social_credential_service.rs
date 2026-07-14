//! Social credential persistence and lookup for OAuth login.

use chrono::Utc;
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::social_credential::{
    Column as ScColumn, Entity as ScEntity, SocialCredentialRecord,
};
use crate::models::user::{Column as UserColumn, Entity as UserEntity, UserModel};

pub struct SocialCredentialService;

impl SocialCredentialService {
    /// Find a tenant-scoped user linked to a provider account.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn find_user_by_provider<E: LifeExecutor>(
        tenant_id: &str,
        provider: &str,
        provider_user_id: &str,
        exec: &E,
    ) -> Result<Option<UserModel>, LifeError> {
        let credentials = ScEntity::find()
            .filter(ScColumn::Provider.eq(provider.to_string()))
            .filter(ScColumn::ProviderUserId.eq(provider_user_id.to_string()))
            .all(exec)?;

        for cred in credentials {
            if let Ok(Some(user)) = UserEntity::find()
                .filter(UserColumn::Id.eq(cred.user_id))
                .find_one(exec)
            {
                if user.tenant_id == tenant_id {
                    return Ok(Some(user));
                }
            }
        }
        Ok(None)
    }

    /// Link a provider account to a user (idempotent insert).
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on insert failure.
    pub fn link_provider<E: LifeExecutor>(
        user_id: Uuid,
        provider: &str,
        provider_user_id: &str,
        exec: &E,
    ) -> Result<(), LifeError> {
        let existing = ScEntity::find()
            .filter(ScColumn::UserId.eq(user_id))
            .filter(ScColumn::Provider.eq(provider.to_string()))
            .find_one(exec)?;
        if existing.is_some() {
            return Ok(());
        }

        let now = Utc::now();
        let mut record = SocialCredentialRecord::new();
        record
            .set_id(Uuid::new_v4())
            .set_user_id(user_id)
            .set_provider(provider.to_string())
            .set_provider_user_id(provider_user_id.to_string())
            .set_access_token(None)
            .set_refresh_token(None)
            .set_created_at(now)
            .set_updated_at(now);

        record
            .insert(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;
        Ok(())
    }
}
