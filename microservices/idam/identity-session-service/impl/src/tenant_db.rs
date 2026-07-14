//! Tenant-scoped profile DB access under forced `users` RLS.
//!
//! JWT-authenticated handlers validate principal + tenant at the edge; reads and
//! writes against `sesame_idam.users` must pin `sesame.tenant_id` via
//! [`sesame_idam_database::with_pre_auth_tenant`] so Lifeguard queries see rows.

use lifeguard::LifeError;
use uuid::Uuid;

use crate::models::user::UserModel;
use crate::models::user_profile::UserProfileModel;
use crate::services::profile_service::{ProfileService, ProfileUpdate};

/// Result of loading a tenant-scoped user + optional profile row.
pub enum ProfileLoad {
    /// User exists in the authenticated tenant.
    Found(UserModel, Option<UserProfileModel>),
    /// No row for this principal in the tenant (401 at controller edge).
    NotFound,
}

/// Load user + profile inside a tenant-only RLS transaction.
///
/// # Errors
///
/// Returns [`LifeError`] on database failure.
pub fn load_profile(tenant_id: &str, user_id: Uuid) -> Result<ProfileLoad, LifeError> {
    sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| {
        let Some(user) = ProfileService::find_user(tenant_id, user_id, exec)? else {
            return Ok(ProfileLoad::NotFound);
        };
        let profile = ProfileService::find_profile(user_id, exec)?;
        Ok(ProfileLoad::Found(user, profile))
    })
}

/// Apply a profile patch for the authenticated user within tenant RLS scope.
///
/// # Errors
///
/// Returns [`LifeError`] on database failure.
pub fn patch_profile(
    tenant_id: &str,
    user_id: Uuid,
    update: &ProfileUpdate,
) -> Result<ProfileLoad, LifeError> {
    sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| {
        let Some(user) = ProfileService::find_user(tenant_id, user_id, exec)? else {
            return Ok(ProfileLoad::NotFound);
        };
        let profile = if update.is_empty() {
            ProfileService::find_profile(user_id, exec)?
        } else {
            Some(ProfileService::upsert_profile(user_id, update, exec)?)
        };
        Ok(ProfileLoad::Found(user, profile))
    })
}
