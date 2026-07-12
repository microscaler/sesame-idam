// BRRTRouter: user-owned

//! `PATCH /identity/me` — partial update of the current user's profile.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::users_me_patch::Request;

use crate::auth_context::authenticated_principal;
use crate::controllers::users_me_get::profile_json;
use crate::services::profile_service::{ProfileService, ProfileUpdate};

/// Maximum accepted length for name fields (per spec `maxLength: 100`).
const MAX_NAME_LEN: usize = 100;

#[handler(UsersMePatchController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let (user_id, tenant_id) = match authenticated_principal(&req.jwt_claims, &req.data.x_tenant_id)
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };

    let update = ProfileUpdate {
        first_name: req.data.first_name.clone(),
        last_name: req.data.last_name.clone(),
        avatar_url: req.data.avatar_url.clone(),
    };

    for (label, value) in [
        ("first_name", &update.first_name),
        ("last_name", &update.last_name),
    ] {
        if let Some(v) = value {
            if v.chars().count() > MAX_NAME_LEN {
                return HttpJson::new(
                    400,
                    serde_json::json!({
                        "error": "validation_error",
                        "error_description": format!("{label} exceeds {MAX_NAME_LEN} characters"),
                    }),
                );
            }
        }
    }

    let entry = AuditLogEntry::new(AuditEventType::JwtValidated, "identity-session-service")
        .tenant_id(tenant_id.clone())
        .user_id(user_id.to_string())
        .decision_source("users_me_patch")
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
            tracing::error!(error = %e, "users_me_patch: user lookup failed");
            return internal_error();
        }
    };

    let profile = if update.is_empty() {
        match ProfileService::find_profile(user_id, exec) {
            Ok(profile) => profile,
            Err(e) => {
                tracing::error!(error = %e, "users_me_patch: profile lookup failed");
                return internal_error();
            }
        }
    } else {
        match ProfileService::upsert_profile(user_id, &update, exec) {
            Ok(profile) => Some(profile),
            Err(e) => {
                tracing::error!(error = %e, "users_me_patch: profile upsert failed");
                return internal_error();
            }
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
