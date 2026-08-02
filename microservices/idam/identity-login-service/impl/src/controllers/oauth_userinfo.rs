//! OpenID Connect UserInfo endpoint.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::oauth_userinfo::Request;

use crate::auth_context::authenticated_principal;
use crate::services::user_service::UserService;

#[handler(OauthUserinfoController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (user_id, tenant_id) = match authenticated_principal(&req.jwt_claims, None) {
        Ok(principal) => principal,
        Err(_) => return bearer_error("invalid_token", "A valid access token is required"),
    };
    let scopes = req
        .jwt_claims
        .as_ref()
        .and_then(|claims| claims.get("scope"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    if !scopes.contains(&"openid") {
        return bearer_error("insufficient_scope", "The openid scope is required");
    }

    let exec = sesame_idam_database::db();
    let user = match UserService::find_by_tenant_and_id(&tenant_id, user_id, exec) {
        Ok(Some(user)) if user.status == "active" => user,
        Ok(_) => return bearer_error("invalid_token", "The token subject is unavailable"),
        Err(error) => {
            tracing::error!(%error, "UserInfo subject lookup failed");
            return HttpJson::new(
                503,
                serde_json::json!({
                    "error": "temporarily_unavailable",
                    "error_description": "UserInfo is temporarily unavailable",
                }),
            );
        }
    };

    let mut response = serde_json::json!({ "sub": user.id.to_string() });
    let object = response
        .as_object_mut()
        .expect("UserInfo response is an object");
    if scopes.contains(&"email") {
        object.insert("email".to_string(), serde_json::json!(user.email));
        object.insert(
            "email_verified".to_string(),
            serde_json::json!(user.email_verified),
        );
    }
    if scopes.contains(&"profile") {
        object.insert(
            "preferred_username".to_string(),
            serde_json::json!(user.email),
        );
    }
    HttpJson::ok(response)
}

fn bearer_error(error: &str, description: &str) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        401,
        serde_json::json!({
            "error": error,
            "error_description": description,
        }),
    )
}
