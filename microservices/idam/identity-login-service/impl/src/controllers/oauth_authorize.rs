use brrtrouter::dispatcher::HandlerResponse;
use brrtrouter::typed::{HandlerResponseOutput, HttpJson, HttpRedirect, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::oauth_authorize::Request;

use crate::services::client_registry::{ClientRegistry, ClientRegistryError};
use crate::services::oidc_authorization::{
    store_authorization_session, validate_authorization_request, AuthorizationError,
};

pub enum OauthAuthorizeOutcome {
    Redirect(HttpRedirect),
    Error(HttpJson<serde_json::Value>),
}

impl HandlerResponseOutput for OauthAuthorizeOutcome {
    fn into_handler_response(self) -> Result<HandlerResponse, serde_json::Error> {
        match self {
            Self::Redirect(redirect) => redirect.into_handler_response(),
            Self::Error(error) => error.into_handler_response(),
        }
    }
}

#[handler(OauthAuthorizeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> OauthAuthorizeOutcome {
    let exec = sesame_idam_database::db();
    let binding = match ClientRegistry::resolve_active(&req.data.client_id, exec) {
        Ok(binding) => binding,
        Err(ClientRegistryError::Unknown | ClientRegistryError::NotActive) => {
            return OauthAuthorizeOutcome::Error(oauth_error(
                400,
                AuthorizationError::InvalidClient,
            ));
        }
        Err(ClientRegistryError::InvalidPolicy(error) | ClientRegistryError::Db(error)) => {
            tracing::error!(%error, client_id = %req.data.client_id, "OIDC client lookup failed");
            return OauthAuthorizeOutcome::Error(oauth_error(
                503,
                AuthorizationError::ServerUnavailable,
            ));
        }
    };

    let session = match validate_authorization_request(
        &binding,
        &req.data.response_type,
        &req.data.redirect_uri,
        &req.data.state,
        &req.data.nonce,
        &req.data.scope,
        &req.data.code_challenge,
        &req.data.code_challenge_method,
    ) {
        Ok(session) => session,
        Err(AuthorizationError::InvalidRedirectUri) => {
            return OauthAuthorizeOutcome::Error(oauth_error(
                400,
                AuthorizationError::InvalidRedirectUri,
            ));
        }
        Err(error) => {
            return OauthAuthorizeOutcome::Redirect(HttpRedirect::found(
                authorization_error_redirect(&req.data.redirect_uri, &req.data.state, &error),
            ));
        }
    };

    match store_authorization_session(&session) {
        Ok(request_id) => {
            let hosted_auth = std::env::var("OIDC_HOSTED_AUTH_URL")
                .unwrap_or_else(|_| "https://auth.sesameidentity.dev.local/authorize".to_string());
            match url::Url::parse(&hosted_auth) {
                Ok(mut url) => {
                    // Hosted auth needs tenant + client_id to call /auth/login, then
                    // completes the OIDC request via /oauth/authorize/complete.
                    url.query_pairs_mut()
                        .append_pair("request_id", &request_id)
                        .append_pair("tenant", &binding.tenant_id)
                        .append_pair("client_id", &binding.client_id);
                    OauthAuthorizeOutcome::Redirect(HttpRedirect::found(url.to_string()))
                }
                Err(error) => {
                    tracing::error!(%error, "OIDC_HOSTED_AUTH_URL is invalid");
                    OauthAuthorizeOutcome::Redirect(HttpRedirect::found(
                        authorization_error_redirect(
                            &req.data.redirect_uri,
                            &req.data.state,
                            &AuthorizationError::ServerUnavailable,
                        ),
                    ))
                }
            }
        }
        Err(error) => {
            tracing::error!(%error, "failed to persist OIDC authorization session");
            OauthAuthorizeOutcome::Redirect(HttpRedirect::found(authorization_error_redirect(
                &req.data.redirect_uri,
                &req.data.state,
                &AuthorizationError::ServerUnavailable,
            )))
        }
    }
}

fn oauth_error(status: u16, error: AuthorizationError) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        status,
        serde_json::json!({
            "error": error.oauth_code(),
            "error_description": error.description(),
        }),
    )
}

fn authorization_error_redirect(
    redirect_uri: &str,
    state: &str,
    error: &AuthorizationError,
) -> String {
    let Ok(mut redirect) = url::Url::parse(redirect_uri) else {
        return redirect_uri.to_string();
    };
    redirect
        .query_pairs_mut()
        .append_pair("error", error.oauth_code())
        .append_pair("error_description", error.description())
        .append_pair("state", state);
    redirect.to_string()
}
