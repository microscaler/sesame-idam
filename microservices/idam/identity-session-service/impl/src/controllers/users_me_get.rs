//! `GET /identity/me` — current user profile (DB-backed).
//!
//! Identity comes from the validated JWT (`sub` + `tenant_id` claims), which
//! typed dispatch cannot see — this is a raw handler (see
//! [`crate::raw_handler`]). PII lives here by design: it was removed from
//! access tokens (Story 2.3) and consumers fetch it from this endpoint.

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse};

use crate::models::user::UserModel;
use crate::models::user_profile::UserProfileModel;
use crate::raw_handler::authenticated_principal;
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

/// Raw handler for `GET /identity/me`.
pub fn handle_raw(req: &HandlerRequest) -> HandlerResponse {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let (user_id, tenant_id) = match authenticated_principal(req) {
        Ok(principal) => principal,
        Err(response) => return *response,
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
            // Token references a user that no longer exists on this tenant.
            return HandlerResponse::json(
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

    HandlerResponse::json(200, profile_json(&user, profile.as_ref()))
}

fn internal_error() -> HandlerResponse {
    HandlerResponse::json(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "An unexpected error occurred",
        }),
    )
}
