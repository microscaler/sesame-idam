//! `PATCH /identity/me` — partial update of the current user's profile.
//!
//! Raw handler (needs the JWT principal — see [`crate::raw_handler`]).
//! Accepts `first_name`, `last_name`, `avatar_url` from
//! `UpdateUserProfileRequest`; other spec fields (`name`,
//! `preferred_username`) have no storage column yet and are ignored.

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse};

use crate::controllers::users_me_get::profile_json;
use crate::raw_handler::authenticated_principal;
use crate::services::profile_service::{ProfileService, ProfileUpdate};

/// Maximum accepted length for name fields (per spec `maxLength: 100`).
const MAX_NAME_LEN: usize = 100;

/// Raw handler for `PATCH /identity/me`.
pub fn handle_raw(req: &HandlerRequest) -> HandlerResponse {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let (user_id, tenant_id) = match authenticated_principal(req) {
        Ok(principal) => principal,
        Err(response) => return *response,
    };

    // Parse the update body.
    let body = req.body.clone().unwrap_or(serde_json::Value::Null);
    let field = |name: &str| body.get(name).and_then(|v| v.as_str()).map(String::from);
    let update = ProfileUpdate {
        first_name: field("first_name"),
        last_name: field("last_name"),
        avatar_url: field("avatar_url"),
    };

    for (label, value) in [
        ("first_name", &update.first_name),
        ("last_name", &update.last_name),
    ] {
        if let Some(v) = value {
            if v.chars().count() > MAX_NAME_LEN {
                return HandlerResponse::json(
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

    // The user must exist on this tenant before any profile write.
    let user = match ProfileService::find_user(&tenant_id, user_id, exec) {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HandlerResponse::json(
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
        // Nothing to change — return the current profile.
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
