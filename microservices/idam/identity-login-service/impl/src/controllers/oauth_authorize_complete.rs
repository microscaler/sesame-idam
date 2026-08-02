use brrtrouter::dispatcher::HandlerResponse;
use brrtrouter::typed::{HandlerResponseOutput, HttpJson, HttpRedirect, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::oauth_authorize_complete::Request;

use crate::auth_context::authenticated_principal;
use crate::services::oidc_authorization::{
    consume_authorization_session, mint_authorization_code, AuthorizationCode,
};

pub enum OauthAuthorizeCompleteOutcome {
    Redirect(HttpRedirect),
    Error(HttpJson<serde_json::Value>),
}

impl HandlerResponseOutput for OauthAuthorizeCompleteOutcome {
    fn into_handler_response(self) -> Result<HandlerResponse, serde_json::Error> {
        match self {
            Self::Redirect(redirect) => redirect.into_handler_response(),
            Self::Error(error) => error.into_handler_response(),
        }
    }
}

#[handler(OauthAuthorizeCompleteController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> OauthAuthorizeCompleteOutcome {
    let (user_id, tenant_id) = match authenticated_principal(&req.jwt_claims, None) {
        Ok(principal) => principal,
        Err(error) => return OauthAuthorizeCompleteOutcome::Error(error),
    };
    let Some(session) = consume_authorization_session(req.data.request_id.trim()) else {
        return OauthAuthorizeCompleteOutcome::Error(oauth_error(
            400,
            "invalid_request",
            "Authorization session is invalid, expired, or already used",
        ));
    };
    if session.tenant_id != tenant_id {
        return OauthAuthorizeCompleteOutcome::Error(oauth_error(
            400,
            "invalid_request",
            "Authorization session does not match the authenticated tenant",
        ));
    }

    let claims = req.jwt_claims.as_ref();
    let auth_time = claims
        .and_then(|claims| claims.get("auth_time").or_else(|| claims.get("iat")))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    let code_payload = AuthorizationCode {
        client_id: session.client_id,
        tenant_id: session.tenant_id,
        application_id: session.application_id,
        redirect_uri: session.redirect_uri.clone(),
        user_id: user_id.to_string(),
        scopes: session.scopes,
        nonce: session.nonce,
        code_challenge: session.code_challenge,
        auth_time,
        created_at: chrono::Utc::now().timestamp(),
    };
    let code = match mint_authorization_code(&code_payload) {
        Ok(code) => code,
        Err(error) => {
            tracing::error!(%error, "failed to persist OIDC authorization code");
            return OauthAuthorizeCompleteOutcome::Error(oauth_error(
                503,
                "temporarily_unavailable",
                "Authorization service is temporarily unavailable",
            ));
        }
    };

    let Ok(mut redirect) = url::Url::parse(&session.redirect_uri) else {
        return OauthAuthorizeCompleteOutcome::Error(oauth_error(
            500,
            "server_error",
            "Registered redirect URI is invalid",
        ));
    };
    redirect
        .query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", &session.state);
    OauthAuthorizeCompleteOutcome::Redirect(HttpRedirect::found(redirect.to_string()))
}

fn oauth_error(status: u16, error: &str, error_description: &str) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        status,
        serde_json::json!({
            "error": error,
            "error_description": error_description,
        }),
    )
}
