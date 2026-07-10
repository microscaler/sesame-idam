// BRRTRouter: user-owned

//! `GET /identity/me` — current user profile (DB-backed).
//!
//! Identity comes from validated JWT claims on [`TypedHandlerRequest::jwt_claims`]
//! (BR-2). PII lives here by design: it was removed from access tokens (Story 2.3).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::users_me_get::Request;

use crate::auth_context::authenticated_principal;
use crate::models::user::UserModel;
use crate::models::user_profile::UserProfileModel;
use crate::services::profile_service::ProfileService;

/// Build the spec `UserProfile` JSON from the user row + optional profile.
pub fn profile_json(user: &UserModel, profile: Option<&UserProfileModel>) -> serde_json::Value {
    let first_name = profile.and_then(|p| p.first_name.clone());
    let last_name = profile.and_then(|p| p.last_name.clone());
    let name = match (&first_name, &last_name) {
        (Some(f), Some(l)) => Some(format!("{f} {l}")),
        (Some(f), None) => Some(f.clone()),
        (None, Some(l)) => Some(l.clone()),
        (None, None) => None,
    };

    serde_json::json!({
        "sub": user.id,
        "user_id": user.id,
        "email": user.email,
        "email_verified": user.email_verified,
        "phone": user.phone,
        "phone_verified": user.phone_verified,
        "name": name,
        "first_name": first_name,
        "last_name": last_name,
        "avatar_url": profile.and_then(|p| p.avatar_url.clone()),
        "is_active": user.status == "active",
        "created_at": user.created_at.to_rfc3339(),
        "updated_at": user.updated_at.to_rfc3339(),
    })
}

#[handler(UsersMeGetController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let (user_id, tenant_id) = match authenticated_principal(&req.jwt_claims, &req.data.x_tenant_id)
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };

    let entry = AuditLogEntry::new(AuditEventType::JwtValidated, "identity-session-service")
        .tenant_id(tenant_id.clone())
        .user_id(user_id.to_string())
        .decision_source("users_me_get")
        .result("allowed")
        .build();
    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    let exec = sesame_idam_database::db();

    let user = match ProfileService::find_user(&tenant_id, user_id, exec) {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpJson::new(
                401,
                serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "Unauthorized (invalid or missing token)",
                }),
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "users_me_get: user lookup failed");
            return internal_error();
        }
    };

    let profile = match ProfileService::find_profile(user_id, exec) {
        Ok(profile) => profile,
        Err(e) => {
            tracing::error!(error = %e, "users_me_get: profile lookup failed");
            return internal_error();
        }
    };

    HttpJson::ok(profile_json(&user, profile.as_ref()))
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
