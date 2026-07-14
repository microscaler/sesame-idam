//! Shared enable/disable implementation for the admin status endpoints.

use brrtrouter::typed::HttpJson;
use uuid::Uuid;

use crate::audit::EMITTER;
use crate::services::user_admin_service::{user_response_json, UserAdminService};
use sesame_common::{
    audit::{AuditEventType, AuditLogEntry},
    VersionStore,
};

/// Set the status of a user and return the admin user response.
pub fn set_user_status(
    tenant_id: &str,
    user_id: &str,
    status: &str,
    decision_source: &str,
) -> HttpJson<serde_json::Value> {
    let Ok(user_uuid) = user_id.parse::<Uuid>() else {
        return HttpJson::new(
            400,
            serde_json::json!({
                "error": "invalid_request",
                "error_description": "user_id must be a uuid",
            }),
        );
    };

    let bumped_version = match VersionStore::from_env()
        .and_then(|store| store.increment_subject(&user_uuid.to_string()))
    {
        Ok(version) => version,
        Err(error) => {
            tracing::error!(%error, user_id = %user_uuid, "token version bump failed before status change");
            return HttpJson::new(
                503,
                serde_json::json!({
                    "error": "security_state_unavailable",
                    "error_description": "Session invalidation is temporarily unavailable",
                }),
            );
        }
    };

    let exec = sesame_idam_database::db();
    let updated = match UserAdminService::set_status(tenant_id, user_uuid, status, exec) {
        Ok(updated) => updated,
        Err(e) => {
            tracing::error!(error = %e, "set_user_status: update failed");
            return HttpJson::new(
                500,
                serde_json::json!({
                    "error": "internal_error",
                    "error_description": "An unexpected error occurred",
                }),
            );
        }
    };

    let Some(user) = updated else {
        return HttpJson::new(
            404,
            serde_json::json!({
                "error": "invalid_request",
                "error_description": "User not found",
            }),
        );
    };

    let entry = AuditLogEntry::new(AuditEventType::TokenRevoked, "identity-user-mgmt-service")
        .tenant_id(tenant_id.to_string())
        .user_id(user.id.to_string())
        .decision_source(decision_source.to_string())
        .result("allowed")
        .build();
    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    tracing::info!(user_id = %user.id, token_version = bumped_version, "user status invalidated existing access tokens");
    HttpJson::ok(user_response_json(&user))
}
