// BRRTRouter: user-owned

//! `POST /auth/social/{provider}/callback` — exchange OAuth code and issue Sesame tokens.

use base64::Engine;
use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use rand::RngCore;
use sesame_idam_identity_login_service_gen::handlers::social_callback::{Request, Response};

use crate::audit::EMITTER;
use crate::services::oauth::{consume_oauth_state, exchange_code, SupportedProvider};
use crate::services::password;
use crate::services::social_credential_service::SocialCredentialService;
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_oauth_service::TenantOAuthService;
use crate::services::tenant_service::TenantService;
use crate::services::token_issuer;
use crate::services::user_service::{UserService, STATUS_ACTIVE};
use sesame_common::audit::{AuditEventType, AuditLogEntry};

const DEFAULT_PORTAL: &str = "frontend";

#[handler(SocialCallbackController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.trim();
    let provider_name = req.data.provider.trim();
    let code = req.data.code.trim();
    let state = req.data.state.trim();

    if tenant_id.is_empty() {
        return oauth_error(400, "tenant_required");
    }

    let Some(provider) = SupportedProvider::parse(provider_name) else {
        return oauth_error(400, "unsupported_provider");
    };

    let stored = match consume_oauth_state(state) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "social_callback: invalid state");
            return oauth_error(400, "invalid_state");
        }
    };

    if stored.tenant_id != tenant_id {
        return oauth_error(400, "tenant_state_mismatch");
    }
    if stored.provider != provider.as_str() {
        return oauth_error(400, "provider_state_mismatch");
    }

    let redirect_uri = req
        .data
        .redirect_uri
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(stored.redirect_uri.as_str());

    if redirect_uri != stored.redirect_uri {
        return oauth_error(400, "redirect_uri_mismatch");
    }

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id, exec) {
        return tenant_http_error(e);
    }

    let resolved = match TenantOAuthService::resolve(tenant_id, provider.as_str(), exec) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "social_callback: oauth resolve failed");
            return oauth_error(500, "internal_error");
        }
    };

    let Some(resolved) = resolved else {
        return oauth_error(503, "oauth_not_configured");
    };

    if !resolved.redirect_uri_allowed(redirect_uri) {
        return oauth_error(400, "redirect_uri_not_allowed");
    }

    let creds = resolved.credentials();

    let profile = match exchange_code(provider, &creds, code, redirect_uri) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, provider = provider.as_str(), "social_callback: code exchange failed");
            return oauth_error(400, "provider_exchange_failed");
        }
    };

    if !profile.email_verified {
        return oauth_error(400, "email_not_verified");
    }

    let user = match SocialCredentialService::find_user_by_provider(
        tenant_id,
        provider.as_str(),
        &profile.provider_user_id,
        exec,
    ) {
        Ok(Some(user)) => user,
        Ok(None) => {
            match UserService::find_by_tenant_and_email(tenant_id, &profile.email, exec) {
                Ok(Some(_)) => {
                    return oauth_error(409, "account_exists_link_required");
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::error!(error = %e, "social_callback: email lookup failed");
                    return oauth_error(500, "internal_error");
                }
            }

            let mut random_secret = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut random_secret);
            let secret = base64::engine::general_purpose::STANDARD.encode(random_secret);
            let placeholder_password = match password::hash_password(&secret) {
                Ok(hash) => hash,
                Err(e) => {
                    tracing::error!(error = %e, "social_callback: placeholder hash failed");
                    return oauth_error(500, "internal_error");
                }
            };

            let user_id = match UserService::create_oauth_user(
                tenant_id,
                &profile.email,
                &placeholder_password,
                exec,
            ) {
                Ok(id) => id,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("unique") || msg.contains("duplicate") {
                        return oauth_error(409, "account_exists_link_required");
                    }
                    tracing::error!(error = %e, "social_callback: user create failed");
                    return oauth_error(500, "internal_error");
                }
            };

            if let Err(e) = SocialCredentialService::link_provider(
                user_id,
                provider.as_str(),
                &profile.provider_user_id,
                exec,
            ) {
                tracing::error!(error = %e, "social_callback: link provider failed");
                return oauth_error(500, "internal_error");
            }

            match UserService::find_by_tenant_and_email(tenant_id, &profile.email, exec) {
                Ok(Some(user)) => user,
                Ok(None) => return oauth_error(500, "internal_error"),
                Err(e) => {
                    tracing::error!(error = %e, "social_callback: reload user failed");
                    return oauth_error(500, "internal_error");
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "social_callback: provider lookup failed");
            return oauth_error(500, "internal_error");
        }
    };

    if user.status != STATUS_ACTIVE {
        return oauth_error(403, "account_not_active");
    }

    let user_id_str = user.id.to_string();
    let tokens = match token_issuer::issue_tokens(
        &user_id_str,
        tenant_id,
        DEFAULT_PORTAL,
        vec![],
        "customer",
        None,
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "social_callback: token issuance failed");
            return oauth_error(500, "internal_error");
        }
    };

    emit_social_login_audit(
        tenant_id,
        Some(user_id_str.as_str()),
        provider.as_str(),
        true,
    );

    let body = Response {
        access_token: tokens.access_token,
        token_type: "Bearer".to_string(),
        expires_in: i32::try_from(tokens.expires_in).unwrap_or(300),
        refresh_token: tokens.refresh_token,
        user_id: user_id_str,
        social_provider: provider.as_str().to_string(),
        social_provider_user_id: Some(profile.provider_user_id),
    };

    match serde_json::to_value(&body) {
        Ok(json) => HttpJson::ok(json),
        Err(e) => {
            tracing::error!(error = %e, "social_callback: response serialization failed");
            oauth_error(500, "internal_error")
        }
    }
}

fn oauth_error(status: u16, error: &str) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        status,
        serde_json::json!({
            "error": error,
            "error_description": error,
        }),
    )
}

fn emit_social_login_audit(tenant_id: &str, user_id: Option<&str>, provider: &str, success: bool) {
    if tenant_id.is_empty() {
        return;
    }
    let result = if success { "allowed" } else { "denied" };
    match AuditLogEntry::new(AuditEventType::JwtIssued, "identity-login-service")
        .tenant_id(tenant_id.to_string())
        .user_id(user_id.unwrap_or("").to_string())
        .decision_source("social_oauth")
        .result(result)
        .build()
    {
        Ok(entry) => EMITTER.emit(entry),
        Err(e) => tracing::warn!(error = %e, provider, "social_callback: audit entry build failed"),
    }
}
