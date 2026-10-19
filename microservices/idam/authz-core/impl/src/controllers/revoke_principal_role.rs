//! `DELETE /authz/principals/roles` — take a role from a user of the caller's own tenant (idempotent).
//!
//! Same caller rule as assign. Every assignment of the role to the user in the tenant goes. Takes effect at the user's
//! next refresh. Refuses to take the caller's last administrator role from themselves (a tenant nobody can administer
//! is recovered only by hand).
use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use serde_json::{json, Value};
use sesame_common::tenant_admin::forbidden;
use sesame_common::token_versioning::BumpReason;
use sesame_idam_authz_core_gen::handlers::revoke_principal_role::Request;

use super::role_admin_common::{admin, bad_request, internal, parse_uuid, record_change};
use crate::services::role_admin::{valid_role, RoleAdmin};

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
    let exec = sesame_idam_database::db();
    if user_id == a.user_id {
        let admin_roles = sesame_common::tenant_admin::admin_roles(
            std::env::var("SESAME_TENANT_ADMIN_ROLES").ok().as_deref(),
        );
        if admin_roles.contains(&role) {
            let others = match RoleAdmin::list(exec, &a.tenant, Some(user_id)) {
                Ok(rows) => rows
                    .iter()
                    .filter(|r| {
                        let n = r.role_name.to_ascii_lowercase();
                        n != role && admin_roles.contains(&n)
                    })
                    .count(),
                Err(e) => return internal("revoke_principal_role", &e),
            };
            if others == 0 {
                return forbidden("You cannot remove your own last administrator role");
            }
        }
    }
    match RoleAdmin::revoke(exec, &a.tenant, user_id, &role) {
        Ok(n) => {
            if n > 0 {
                record_change(&a, user_id, &role, "role_revoked", BumpReason::RoleRevoked);
            }
            HttpJson::new(204, json!(null))
        }
        Err(e) => internal("revoke_principal_role", &e),
    }
}
