//! Admin user lifecycle: create (idempotent by email), fetch, enable/disable.
//!
//! Stateless service (hauliage pattern): methods are generic over
//! `E: LifeExecutor`; the executor comes from `sesame_idam_database::db()`
//! at the controller edge.

use chrono::Utc;
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::user::{Column, Entity, UserModel, UserRecord};

/// Status values for the single `status` column.
pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_DISABLED: &str = "disabled";

/// Result of an idempotent create.
pub struct CreateOutcome {
    pub user: UserModel,
    /// False when the email already existed on this tenant (idempotent hit).
    pub created: bool,
}

pub struct UserAdminService;

impl UserAdminService {
    /// Find a user by tenant + email.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn find_by_email<E: LifeExecutor>(
        tenant_id: &str,
        email: &str,
        exec: &E,
    ) -> Result<Option<UserModel>, LifeError> {
        Entity::find()
            .filter(Column::TenantId.eq(tenant_id.to_string()))
            .filter(Column::Email.eq(email.to_string()))
            .find_one(exec)
    }

    /// Find a user by tenant + id.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn find_by_id<E: LifeExecutor>(
        tenant_id: &str,
        user_id: Uuid,
        exec: &E,
    ) -> Result<Option<UserModel>, LifeError> {
        Entity::find()
            .filter(Column::TenantId.eq(tenant_id.to_string()))
            .filter(Column::Id.eq(user_id))
            .find_one(exec)
    }

    /// Create a user (idempotent by email): returns the existing user when
    /// the email is already registered on this tenant.
    ///
    /// Admin-created users have no password (`password_hash` empty) until a
    /// password-set/magic-link flow assigns one — argon2 verification fails
    /// closed on the empty hash.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query/insert failure.
    pub fn create_idempotent<E: LifeExecutor>(
        tenant_id: &str,
        email: &str,
        email_confirmed: bool,
        exec: &E,
    ) -> Result<CreateOutcome, LifeError> {
        if let Some(existing) = Self::find_by_email(tenant_id, email, exec)? {
            return Ok(CreateOutcome {
                user: existing,
                created: false,
            });
        }

        let now = Utc::now();
        let id = Uuid::new_v4();
        let mut record = UserRecord::new();
        record
            .set_id(id)
            .set_email(email.to_string())
            .set_password_hash(String::new())
            .set_tenant_id(tenant_id.to_string())
            .set_status(STATUS_ACTIVE.to_string())
            .set_email_verified(email_confirmed)
            .set_phone(None)
            .set_phone_verified(false)
            .set_created_at(now)
            .set_updated_at(now);
        record
            .insert(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;

        let user = Self::find_by_id(tenant_id, id, exec)?
            .ok_or_else(|| LifeError::Other("user vanished after insert".into()))?;
        Ok(CreateOutcome {
            user,
            created: true,
        })
    }

    /// Set a user's status (enable/disable). Returns the updated user, or
    /// `None` when the user does not exist on this tenant.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query/update failure.
    pub fn set_status<E: LifeExecutor>(
        tenant_id: &str,
        user_id: Uuid,
        status: &str,
        exec: &E,
    ) -> Result<Option<UserModel>, LifeError> {
        let Some(user) = Self::find_by_id(tenant_id, user_id, exec)? else {
            return Ok(None);
        };

        let mut record = UserRecord::new();
        record
            .set_id(user.id)
            .set_email(user.email)
            .set_password_hash(user.password_hash)
            .set_tenant_id(user.tenant_id)
            .set_status(status.to_string())
            .set_email_verified(user.email_verified)
            .set_phone(user.phone)
            .set_phone_verified(user.phone_verified)
            .set_created_at(user.created_at)
            .set_updated_at(Utc::now());
        record
            .update(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;

        Self::find_by_id(tenant_id, user_id, exec)
    }
}

/// Build the admin `UserResponse` JSON shape shared by these endpoints.
///
/// `first_name`/`last_name`/`username`/`picture_url` are not stored on the
/// users table (profile fields live in `user_profiles`, owned by
/// identity-session-service) — returned empty per the entity-user wiki page.
#[must_use]
pub fn user_response_json(user: &UserModel) -> serde_json::Value {
    serde_json::json!({
        "user_id": user.id,
        "email": user.email,
        "email_confirmed": user.email_verified,
        "enabled": user.status == STATUS_ACTIVE,
        "locked": user.status == STATUS_DISABLED,
        "has_password": !user.password_hash.is_empty(),
        "first_name": "",
        "last_name": "",
        "username": "",
        "picture_url": null,
        "properties": {},
    })
}
