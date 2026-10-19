//! Admin user lifecycle: create (idempotent by email), fetch, enable/disable.
//!
//! Stateless service (BRRTRouter pattern): methods are generic over
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

/// The admin query's filters (GET /admin/users/query).
#[derive(Debug, Default, Clone)]
pub struct UserQuery {
    /// Case-insensitive substring of the email; `*` matches anything.
    pub email_pattern: Option<String>,
    pub email_confirmed: Option<bool>,
    /// true: active users only.
    pub enabled: Option<bool>,
    /// true: disabled users only.
    pub disabled: Option<bool>,
    pub created_after: Option<chrono::DateTime<Utc>>,
    pub created_before: Option<chrono::DateTime<Utc>>,
}

impl UserQuery {
    /// Does `u` pass every filter given?
    pub fn matches(&self, u: &UserModel) -> bool {
        let active = u.status == STATUS_ACTIVE;
        let pattern_ok = self.email_pattern.as_deref().map_or(true, |p| {
            let email = u.email.to_ascii_lowercase();
            let mut rest = email.as_str();
            let parts: Vec<String> = p
                .to_ascii_lowercase()
                .split('*')
                .map(str::to_string)
                .collect();
            if parts.len() == 1 {
                return email.contains(&parts[0]);
            }
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }
                if i == 0 && !rest.starts_with(part.as_str()) {
                    return false;
                }
                match rest.find(part.as_str()) {
                    Some(at) => rest = &rest[at + part.len()..],
                    None => return false,
                }
            }
            parts
                .last()
                .map_or(true, |l| l.is_empty() || email.ends_with(l.as_str()))
        });
        pattern_ok
            && self.email_confirmed.map_or(true, |c| u.email_verified == c)
            && self.enabled.map_or(true, |e| !e || active)
            && self.disabled.map_or(true, |d| !d || !active)
            && self.created_after.map_or(true, |t| u.created_at >= t)
            && self.created_before.map_or(true, |t| u.created_at < t)
    }
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

    /// One page of a tenant's users that pass `q`, by email, and how many pass in all.
    ///
    /// Filters and pages in memory over the tenant's rows: fine for the staff consoles' tenants (thousands of users);
    /// a tenant of millions wants this pushed into SQL.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn query<E: LifeExecutor>(
        tenant_id: &str,
        q: &UserQuery,
        page: usize,
        limit: usize,
        exec: &E,
    ) -> Result<(Vec<UserModel>, usize), LifeError> {
        let mut rows: Vec<UserModel> = Entity::find()
            .filter(Column::TenantId.eq(tenant_id.to_string()))
            .all(exec)?
            .into_iter()
            .filter(|u| q.matches(u))
            .collect();
        rows.sort_by(|a, b| a.email.cmp(&b.email));
        let total = rows.len();
        let start = page.saturating_sub(1).saturating_mul(limit);
        Ok((rows.into_iter().skip(start).take(limit).collect(), total))
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

#[cfg(test)]
mod query_tests {
    use super::*;

    fn user(email: &str, status: &str, verified: bool, day: u32) -> UserModel {
        let t = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 10, day, 12, 0, 0).unwrap();
        UserModel {
            id: Uuid::new_v4(),
            email: email.into(),
            password_hash: String::new(),
            tenant_id: "acme".into(),
            status: status.into(),
            email_verified: verified,
            phone: None,
            phone_verified: false,
            created_at: t,
            updated_at: t,
        }
    }

    #[test]
    fn email_patterns_are_substrings_or_wildcards_case_blind() {
        let u = user("Ann@PriceWhisperer.ai", STATUS_ACTIVE, true, 1);
        let q = |p: &str| UserQuery {
            email_pattern: Some(p.into()),
            ..Default::default()
        };
        assert!(q("pricewhisperer").matches(&u));
        assert!(q("ann@*").matches(&u));
        assert!(q("*@pricewhisperer.ai").matches(&u));
        assert!(q("a*whisperer*ai").matches(&u));
        assert!(!q("bob").matches(&u));
        assert!(!q("*@example.com").matches(&u));
    }

    #[test]
    fn status_confirmation_and_dates_filter() {
        let on = user("a@x.io", STATUS_ACTIVE, true, 5);
        let off = user("b@x.io", STATUS_DISABLED, false, 10);
        let enabled = UserQuery {
            enabled: Some(true),
            ..Default::default()
        };
        assert!(enabled.matches(&on) && !enabled.matches(&off));
        let disabled = UserQuery {
            disabled: Some(true),
            ..Default::default()
        };
        assert!(!disabled.matches(&on) && disabled.matches(&off));
        let confirmed = UserQuery {
            email_confirmed: Some(false),
            ..Default::default()
        };
        assert!(!confirmed.matches(&on) && confirmed.matches(&off));
        let after = UserQuery {
            created_after: Some(off.created_at),
            ..Default::default()
        };
        assert!(!after.matches(&on) && after.matches(&off));
        assert!(UserQuery::default().matches(&on));
    }
}
