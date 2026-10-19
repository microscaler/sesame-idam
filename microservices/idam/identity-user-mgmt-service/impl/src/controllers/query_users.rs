//! `GET /admin/users/query` — a page of the caller's tenant's users, by email.
//!
//! The caller is a tenant administrator of the tenant (sesame_common::tenant_admin); `X-Tenant-ID` naming another
//! tenant is 403. The users table has no names, username or lock: names are empty, the username is the email, locked
//! is false (a `locked=true` filter matches nobody), and signup_flow is not recorded (filtering on it matches nobody).
use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use serde_json::{json, Value};
use sesame_common::tenant_admin::tenant_admin;
use sesame_idam_identity_user_mgmt_service_gen::handlers::query_users::Request;

use crate::services::user_admin_service::{UserAdminService, UserQuery, STATUS_ACTIVE};

fn bad_request(message: &str) -> HttpJson<Value> {
    HttpJson::new(
        400,
        json!({ "error": "invalid_request", "error_description": message }),
    )
}

fn time(
    raw: Option<&str>,
    what: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, HttpJson<Value>> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|t| Some(t.with_timezone(&chrono::Utc)))
            .map_err(|_| bad_request(&format!("{what} is not an RFC 3339 time"))),
    }
}

#[handler(QueryUsersController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<Value> {
    let d = &req.data;
    let admin = match tenant_admin(&req.jwt_claims) {
        Ok(a) => a,
        Err(r) => return r,
    };
    if let Err(r) = admin.require_same_tenant(Some(&d.x_tenant_id)) {
        return r;
    }
    let page = d.page.unwrap_or(1).max(1);
    let limit = d.limit.unwrap_or(50).clamp(1, 200);
    let q = UserQuery {
        email_pattern: d.email_pattern.clone().filter(|p| !p.trim().is_empty()),
        email_confirmed: d.email_confirmed,
        enabled: d.enabled,
        disabled: d.disabled,
        created_after: match time(d.created_after.as_deref(), "created_after") {
            Ok(t) => t,
            Err(r) => return r,
        },
        created_before: match time(d.created_before.as_deref(), "created_before") {
            Ok(t) => t,
            Err(r) => return r,
        },
    };
    // Nothing is locked and no signup flow is recorded: asking for either matches nobody.
    let none_match = d.locked == Some(true)
        || d.signup_flow
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty());
    let (users, total) = if none_match {
        (Vec::new(), 0)
    } else {
        match UserAdminService::query(
            &admin.tenant,
            &q,
            page as usize,
            limit as usize,
            sesame_idam_database::db(),
        ) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, "query_users: query failed");
                return HttpJson::new(
                    500,
                    json!({ "error": "internal_error", "error_description": "An unexpected error occurred" }),
                );
            }
        }
    };
    let items: Vec<Value> = users
        .iter()
        .map(|u| {
            json!({
                "user_id": u.id.to_string(),
                "email": u.email,
                "first_name": "",
                "last_name": "",
                "username": u.email,
                "email_confirmed": u.email_verified,
                "enabled": u.status == STATUS_ACTIVE,
                "locked": false,
                "has_password": !u.password_hash.is_empty(),
                "created_at": u.created_at.to_rfc3339(),
            })
        })
        .collect();
    let has_more = (page as usize).saturating_mul(limit as usize) < total;
    HttpJson::ok(json!({
        "users": items,
        "total": total,
        "page": page,
        "limit": limit,
        "has_more": has_more,
    }))
}
