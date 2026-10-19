//! `POST /admin/users` — admin user creation, idempotent by email. The caller is a tenant administrator of the tenant.
//!
//! Returns 201 on creation, 200 with the existing user when the email is
//! already registered on the tenant. Admin-created users have no password
//! until a password flow assigns one.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::create_user::Request;

use crate::audit::EMITTER;
use crate::services::user_admin_service::{user_response_json, UserAdminService};
use sesame_common::audit::{AuditEventType, AuditLogEntry};

#[handler(CreateUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    // A tenant administrator of their own tenant only (sesame_common::tenant_admin, ADR-011).
    let admin = match sesame_common::tenant_admin::tenant_admin(&req.jwt_claims) {
        Ok(a) => a,
        Err(r) => return r,
    };
    if let Err(r) = admin.require_same_tenant(Some(&req.data.x_tenant_id)) {
        return r;
    }
    let tenant_id = admin.tenant.clone();
    let email = req.data.email.trim().to_lowercase();

    if email.is_empty() || !email.contains('@') {
        return HttpJson::new(
            400,
            serde_json::json!({
                "error": "invalid_request",
                "error_description": "a valid email is required",
            }),
        );
    }

    let exec = sesame_idam_database::db();
    let outcome = match UserAdminService::create_idempotent(
        &tenant_id,
        &email,
        req.data.email_confirmed.unwrap_or(false),
        exec,
    ) {
        Ok(outcome) => outcome,
        Err(e) => {
            tracing::error!(error = %e, "create_user: insert failed");
            return internal_error();
        }
    };

    let entry = AuditLogEntry::new(AuditEventType::JwtIssued, "identity-user-mgmt-service")
        .tenant_id(tenant_id)
        .user_id(outcome.user.id.to_string())
        .decision_source("admin_create_user")
        .result(if outcome.created {
            "allowed"
        } else {
            "idempotent"
        })
        .build();
    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    let status = if outcome.created { 201 } else { 200 };
    HttpJson::new(status, user_response_json(&outcome.user))
}

fn internal_error() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "An unexpected error occurred",
        }),
    )
}
