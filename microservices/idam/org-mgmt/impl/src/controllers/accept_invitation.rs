// BRRTRouter: user-owned

//! POST /invitations/accept — accept org invite token (authenticated user).

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse};
use lifeguard::LifeExecutor;
use sesame_common::VersionStore;
use sesame_idam_database::db;

use crate::jwt_context;
use crate::services::org_lifecycle::{self, OrgLifecycleError};

pub fn handle(req: HandlerRequest) -> HandlerResponse {
    let Some(tenant_id) = jwt_context::tenant_from_request(&req) else {
        return HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "missing_tenant",
                "message": "X-Tenant-ID header is required"
            }),
        );
    };

    let Some(user_id) = jwt_context::user_id_from_request(&req) else {
        return HandlerResponse::json(
            401,
            serde_json::json!({
                "error": "unauthorized",
                "message": "Authentication required"
            }),
        );
    };

    let exec = db();
    let Some(email) = user_email(exec, &tenant_id, &user_id) else {
        return HandlerResponse::json(
            404,
            serde_json::json!({
                "error": "user_not_found",
                "message": "User profile not found"
            }),
        );
    };

    let body = req.body.clone().unwrap_or(serde_json::json!({}));
    let token = body
        .get("token")
        .and_then(|v| v.as_str())
        .map_or("", str::trim);

    if token.is_empty() {
        return HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "validation_error",
                "message": "token is required"
            }),
        );
    }

    let bumped_version = match VersionStore::from_env()
        .and_then(|store| store.increment_subject(&user_id))
    {
        Ok(version) => version,
        Err(error) => {
            tracing::error!(%error, user_id, "token version bump failed before invitation acceptance");
            return HandlerResponse::json(
                503,
                serde_json::json!({
                    "error": "security_state_unavailable",
                    "message": "Session invalidation is temporarily unavailable"
                }),
            );
        }
    };

    match org_lifecycle::accept_invitation(exec, &tenant_id, &user_id, &email, token) {
        Ok(org) => {
            tracing::info!(
                user_id,
                token_version = bumped_version,
                "invitation acceptance invalidated existing access tokens"
            );
            HandlerResponse::json(
                200,
                serde_json::json!({
                    "id": org.id.to_string(),
                    "name": org.name,
                    "tenant_id": org.tenant_id,
                }),
            )
        }
        Err(OrgLifecycleError::NotFound) => HandlerResponse::json(
            404,
            serde_json::json!({
                "error": "invite_not_found",
                "message": "Invitation not found or already accepted"
            }),
        ),
        Err(OrgLifecycleError::InviteExpired) => HandlerResponse::json(
            410,
            serde_json::json!({
                "error": "invite_expired",
                "message": "Invitation has expired"
            }),
        ),
        Err(OrgLifecycleError::EmailMismatch) => HandlerResponse::json(
            403,
            serde_json::json!({
                "error": "email_mismatch",
                "message": "Signed-in account email does not match the invitation"
            }),
        ),
        Err(OrgLifecycleError::AlreadyHasOrganization) => HandlerResponse::json(
            409,
            serde_json::json!({
                "error": "organization_exists",
                "message": "Account already belongs to an organization"
            }),
        ),
        Err(e) => HandlerResponse::error(500, &format!("{e:?}")),
    }
}

fn user_email(
    exec: &lifeguard::PooledLifeExecutor,
    tenant_id: &str,
    user_id: &str,
) -> Option<String> {
    let uid = uuid::Uuid::parse_str(user_id).ok()?;
    let row = exec
        .query_one_values(
            "SELECT email FROM sesame_idam.users WHERE id = $1 AND tenant_id = $2",
            &sea_query::Values(vec![uid.into(), tenant_id.into()]),
        )
        .ok()?;
    Some(row.get(0))
}
