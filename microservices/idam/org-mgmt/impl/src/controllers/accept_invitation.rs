// BRRTRouter: user-owned

//! POST /invitations/accept — accept org invite token (authenticated user).

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse};

use crate::jwt_context;
use crate::services::org_lifecycle::{self, OrgLifecycleError};

pub fn handle(req: HandlerRequest) -> HandlerResponse {
    let Some(tenant_id) = jwt_context::tenant_from_request(&req) else {
        return HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "missing_tenant",
                "message": "Validated JWT tenant_id claim is required"
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

    let user_uuid = match uuid::Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return HandlerResponse::json(
                401,
                serde_json::json!({
                    "error": "unauthorized",
                    "message": "Invalid token subject"
                }),
            );
        }
    };

    // users.email (and invite/membership writes) sit behind tenant RLS.
    let email = match sesame_idam_database::with_pre_auth_tenant(&tenant_id, |exec| {
        Ok(org_lifecycle::lookup_user_email(exec, &tenant_id, user_uuid))
    }) {
        Ok(Some(email)) => email,
        Ok(None) => {
            return HandlerResponse::json(
                404,
                serde_json::json!({
                    "error": "user_not_found",
                    "message": "User profile not found"
                }),
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "accept_invitation: tenant session failed for user lookup");
            return HandlerResponse::json(
                500,
                serde_json::json!({
                    "error": "internal_error",
                    "message": "An unexpected error occurred"
                }),
            );
        }
    };

    // Do not bump token version here. The BFF immediately calls
    // `POST /sessions/active-organization` with the same bearer token to re-issue
    // JWT with `org_id`. A pre-accept bump would 401 that re-issue (Launch 1.0
    // revocation enforcement). Token rotation happens on successful activate.

    let accepted = match sesame_idam_database::with_pre_auth_tenant(&tenant_id, |exec| {
        match org_lifecycle::accept_invitation(exec, &tenant_id, &user_id, &email, token) {
            Ok(org) => Ok(Ok(org)),
            Err(err) => Ok(Err(err)),
        }
    }) {
        Ok(inner) => inner,
        Err(e) => {
            tracing::error!(error = %e, "accept_invitation: tenant session failed");
            return HandlerResponse::json(
                500,
                serde_json::json!({
                    "error": "internal_error",
                    "message": "An unexpected error occurred"
                }),
            );
        }
    };

    match accepted {
        Ok(org) => {
            tracing::info!(user_id, org_id = %org.id, "invitation accepted; await active-org token re-issue");
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
