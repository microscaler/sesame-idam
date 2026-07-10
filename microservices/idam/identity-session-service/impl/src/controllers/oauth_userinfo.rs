// BRRTRouter: user-owned

//! `GET /identity/userinfo` — OIDC userinfo (DB-backed profile).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::oauth_userinfo::Request;

use crate::auth_context::authenticated_principal;
use crate::controllers::users_me_get::profile_json;
use crate::models::user::UserModel;
use crate::models::user_profile::UserProfileModel;
use crate::services::profile_service::ProfileService;

fn internal_error() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "An unexpected error occurred",
        }),
    )
}

/// Map DB user + profile to the OIDC userinfo response shape.
fn userinfo_json(user: &UserModel, profile: Option<&UserProfileModel>) -> serde_json::Value {
    let base = profile_json(user, profile);
    let first_name = base.get("first_name").and_then(|v| v.as_str()).map(String::from);
    let last_name = base.get("last_name").and_then(|v| v.as_str()).map(String::from);
    let name = base.get("name").and_then(|v| v.as_str()).map(String::from);

    serde_json::json!({
        "sub": user.id.to_string(),
        "user_id": user.id.to_string(),
        "email": user.email,
        "email_verified": user.email_verified,
        "phone_number": user.phone,
        "phone_verified": user.phone_verified,
        "name": name,
        "first_name": first_name,
        "last_name": last_name,
        "preferred_username": user.email,
        "picture_url": profile.and_then(|p| p.avatar_url.clone()),
        "updated_at": user.updated_at.to_rfc3339(),
    })
}

#[handler(OauthUserinfoController)]
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
        .decision_source("oauth_userinfo")
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
            tracing::error!(error = %e, "oauth_userinfo: user lookup failed");
            return internal_error();
        }
    };

    let profile = match ProfileService::find_profile(user_id, exec) {
        Ok(profile) => profile,
        Err(e) => {
            tracing::error!(error = %e, "oauth_userinfo: profile lookup failed");
            return internal_error();
        }
    };

    HttpJson::ok(userinfo_json(&user, profile.as_ref()))
}
