//! User lookup and creation for authentication flows.
//!
//! Stateless service (hauliage pattern): methods are generic over
//! `E: LifeExecutor` and receive the executor from the controller edge
//! (`sesame_idam_database::db()` in production, `MayPostgresExecutor` in
//! tests).

use chrono::Utc;
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::user::{Column, Entity, UserModel, UserRecord};

/// User status for freshly registered accounts.
pub const STATUS_ACTIVE: &str = "active";

pub struct UserService;

impl UserService {
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
        Self::insert_user(
            tenant_id,
            email,
            password_hash,
            phone,
            false,
            exec,
        )
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
}
