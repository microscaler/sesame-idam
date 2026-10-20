//! `POST /authz/principals/roles` — give a user of the caller's own tenant a role (idempotent).
//!
//! The caller is a tenant administrator (sesame_common::tenant_admin: tenant_admin, owner, or a role in
//! SESAME_TENANT_ADMIN_ROLES) and acts on the token's tenant only: an X-Tenant-ID or tenant_id naming another tenant
//! is 403. The role reaches the user's token at their next sign-in or refresh (principal_effective).
use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use serde_json::{json, Value};
use sesame_common::token_versioning::BumpReason;
use sesame_idam_authz_core_gen::handlers::assign_principal_role::Request;

use super::role_admin_common::{
    admin, bad_request, internal, invalidate_tokens, not_found, parse_uuid, record_change, refused,
};
use crate::services::role_admin::{valid_role, Assigned, RoleAdmin};
use sesame_common::role_policy::RolePolicy;

#[handler(AssignPrincipalRoleController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<Value> {
    let d = &req.data;
    let a = match admin(
        &req.jwt_claims,
        &[Some(d.x_tenant_id.as_str()), d.tenant_id.as_deref()],
    ) {
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
    let given = |v: &Option<Value>| match v {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.trim().is_empty() => None,
        Some(Value::String(s)) => Some(s.trim().to_string()),
        Some(other) => Some(other.to_string()),
    };
    let policy = RolePolicy::configured();
    if let Err(r) = policy.may_change(&a.roles, &role) {
        return refused(&r);
    }
    if given(&d.expires_at).is_some() {
        return bad_request("expiring role assignments are not supported yet");
    }
    let org_id = match given(&d.org_id).as_deref() {
        Some(o) => match parse_uuid(o, "org_id") {
            Ok(u) => Some(u),
            Err(r) => return r,
        },
        None => None,
    };
    if let Some(org) = org_id {
        // A token has no place for an organisation's scope, so an administrator role "for one organisation" would act
        // tenant-wide: administrator roles are tenant-wide only.
        if policy.is_admin_role(&role) {
            return bad_request(
                "administrator roles are tenant-wide; they cannot be given for one organisation",
            );
        }
        if !RoleAdmin::org_in_tenant(sesame_idam_database::db(), &a.tenant, org) {
            return not_found("No such organisation in this tenant");
        }
    }
    // sesame_idam.users is under forced RLS (tenant_id = sesame.tenant_id): the user check inside assign reads nothing
    // without the tenant set on the session, and every grant came back "No such user in this tenant".
    match sesame_idam_database::with_pre_auth_tenant(&a.tenant, |exec| {
        RoleAdmin::assign(exec, &a.tenant, user_id, &role, org_id)
    }) {
        Ok(Assigned::Created) => {
            // The grant stands either way; without the bump it reaches the user's token at their next refresh.
            if let Err(e) = invalidate_tokens(user_id) {
                tracing::warn!(error = %e, %user_id, "assign_principal_role: token version bump failed");
            }
            record_change(
                &a,
                user_id,
                &role,
                "role_assigned",
                BumpReason::RoleAssigned,
            );
            HttpJson::new(204, json!(null))
        }
        Ok(Assigned::AlreadyHeld) => HttpJson::new(204, json!(null)),
        Ok(Assigned::NoSuchUser) => not_found("No such user in this tenant"),
        Err(e) => internal("assign_principal_role", &e),
    }
}
