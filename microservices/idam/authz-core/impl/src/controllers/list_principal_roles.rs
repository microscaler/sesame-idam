//! `GET /authz/principals/roles` — every role assignment in the caller's own tenant, or one user's.
//!
//! Same caller rule as assign (a tenant administrator; the tenant is the token's).
use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use serde_json::{json, Value};
use sesame_idam_authz_core_gen::handlers::list_principal_roles::Request;

use super::role_admin_common::{admin, internal, parse_uuid};
use crate::services::role_admin::RoleAdmin;

#[handler(ListPrincipalRolesController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<Value> {
    let a = match admin(&req.jwt_claims, &[]) {
        Ok(a) => a,
        Err(r) => return r,
    };
    let user_id = match req
        .data
        .user_id
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())
    {
        Some(u) => match parse_uuid(u, "user_id") {
            Ok(u) => Some(u),
            Err(r) => return r,
        },
        None => None,
    };
    match RoleAdmin::list(sesame_idam_database::db(), &a.tenant, user_id) {
        Ok(rows) => {
            let assignments: Vec<Value> = rows
                .iter()
                .map(|r| {
                    let mut v = json!({
                        "user_id": r.principal_id.to_string(),
                        "role": r.role_name,
                        "created_at": r.created_at.to_rfc3339(),
                    });
                    if let Some(o) = r.resource_id {
                        v["org_id"] = json!(o.to_string());
                    }
                    v
                })
                .collect();
            HttpJson::new(200, json!({ "assignments": assignments }))
        }
        Err(e) => internal("list_principal_roles", &e),
    }
}
