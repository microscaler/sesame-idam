// BRRTRouter: user-owned

//! POST /sessions/active-organization — re-issue JWT with `org_id` after org create/accept.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::set_active_organization::{
    Request, Response,
};

use crate::auth_context::authenticated_principal;

const DEFAULT_PORTAL: &str = "frontend";

#[handler(SetActiveOrganizationController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (user_id, tenant_id) = match authenticated_principal(&req.jwt_claims, &req.data.x_tenant_id)
    {
        Ok(pair) => pair,
        Err(resp) => return resp,
    };

    let org_id = req.data.organization_id.trim();
    if org_id.is_empty() {
        return HttpJson::new(
            400,
            serde_json::json!({
                "error": "validation_error",
                "message": "organization_id is required"
            }),
        );
    }

    let exec = sesame_idam_database::db();
    let user_id_str = user_id.to_string();
    let active = crate::services::org_context::resolve_active_org_id(
        exec,
        &user_id_str,
        &tenant_id,
        Some(org_id),
    );

    if active.is_none() {
        return HttpJson::new(
            403,
            serde_json::json!({
                "error": "forbidden",
                "message": "User is not an active member of this organization"
            }),
        );
    }

    let roles = crate::services::authz_client::fetch_effective_roles(
        &user_id_str,
        &tenant_id,
        DEFAULT_PORTAL,
    )
    .unwrap_or_default();

    let tokens = match crate::services::token_issuer::issue_tokens(
        &user_id_str,
        &tenant_id,
        DEFAULT_PORTAL,
        roles.clone(),
        "customer",
        Some(org_id),
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "set_active_organization: token issuance failed");
            return HttpJson::new(
                500,
                serde_json::json!({
                    "error": "internal_error",
                    "message": "Token issuance failed"
                }),
            );
        }
    };

    let body = Response {
        access_token: tokens.access_token,
        entitlements_hash: None,
        entitlements_ref: None,
        expires_in: i32::try_from(tokens.expires_in).unwrap_or(300),
        id_token: None,
        mfa_required: Some(false),
        permissions: None,
        refresh_token: tokens.refresh_token,
        refresh_token_expires_in: Some(
            i32::try_from(tokens.refresh_expires_in).unwrap_or(i32::MAX),
        ),
        roles: Some(roles),
        scope: Some(tokens.scope),
        token_type: "Bearer".to_string(),
        token_version: i32::try_from(tokens.token_version).ok(),
        user_id: user_id_str,
    };

    match serde_json::to_value(&body) {
        Ok(mut json) => {
            if let Some(obj) = json.as_object_mut() {
                obj.insert(
                    "organization_id".to_string(),
                    serde_json::Value::String(org_id.to_string()),
                );
            }
            HttpJson::ok(json)
        }
        Err(e) => {
            tracing::error!(error = %e, "set_active_organization: response serialization failed");
            HttpJson::new(
                500,
                serde_json::json!({
                    "error": "internal_error",
                    "message": "Response serialization failed"
                }),
            )
        }
    }
}
