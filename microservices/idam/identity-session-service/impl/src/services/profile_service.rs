//! Current-user profile resolution for `/identity/me` (and OIDC userinfo).
//!
//! Stateless service (hauliage pattern): methods are generic over
//! `E: LifeExecutor`; the executor comes from `sesame_idam_database::db()`
//! at the controller edge.

use chrono::Utc;
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::user::{Column as UserColumn, Entity as UserEntity, UserModel};
use crate::models::user_profile::{
    Column as ProfileColumn, Entity as ProfileEntity, UserProfileModel, UserProfileRecord,
};

/// Fields accepted by a profile update (PATCH /identity/me).
#[derive(Debug, Default, Clone)]
pub struct ProfileUpdate {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar_url: Option<String>,
}

impl ProfileUpdate {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.first_name.is_none() && self.last_name.is_none() && self.avatar_url.is_none()
    }
}

pub struct ProfileService;

impl ProfileService {
    /// Fetch a user by id, tenant-scoped (returns `None` for other tenants).
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn find_user<E: LifeExecutor>(
        tenant_id: &str,
        user_id: Uuid,
        exec: &E,
    ) -> Result<Option<UserModel>, LifeError> {
        UserEntity::find()
            .filter(UserColumn::TenantId.eq(tenant_id.to_string()))
            .filter(UserColumn::Id.eq(user_id))
            .find_one(exec)
    }

    /// Fetch the extended profile row for a user, if one exists.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn find_profile<E: LifeExecutor>(
        user_id: Uuid,
        exec: &E,
    ) -> Result<Option<UserProfileModel>, LifeError> {
        ProfileEntity::find()
            .filter(ProfileColumn::UserId.eq(user_id))
            .find_one(exec)
    }

    /// Apply a partial profile update, creating the profile row on first
    /// write. Returns the resulting profile.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query/insert/update failure.
    pub fn upsert_profile<E: LifeExecutor>(
        user_id: Uuid,
        update: &ProfileUpdate,
        exec: &E,
    ) -> Result<UserProfileModel, LifeError> {
        let now = Utc::now();

        if let Some(existing) = Self::find_profile(user_id, exec)? {
            let mut record = UserProfileRecord::new();
            record
                .set_id(existing.id)
                .set_user_id(existing.user_id)
                .set_first_name(update.first_name.clone().or(existing.first_name.clone()))
                .set_last_name(update.last_name.clone().or(existing.last_name.clone()))
                .set_avatar_url(update.avatar_url.clone().or(existing.avatar_url.clone()))
                .set_created_at(existing.created_at)
                .set_updated_at(now);
            record
                .update(exec)
                .map_err(|e| LifeError::Other(e.to_string()))?;
        } else {
            let mut record = UserProfileRecord::new();
            record
                .set_id(Uuid::new_v4())
                .set_user_id(user_id)
                .set_first_name(update.first_name.clone())
                .set_last_name(update.last_name.clone())
                .set_avatar_url(update.avatar_url.clone())
                .set_created_at(now)
                .set_updated_at(now);
            record
                .insert(exec)
                .map_err(|e| LifeError::Other(e.to_string()))?;
        }

        Self::find_profile(user_id, exec)?
            .ok_or_else(|| LifeError::Other("profile vanished after upsert".into()))
    }
}
