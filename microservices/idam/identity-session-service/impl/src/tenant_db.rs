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

/// One statement: `sesame_idam.current_user_profile` sets the same tenant-only RLS context as
/// [`sesame_idam_database::with_pre_auth_tenant`] and reads both rows under it (migration
/// `identity-session-service/20261026120000_current_user_profile.sql` has the reasoning).
const CURRENT_USER_PROFILE_SQL: &str =
    "SELECT user_row, profile_row FROM sesame_idam.current_user_profile($1, $2)";

/// Load user + profile under the tenant-only RLS context, in one round trip.
///
/// Before 26 Oct 2026 this was a Lifeguard contextual transaction: BEGIN, `rls_set_session`, the users row, the
/// profile row, COMMIT - five round trips (~25 ms in pp) for ~0.1 ms of database work. The function runs the same
/// `rls_set_session` call with the same values, then the same two reads, in one autocommit statement; RLS still
/// filters the users read (SECURITY INVOKER, FORCE ROW LEVEL SECURITY) and the context ends with the statement.
///
/// # Errors
///
/// Returns [`LifeError`] on database failure, or when a returned row cannot be read back into its model.
pub fn load_profile(tenant_id: &str, user_id: Uuid) -> Result<ProfileLoad, LifeError> {
    use lifeguard::LifeExecutor;

    // Same guard as the transaction path: a blank tenant never reaches the database.
    if tenant_id.trim().is_empty() {
        return Err(LifeError::Other(
            "pre-auth tenant id is required".to_string(),
        ));
    }
    let values = sea_query::Values(vec![
        sea_query::Value::String(Some(tenant_id.to_string())),
        sea_query::Value::Uuid(Some(user_id)),
    ]);
    let rows = sesame_idam_database::db().query_all_values(CURRENT_USER_PROFILE_SQL, &values)?;
    let Some(row) = rows.first() else {
        return Ok(ProfileLoad::NotFound);
    };
    let user_row: serde_json::Value = row
        .try_get(0)
        .map_err(|e| LifeError::Other(format!("current_user_profile user_row: {e}")))?;
    let profile_row: Option<serde_json::Value> = row
        .try_get(1)
        .map_err(|e| LifeError::Other(format!("current_user_profile profile_row: {e}")))?;
    profile_load_from_json(user_row, profile_row)
}

/// Rebuild the models from the function's jsonb rows (`to_jsonb` of each table row: column names as keys).
fn profile_load_from_json(
    user_row: serde_json::Value,
    profile_row: Option<serde_json::Value>,
) -> Result<ProfileLoad, LifeError> {
    let user: UserModel = serde_json::from_value(user_row)
        .map_err(|e| LifeError::Other(format!("current_user_profile users row: {e}")))?;
    let profile = match profile_row {
        None | Some(serde_json::Value::Null) => None,
        Some(row) => Some(
            serde_json::from_value::<UserProfileModel>(row).map_err(|e| {
                LifeError::Other(format!("current_user_profile user_profiles row: {e}"))
            })?,
        ),
    };
    Ok(ProfileLoad::Found(user, profile))
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

#[cfg(test)]
mod tests {
    use super::*;

    // What PostgreSQL 17 returns for to_jsonb(u) / to_jsonb(p) (pp, 26 Oct 2026; values made up).
    fn user_row() -> serde_json::Value {
        serde_json::json!({
            "id": "078bcf47-31e4-46fe-9cb7-e77aca01b267",
            "email": "trader@example.com",
            "password_hash": "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA",
            "tenant_id": "pricewhisperer",
            "status": "active",
            "email_verified": true,
            "phone": null,
            "phone_verified": false,
            "created_at": "2026-10-12T09:15:02.123456+00:00",
            "updated_at": "2026-10-20T17:01:59.5+00:00"
        })
    }

    #[test]
    fn rows_rebuild_the_models() {
        let profile = serde_json::json!({
            "id": "5f0c1f0e-8f5e-4c2b-9d7e-0a1b2c3d4e5f",
            "user_id": "078bcf47-31e4-46fe-9cb7-e77aca01b267",
            "first_name": "Ada",
            "last_name": null,
            "avatar_url": null,
            "created_at": "2026-10-12T09:15:02+00:00",
            "updated_at": "2026-10-12T09:15:02+00:00"
        });
        let ProfileLoad::Found(user, Some(p)) =
            profile_load_from_json(user_row(), Some(profile)).expect("rows decode")
        else {
            panic!("expected a user and a profile");
        };
        assert_eq!(user.id.to_string(), "078bcf47-31e4-46fe-9cb7-e77aca01b267");
        assert_eq!(user.status, "active");
        assert_eq!(user.tenant_id, "pricewhisperer");
        assert_eq!(
            user.created_at.to_rfc3339(),
            "2026-10-12T09:15:02.123456+00:00"
        );
        assert_eq!(p.first_name.as_deref(), Some("Ada"));
        assert!(p.last_name.is_none());
    }

    #[test]
    fn a_missing_profile_is_none() {
        for absent in [None, Some(serde_json::Value::Null)] {
            let ProfileLoad::Found(_, profile) =
                profile_load_from_json(user_row(), absent).expect("rows decode")
            else {
                panic!("expected a user");
            };
            assert!(profile.is_none());
        }
    }

    #[test]
    fn a_malformed_row_is_an_error_not_a_user() {
        assert!(profile_load_from_json(serde_json::json!({"id": "x"}), None).is_err());
    }

    #[test]
    fn a_blank_tenant_is_refused_before_the_database() {
        assert!(load_profile("  ", Uuid::nil()).is_err());
    }
}
