//! Organisation sign-in policy (ADR-008 §2, minimal): whether members must sign in with a second factor (`mfa`) or a
//! passkey (`passkey`), or neither (`none`, the default). Stored in `sesame_idam.org_auth_policies`; identity-login-
//! service enforces it at sign-in (a member without the required method enrols before any token is issued). Members
//! may read it; owners and admins set it.

use lifeguard::may_postgres::types::ToSql;
use lifeguard::{LifeError, LifeExecutor};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::services::org_lifecycle::{self, OrgLifecycleError};

/// Accepted values, weakest first.
pub const REQUIREMENTS: [&str; 3] = ["none", "mfa", "passkey"];

/// Why a policy request failed.
#[derive(Debug, PartialEq, Eq)]
pub enum PolicyError {
    /// Not an organisation of the caller's tenant, or the caller is not a member.
    NotFound,
    /// The caller is a member but not an owner or admin.
    Forbidden,
    /// Not one of [`REQUIREMENTS`].
    Invalid,
    Db(String),
}

impl PolicyError {
    #[must_use]
    pub fn status(&self) -> u16 {
        match self {
            Self::NotFound => 404,
            Self::Forbidden => 403,
            Self::Invalid => 400,
            Self::Db(_) => 500,
        }
    }

    #[must_use]
    pub fn body(&self) -> Value {
        let (error, message) = match self {
            Self::NotFound => ("not_found", "Organisation not found"),
            Self::Forbidden => (
                "forbidden",
                "Only organisation owners and admins can change the sign-in policy",
            ),
            Self::Invalid => (
                "invalid_request",
                "require must be one of none, mfa, passkey",
            ),
            Self::Db(_) => ("internal_error", "An unexpected error occurred"),
        };
        json!({ "error": error, "message": message })
    }
}

impl From<LifeError> for PolicyError {
    fn from(e: LifeError) -> Self {
        Self::Db(e.to_string())
    }
}

fn role<E: LifeExecutor>(
    exec: &E,
    tenant: &str,
    org: Uuid,
    caller: Uuid,
) -> Result<String, PolicyError> {
    match org_lifecycle::member_role(exec, tenant, org, caller) {
        Ok(Some(role)) => Ok(role),
        Ok(None) | Err(OrgLifecycleError::NotFound) => Err(PolicyError::NotFound),
        Err(e) => Err(PolicyError::Db(format!("{e:?}"))),
    }
}

fn read<E: LifeExecutor>(exec: &E, tenant: &str, org: Uuid) -> Result<Value, PolicyError> {
    let (t, o) = (tenant.to_string(), org.to_string());
    let rows = exec.query_all(
        "SELECT require_method, extract(epoch from updated_at)::bigint FROM sesame_idam.org_auth_policies \
         WHERE tenant_id = $1 AND org_id = $2::uuid",
        &[&t as &dyn ToSql, &o],
    )?;
    let (require, updated_at) = match rows.first() {
        Some(row) => (
            row.try_get::<_, String>(0)
                .map_err(|e| PolicyError::Db(e.to_string()))?,
            Some(
                row.try_get::<_, i64>(1)
                    .map_err(|e| PolicyError::Db(e.to_string()))?,
            ),
        ),
        None => ("none".to_string(), None),
    };
    Ok(json!({ "org_id": org.to_string(), "require": require, "updated_at": updated_at }))
}

/// The organisation's policy (members only).
///
/// # Errors
/// [`PolicyError::NotFound`] for a non-member or another tenant's organisation; database errors.
pub fn get(tenant: &str, org: Uuid, caller: Uuid) -> Result<Value, PolicyError> {
    let mut out = None;
    sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        out = Some(role(exec, tenant, org, caller).and_then(|_| read(exec, tenant, org)));
        Ok(())
    })?;
    out.unwrap_or_else(|| Err(PolicyError::Db("no result".into())))
}

/// Set the organisation's policy (owners and admins only). Returns the new policy.
///
/// # Errors
/// [`PolicyError::Invalid`], [`PolicyError::NotFound`], [`PolicyError::Forbidden`]; database errors.
pub fn set(tenant: &str, org: Uuid, caller: Uuid, require: &str) -> Result<Value, PolicyError> {
    let require = require.trim().to_ascii_lowercase();
    if !REQUIREMENTS.contains(&require.as_str()) {
        return Err(PolicyError::Invalid);
    }
    let mut out = Err(PolicyError::Db("no result".into()));
    let result = sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        let outcome = role(exec, tenant, org, caller).and_then(|role| {
            if role != "owner" && role != "admin" {
                return Err(PolicyError::Forbidden);
            }
            let (t, o, r, c) = (tenant.to_string(), org.to_string(), require.clone(), caller.to_string());
            exec.execute(
                "INSERT INTO sesame_idam.org_auth_policies (org_id, tenant_id, require_method, updated_by, updated_at) \
                 VALUES ($1::uuid, $2, $3, $4::uuid, now()) \
                 ON CONFLICT (org_id) DO UPDATE SET require_method = excluded.require_method, \
                   updated_by = excluded.updated_by, updated_at = now() \
                 WHERE sesame_idam.org_auth_policies.tenant_id = excluded.tenant_id",
                &[&o as &dyn ToSql, &t, &r, &c],
            )?;
            read(exec, tenant, org)
        });
        let failed = outcome.is_err();
        out = outcome;
        if failed {
            // Abort the transaction: nothing is written for a refused request.
            return Err(LifeError::Other("refused".into()));
        }
        Ok(())
    });
    if let (Err(e), Ok(_)) = (&result, &out) {
        return Err(PolicyError::Db(e.to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_map_to_statuses_and_documented_codes() {
        assert_eq!(PolicyError::NotFound.status(), 404);
        assert_eq!(PolicyError::Forbidden.status(), 403);
        assert_eq!(PolicyError::Invalid.status(), 400);
        assert_eq!(PolicyError::Invalid.body()["error"], "invalid_request");
        assert_eq!(
            PolicyError::Db("x".into()).body()["error"],
            "internal_error"
        );
    }

    #[test]
    fn requirements_are_the_three_documented_values() {
        assert_eq!(REQUIREMENTS, ["none", "mfa", "passkey"]);
    }
}
