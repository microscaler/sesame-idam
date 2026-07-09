// BRRTRouter: user-owned

//! `POST /auth/login` — password login.
//!
//! Verifies credentials against `sesame_idam.users` (tenant-scoped), then
//! issues a real Ed25519-signed access token + refresh token pair
//! (`TokenResponse`). Returns 401 `invalid_credentials` for unknown user,
//! wrong password, or non-active account — a single indistinguishable error
//! to prevent user enumeration.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::auth_login::{Request, Response};

use crate::audit::EMITTER;
use crate::services::password;
use crate::services::token_issuer;
use crate::services::user_service::{UserService, STATUS_ACTIVE};
use sesame_common::audit::{AuditEventType, AuditLogEntry};

/// Default portal/client for direct browser logins.
const DEFAULT_PORTAL: &str = "frontend";

#[handler(AuthLoginController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let email = req.data.email.clone();

    let exec = sesame_idam_database::db();

    let user = match UserService::find_by_tenant_and_email(&tenant_id, &email, exec) {
        Ok(user) => user,
        Err(e) => {
            tracing::error!(error = %e, "auth_login: user lookup failed");
            return internal_error();
        }
    };

    // Unknown user, wrong password, and disabled account all produce the
    // same 401 to prevent user enumeration.
    let Some(user) = user else {
        emit_login_audit(&tenant_id, None, false, "user_not_found");
        return invalid_credentials();
    };

    if user.status != STATUS_ACTIVE {
        emit_login_audit(&tenant_id, Some(user.id), false, "account_not_active");
        return invalid_credentials();
    }

    if !password::verify_password(&req.data.password, &user.password_hash) {
        emit_login_audit(&tenant_id, Some(user.id), false, "wrong_password");
        return invalid_credentials();
    }

    let user_id = user.id.to_string();

    // JWT enrichment: fetch effective roles from authz-core (the single
    // sanctioned cross-service call). Degrades to empty roles on failure —
    // login must not hard-fail when authz-core is briefly unavailable.
    let roles = crate::services::authz_client::fetch_effective_roles(
        &user_id,
        &tenant_id,
        DEFAULT_PORTAL,
    )
    .unwrap_or_else(|e| {
        tracing::warn!(error = %e, "auth_login: authz-core enrichment failed — issuing token without roles");
        vec![]
    });

    let exec = sesame_idam_database::db();
    let preferred_org = req.data.organization_id.as_deref();
    let active_org = crate::services::org_context::resolve_active_org_id(
        exec,
        &user_id,
        &tenant_id,
        preferred_org,
    );
    let org_id_str = active_org.map(|id| id.to_string());

    let tokens = match token_issuer::issue_tokens(
        &user_id,
        &tenant_id,
        DEFAULT_PORTAL,
        roles.clone(),
        "customer",
        org_id_str.as_deref(),
    ) {
        Ok(tokens) => tokens,
        Err(e) => {
            tracing::error!(error = %e, "auth_login: token issuance failed");
            return internal_error();
        }
    };

    emit_login_audit(&tenant_id, Some(user.id), true, "password");

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
        user_id,
    };

    match serde_json::to_value(&body) {
        Ok(json) => HttpJson::ok(json),
        Err(e) => {
            tracing::error!(error = %e, "auth_login: response serialization failed");
            internal_error()
        }
    }
}

fn invalid_credentials() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        401,
        serde_json::json!({
            "error": "invalid_credentials",
            "error_description": "Invalid email or password"
        }),
    )
}

fn internal_error() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "An unexpected error occurred"
        }),
    )
}

fn emit_login_audit(tenant_id: &str, user_id: Option<uuid::Uuid>, success: bool, reason: &str) {
    let event_type = if success {
        AuditEventType::JwtIssued
    } else {
        AuditEventType::ValidationFailed
    };
    let mut builder = AuditLogEntry::new(event_type, "identity-login-service")
        .tenant_id(tenant_id.to_string())
        .decision_source("password_login")
        .result(if success { "allowed" } else { "denied" })
        .reason(reason.to_string());
    if let Some(id) = user_id {
        builder = builder.user_id(id.to_string());
    }
    match builder.build() {
        Ok(entry) => EMITTER.emit(entry),
        Err(e) => tracing::warn!(error = %e, "auth_login: audit entry build failed"),
    }
}
