//! Role assignments, administered (assign, revoke, list) for one tenant: the caller's.
//!
//! Assignments live in `sesame_idam.role_assignments` and are tenant-wide (no application column): a token carries
//! every role its user holds in its tenant (principal_effective, on login and on every refresh). Assigning is
//! idempotent (a role the user already holds changes nothing) and revoking removes every assignment of that role to
//! the user in the tenant. Role names are not a catalogue here; they are checked for shape only, so a product names
//! its own roles (PriceWhisperer: platform_admin, crm.editor, ...).
use chrono::Utc;
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::role_assignment::{
    Column as RoleColumn, Entity as RoleEntity, RoleAssignmentModel, RoleAssignmentRecord,
};

/// `resource_type` of an assignment made here: the whole tenant, or one organisation.
pub const RESOURCE_TENANT: &str = "tenant";
pub const RESOURCE_ORGANIZATION: &str = "organization";

/// A role name: 1-64 of lowercase letters, digits, '.', '_' and '-', starting with a letter or digit.
pub fn valid_role(role: &str) -> bool {
    let b = role.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'_' | b'-')
        })
}

pub struct RoleAdmin;

/// What an assign did.
#[derive(Debug, PartialEq, Eq)]
pub enum Assigned {
    Created,
    AlreadyHeld,
    NoSuchUser,
}

impl RoleAdmin {
    /// Is `user_id` a user of `tenant`?
    pub fn user_in_tenant<E: LifeExecutor>(exec: &E, tenant: &str, user_id: Uuid) -> bool {
        exec.query_one_values(
            "SELECT 1 FROM sesame_idam.users WHERE id = $1 AND tenant_id = $2",
            &sea_query::Values(vec![user_id.into(), tenant.into()]),
        )
        .is_ok()
    }

    /// Is `org_id` an organisation of `tenant`? (organizations has no RLS: the tenant is filtered here.)
    pub fn org_in_tenant<E: LifeExecutor>(exec: &E, tenant: &str, org_id: Uuid) -> bool {
        exec.query_one_values(
            "SELECT 1 FROM sesame_idam.organizations WHERE id = $1 AND tenant_id = $2",
            &sea_query::Values(vec![org_id.into(), tenant.into()]),
        )
        .is_ok()
    }

    /// Serialise administrator changes in `tenant` for the rest of the transaction (two admins revoking each other at
    /// the same moment must not both see the other as "still there").
    ///
    /// # Errors
    /// [`LifeError`] when the lock cannot be taken.
    pub fn lock_admins<E: LifeExecutor>(exec: &E, tenant: &str) -> Result<(), LifeError> {
        exec.query_one_values(
            "SELECT 1 FROM (SELECT pg_advisory_xact_lock(hashtext($1))) AS locked",
            &sea_query::Values(vec![format!("sesame-admins:{tenant}").into()]),
        )
        .map(|_| ())
    }

    /// How many active users of `tenant` hold a tenant-wide administrator role, not counting the assignment
    /// (`user_id`, `role`) about to be removed. Needs the tenant's RLS session (it reads users).
    ///
    /// # Errors
    /// [`LifeError`] on a failed query.
    pub fn admins_remaining<E: LifeExecutor>(
        exec: &E,
        tenant: &str,
        admin_roles: &[String],
        user_id: Uuid,
        role: &str,
    ) -> Result<i64, LifeError> {
        if admin_roles.is_empty() {
            return Ok(0);
        }
        let mut values: Vec<sea_query::Value> = vec![
            tenant.into(),
            user_id.into(),
            role.to_ascii_lowercase().into(),
        ];
        let mut marks = Vec::new();
        for r in admin_roles {
            values.push(r.to_ascii_lowercase().into());
            marks.push(format!("${}", values.len()));
        }
        let sql = format!(
            "SELECT count(DISTINCT ra.principal_id)::bigint FROM sesame_idam.role_assignments ra \
             JOIN sesame_idam.users u ON u.id = ra.principal_id AND u.tenant_id = ra.tenant_id \
             WHERE ra.tenant_id = $1 AND ra.resource_id IS NULL AND u.status = 'active' \
             AND lower(ra.role_name) IN ({}) \
             AND NOT (ra.principal_id = $2 AND lower(ra.role_name) = $3)",
            marks.join(", ")
        );
        let row = exec.query_one_values(&sql, &sea_query::Values(values))?;
        row.try_get::<usize, i64>(0)
            .map_err(|e| LifeError::Other(e.to_string()))
    }

    /// Every assignment in `tenant`, or one user's.
    ///
    /// # Errors
    /// [`LifeError`] on a failed query.
    pub fn list<E: LifeExecutor>(
        exec: &E,
        tenant: &str,
        user_id: Option<Uuid>,
    ) -> Result<Vec<RoleAssignmentModel>, LifeError> {
        let mut q = RoleEntity::find().filter(RoleColumn::TenantId.eq(tenant.to_string()));
        if let Some(u) = user_id {
            q = q.filter(RoleColumn::PrincipalId.eq(u));
        }
        let mut rows = q.all(exec)?;
        rows.sort_by(|a, b| {
            (a.principal_id, &a.role_name, a.created_at).cmp(&(
                b.principal_id,
                &b.role_name,
                b.created_at,
            ))
        });
        Ok(rows)
    }

    /// Give `user_id` the role `role` in `tenant` (optionally within one organisation).
    ///
    /// # Errors
    /// [`LifeError`] on a failed query or insert.
    pub fn assign<E: LifeExecutor>(
        exec: &E,
        tenant: &str,
        user_id: Uuid,
        role: &str,
        org_id: Option<Uuid>,
    ) -> Result<Assigned, LifeError> {
        if !Self::user_in_tenant(exec, tenant, user_id) {
            return Ok(Assigned::NoSuchUser);
        }
        let held = Self::list(exec, tenant, Some(user_id))?
            .into_iter()
            .any(|a| a.role_name.eq_ignore_ascii_case(role) && a.resource_id == org_id);
        if held {
            return Ok(Assigned::AlreadyHeld);
        }
        let now = Utc::now();
        let mut record = RoleAssignmentRecord::new();
        record
            .set_id(Uuid::new_v4())
            .set_principal_id(user_id)
            .set_role_name(role.to_string())
            .set_resource_type(
                if org_id.is_some() {
                    RESOURCE_ORGANIZATION
                } else {
                    RESOURCE_TENANT
                }
                .to_string(),
            )
            .set_resource_id(org_id)
            .set_tenant_id(tenant.to_string())
            .set_created_at(now)
            .set_updated_at(now);
        match record.insert(exec) {
            Ok(_) => Ok(Assigned::Created),
            // role_assignments_one_per_scope: a concurrent grant of the same role won the race
            Err(e) if e.to_string().contains("role_assignments_one_per_scope") => {
                Ok(Assigned::AlreadyHeld)
            }
            Err(e) => Err(LifeError::Other(e.to_string())),
        }
    }

    /// Remove every assignment of `role` to `user_id` in `tenant`; the number removed.
    ///
    /// # Errors
    /// [`LifeError`] on a failed delete.
    pub fn revoke<E: LifeExecutor>(
        exec: &E,
        tenant: &str,
        user_id: Uuid,
        role: &str,
    ) -> Result<u64, LifeError> {
        exec.execute_values(
            "DELETE FROM sesame_idam.role_assignments WHERE tenant_id = $1 AND principal_id = $2 AND lower(role_name) = $3",
            &sea_query::Values(vec![tenant.into(), user_id.into(), role.to_ascii_lowercase().into()]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::valid_role;

    #[test]
    fn role_names_have_a_safe_shape() {
        for ok in ["tenant_admin", "crm.editor", "platform-admin", "r2", "a"] {
            assert!(valid_role(ok), "{ok}");
        }
        for bad in [
            "",
            "Admin",
            "-x",
            ".x",
            "a b",
            "rôle",
            "x;drop",
            &"a".repeat(65),
        ] {
            assert!(!valid_role(bad), "{bad}");
        }
    }
}
