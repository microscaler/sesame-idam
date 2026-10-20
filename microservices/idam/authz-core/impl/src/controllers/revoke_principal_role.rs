//! `DELETE /authz/principals/roles` — take a role from a user of the caller's own tenant (idempotent).
//!
//! Same caller rule as assign, and the same role policy (sesame_common::role_policy): reserved roles are not touched
//! here, and an administrator role is taken away only by someone who holds it (or owner). Every assignment of the role
//! to the user in the tenant goes, and the user's tokens are made stale at once (subject version bump), so the role
//! stops working on their next request, not their next refresh.
//!
//! A tenant always keeps an administrator: removing an administrator role is refused when no other active user would
//! hold one tenant-wide. The count and the delete run in one transaction under a per-tenant lock, so two
//! administrators removing each other at the same moment cannot both succeed.
use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use serde_json::{json, Value};
use sesame_common::role_policy::RolePolicy;
use sesame_common::tenant_admin::forbidden;
use sesame_common::token_versioning::BumpReason;
use sesame_idam_authz_core_gen::handlers::revoke_principal_role::Request;

use super::role_admin_common::{
    admin, bad_request, internal, invalidate_tokens, parse_uuid, record_change, refused,
};
use crate::services::role_admin::{valid_role, RoleAdmin};

/// What the transaction decided.
enum Revoked {
    Removed(u64),
    LastAdmin,
}

#[handler(RevokePrincipalRoleController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<Value> {
    let d = &req.data;
    let a = match admin(&req.jwt_claims, &[Some(d.x_tenant_id.as_str())]) {
        Ok(a) => a,
        Err(r) => return r,
    };
    let user_id = match parse_uuid(&d.user_id, "user_id") {
        Ok(u) => u,
        Err(r) => return r,
    };
    let role = d.role.trim().to_ascii_lowercase();
    if !valid_role(&role) {
        return bad_request("role is 1-64 of lowercase letters, digits, '.', '_' and '-'");
    }
    let policy = RolePolicy::configured();
    if let Err(r) = policy.may_change(&a.roles, &role) {
        return refused(&r);
    }
    let is_admin_role = policy.is_admin_role(&role);
    // users is under forced RLS (the last-administrator count reads it): the tenant's session, one transaction.
    let outcome = sesame_idam_database::with_pre_auth_tenant(&a.tenant, |exec| {
        if is_admin_role {
            RoleAdmin::lock_admins(exec, &a.tenant)?;
            if RoleAdmin::admins_remaining(exec, &a.tenant, &policy.admin, user_id, &role)? == 0 {
                return Ok(Revoked::LastAdmin);
            }
        }
        RoleAdmin::revoke(exec, &a.tenant, user_id, &role).map(Revoked::Removed)
    });
    match outcome {
        Ok(Revoked::LastAdmin) => forbidden(
            "That would leave the tenant without an administrator: give another active user an administrator role first",
        ),
        // Nothing to remove (or a retry after a failed bump): make the user's tokens stale anyway, so "retry" is a
        // real remedy; a bump only costs them a silent refresh.
        Ok(Revoked::Removed(0)) => {
            if let Err(e) = invalidate_tokens(user_id) {
                tracing::warn!(error = %e, %user_id, "revoke_principal_role: token version bump failed (nothing removed)");
            }
            HttpJson::new(204, json!(null))
        }
        Ok(Revoked::Removed(_)) => {
            record_change(&a, user_id, &role, "role_revoked", BumpReason::RoleRevoked);
            match invalidate_tokens(user_id) {
                Ok(_) => HttpJson::new(204, json!(null)),
                Err(e) => {
                    tracing::error!(error = %e, %user_id, "revoke_principal_role: role removed but token version bump failed");
                    HttpJson::new(
                        503,
                        json!({
                            "error": "security_state_unavailable",
                            "error_description": "The role was removed, but the user's current sign-in could not be cut short: it keeps the role until its access token expires (minutes). Retry to cut it short.",
                        }),
                    )
                }
            }
        }
        Err(e) => internal("revoke_principal_role", &e),
    }
}
