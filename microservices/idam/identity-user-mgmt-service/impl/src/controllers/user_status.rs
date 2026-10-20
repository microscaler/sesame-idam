//! Shared enable/disable implementation for the admin status endpoints.

use brrtrouter::typed::HttpJson;
use uuid::Uuid;

use crate::audit::EMITTER;
use crate::services::user_admin_service::{user_response_json, UserAdminService};
use sesame_common::{
    audit::{AuditEventType, AuditLogEntry},
    VersionStore,
};

/// The administrator roles `user_id` holds tenant-wide (role_assignments has no RLS: the tenant is filtered here).
fn admin_roles_of(tenant_id: &str, user_id: Uuid) -> Result<Vec<String>, String> {
    use lifeguard::LifeExecutor;
    let policy = sesame_common::role_policy::RolePolicy::configured();
    let mut held = Vec::new();
    for role in &policy.admin {
        // count(*) always returns a row, so a database failure is an error rather than "not held"
        let row = sesame_idam_database::db()
            .query_one_values(
                "SELECT count(*)::bigint FROM sesame_idam.role_assignments \
                 WHERE tenant_id = $1 AND principal_id = $2 AND resource_id IS NULL AND lower(role_name) = $3",
                &sea_query::Values(vec![tenant_id.into(), user_id.into(), role.clone().into()]),
            )
            .map_err(|e| e.to_string())?;
        if row.try_get::<usize, i64>(0).map_err(|e| e.to_string())? > 0 {
            held.push(role.clone());
        }
    }
    Ok(held)
}

/// May `admin` change the status of `user_id`? Only if they could grant every administrator role the user holds
/// (sesame_common::role_policy): a tenant_admin does not disable, or re-enable, an owner.
///
/// # Errors
/// The 403 (or 500) to return.
pub fn may_change_status_of(
    admin: &sesame_common::tenant_admin::TenantAdmin,
    user_id: Uuid,
) -> Result<(), HttpJson<serde_json::Value>> {
    let held = admin_roles_of(&admin.tenant, user_id).map_err(|e| {
        tracing::error!(error = %e, "set_user_status: role lookup failed");
        HttpJson::new(
            500,
            serde_json::json!({ "error": "internal_error", "error_description": "An unexpected error occurred" }),
        )
    })?;
    let policy = sesame_common::role_policy::RolePolicy::configured();
    for role in held {
        if policy.may_change(&admin.roles, &role).is_err() {
            return Err(sesame_common::tenant_admin::forbidden(
                "That user holds an administrator role you do not hold: only someone who holds it (or owner) may change their status",
            ));
        }
    }
    Ok(())
}

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

    // forced RLS on sesame_idam.users: read and update inside the tenant's scope
    let updated = match sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| {
        UserAdminService::set_status(tenant_id, user_uuid, status, exec)
    }) {
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
