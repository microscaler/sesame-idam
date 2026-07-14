// BRRTRouter: user-owned

//! `GET /auth/social/{provider}/login` — start OAuth authorization redirect.

use brrtrouter::dispatcher::HandlerResponse;
use brrtrouter::typed::{HandlerResponseOutput, HttpJson, HttpRedirect, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::social_login::Request;

use crate::services::oauth::{
    build_authorize_url, store_oauth_state, OAuthState, SupportedProvider,
};
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_oauth_service::TenantOAuthService;
use crate::services::tenant_service::TenantService;

/// Success redirect or structured error JSON.
pub enum SocialLoginOutcome {
    Redirect(HttpRedirect),
    Error(HttpJson<serde_json::Value>),
}

impl HandlerResponseOutput for SocialLoginOutcome {
    fn into_handler_response(self) -> Result<HandlerResponse, serde_json::Error> {
        match self {
            Self::Redirect(r) => Ok(r.into_handler_response()?),
            Self::Error(j) => j.into_handler_response(),
        }
    }
}

#[handler(SocialLoginController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> SocialLoginOutcome {
    let tenant_id = req.data.x_tenant_id.trim();
    let provider_name = req.data.provider.trim();
    let redirect_uri = req.data.redirect_uri.trim();

    if tenant_id.is_empty() {
        return SocialLoginOutcome::Error(oauth_json_error(400, "tenant_required"));
    }
    if redirect_uri.is_empty() {
        return SocialLoginOutcome::Error(oauth_json_error(400, "redirect_uri_required"));
    }

    let Some(provider) = SupportedProvider::parse(provider_name) else {
        return SocialLoginOutcome::Error(oauth_json_error(400, "unsupported_provider"));
    };

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id, exec) {
        return SocialLoginOutcome::Error(tenant_http_error(e));
    }

    let resolved = match TenantOAuthService::resolve(tenant_id, provider.as_str(), exec) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, tenant_id, provider = provider.as_str(), "social_login: oauth resolve failed");
            return SocialLoginOutcome::Error(oauth_json_error(500, "internal_error"));
        }
    };

    let Some(resolved) = resolved else {
        return SocialLoginOutcome::Error(oauth_json_error(503, "oauth_not_configured"));
    };

    if !resolved.redirect_uri_allowed(redirect_uri) {
        return SocialLoginOutcome::Error(oauth_json_error(400, "redirect_uri_not_allowed"));
    }

    let creds = resolved.credentials();

    let state_payload = OAuthState {
        tenant_id: tenant_id.to_string(),
        provider: provider.as_str().to_string(),
        redirect_uri: redirect_uri.to_string(),
    };

    let state = match store_oauth_state(&state_payload) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "social_login: failed to store oauth state");
            return SocialLoginOutcome::Error(oauth_json_error(503, "oauth_state_unavailable"));
        }
    };

    let authorize_url = build_authorize_url(
        provider,
        &creds,
        redirect_uri,
        &state,
        req.data.scope.as_deref(),
    );

    SocialLoginOutcome::Redirect(HttpRedirect::found(authorize_url))
}

fn oauth_json_error(status: u16, error: &str) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        status,
        serde_json::json!({
            "error": error,
            "error_description": error,
        }),
    )
}
