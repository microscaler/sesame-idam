//! User lookup and creation for authentication flows.
//!
//! Stateless service (BRRTRouter pattern): methods are generic over
//! `E: LifeExecutor` and receive the executor from the controller edge
//! (`sesame_idam_database::db()` in production, `MayPostgresExecutor` in
//! tests).

use chrono::Utc;
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::user::{Column, Entity, UserModel, UserRecord};
use crate::models::user_profile::UserProfileRecord;

/// User status for freshly registered accounts.
pub const STATUS_ACTIVE: &str = "active";

pub struct UserService;

impl UserService {
    /// Find a user by the tenant-bound OIDC subject.
    pub fn find_by_tenant_and_id<E: LifeExecutor>(
        tenant_id: &str,
        user_id: Uuid,
        exec: &E,
    ) -> Result<Option<UserModel>, LifeError> {
        Entity::find()
            .filter(Column::TenantId.eq(tenant_id.to_string()))
            .filter(Column::Id.eq(user_id))
            .find_one(exec)
    }

    /// Find a user by tenant + email (the tenant-scoped login identity).
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn find_by_tenant_and_email<E: LifeExecutor>(
        tenant_id: &str,
        email: &str,
        exec: &E,
    ) -> Result<Option<UserModel>, LifeError> {
        Entity::find()
            .filter(Column::TenantId.eq(tenant_id.to_string()))
            .filter(Column::Email.eq(email.to_string()))
            .find_one(exec)
    }

    /// Find a user by tenant + phone (the tenant-scoped SMS-login identity).
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn find_by_tenant_and_phone<E: LifeExecutor>(
        tenant_id: &str,
        phone: &str,
        exec: &E,
    ) -> Result<Option<UserModel>, LifeError> {
        Entity::find()
            .filter(Column::TenantId.eq(tenant_id.to_string()))
            .filter(Column::Phone.eq(phone.to_string()))
            .find_one(exec)
    }

    /// Create a new user with an already-hashed password.
    ///
    /// Returns the created user's id. The caller is responsible for checking
    /// email uniqueness beforehand (and the DB enforces
    /// `UNIQUE(tenant_id, email)` as a failsafe).
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on insert failure (including unique violations).
    pub fn create_user<E: LifeExecutor>(
        tenant_id: &str,
        email: &str,
        password_hash: &str,
        phone: Option<String>,
        exec: &E,
    ) -> Result<Uuid, LifeError> {
        Self::insert_user(tenant_id, email, password_hash, phone, false, exec)
    }

    /// Create a user provisioned via OAuth (email marked verified).
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on insert failure (including unique violations).
    pub fn create_oauth_user<E: LifeExecutor>(
        tenant_id: &str,
        email: &str,
        password_hash: &str,
        exec: &E,
    ) -> Result<Uuid, LifeError> {
        Self::insert_user(tenant_id, email, password_hash, None, true, exec)
    }

    fn insert_user<E: LifeExecutor>(
        tenant_id: &str,
        email: &str,
        password_hash: &str,
        phone: Option<String>,
        email_verified: bool,
        exec: &E,
    ) -> Result<Uuid, LifeError> {
        let now = Utc::now();
        let id = Uuid::new_v4();

        let mut record = UserRecord::new();
        record
            .set_id(id)
            .set_email(email.to_string())
            .set_password_hash(password_hash.to_string())
            .set_tenant_id(tenant_id.to_string())
            .set_status(STATUS_ACTIVE.to_string())
            .set_email_verified(email_verified)
            .set_phone(phone)
            .set_phone_verified(false)
            .set_created_at(now)
            .set_updated_at(now);

        record
            .insert(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;

        Ok(id)
    }

    /// Replace a user's password hash (password reset).
    ///
    /// Callers must have already proven control of the account (a consumed
    /// single-use reset token) and validated password strength.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] when the user is missing or the update fails.
    pub fn update_password_hash<E: LifeExecutor>(
        user_id: Uuid,
        password_hash: &str,
        exec: &E,
    ) -> Result<(), LifeError> {
        let user = Entity::find()
            .filter(Column::Id.eq(user_id))
            .find_one(exec)?
            .ok_or_else(|| LifeError::Other(format!("user {user_id} not found")))?;

        // Full-record rebuild (same pattern as TenantService::transition_status):
        // Lifeguard records are built explicitly, not converted from models.
        let mut record = UserRecord::new();
        record
            .set_id(user.id)
            .set_email(user.email.clone())
            .set_password_hash(password_hash.to_string())
            .set_tenant_id(user.tenant_id.clone())
            .set_status(user.status.clone())
            .set_email_verified(user.email_verified)
            .set_phone(user.phone.clone())
            .set_phone_verified(user.phone_verified)
            .set_created_at(user.created_at)
            .set_updated_at(Utc::now());
        record
            .update(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;
        Ok(())
    }

    /// Seed the user's profile row (display name) at registration.
    ///
    /// `user_profiles` holds the display name and avatar, separate from the
    /// auth `users` row. Best-effort at registration: callers log and
    /// continue if it fails, since the account already exists and the name
    /// can be set later via `PATCH /users/me`.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on insert failure.
    pub fn create_profile<E: LifeExecutor>(
        user_id: Uuid,
        first_name: Option<String>,
        last_name: Option<String>,
        exec: &E,
    ) -> Result<(), LifeError> {
        let now = Utc::now();
        let mut record = UserProfileRecord::new();
        record
            .set_id(Uuid::new_v4())
            .set_user_id(user_id)
            .set_first_name(first_name)
            .set_last_name(last_name)
            .set_avatar_url(None)
            .set_created_at(now)
            .set_updated_at(now);
        record
            .insert(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;
        Ok(())
    }
}
