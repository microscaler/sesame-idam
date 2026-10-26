// BRRTRouter: user-owned

//! `POST /auth/login` — password login.
//!
//! Verifies credentials against `sesame_idam.users` (tenant-scoped), then
//! runs the MFA gate: a real Ed25519-signed access token + refresh token pair
//! (`TokenResponse`, 200) when no second factor is needed, else 202 with an
//! `mfa_token` and the next step (`/auth/mfa/verify`, a passkey, or enrolment). Returns 401 `invalid_credentials` for unknown user,
//! wrong password, or non-active account — a single indistinguishable error
//! to prevent user enumeration.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::auth_login::Request;

use crate::audit::EMITTER;
use crate::services::abuse_guard;
use crate::services::client_registry::{ClientRegistry, ClientRegistryError};
use crate::services::mfa::{self, Subject};
use crate::services::password;
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;
use crate::services::user_service::{UserService, STATUS_ACTIVE};
use sesame_common::audit::{AuditEventType, AuditLogEntry};
use sesame_common::authn::Method;

#[handler(AuthLoginController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let email = req.data.email.clone();
    let exec = sesame_idam_database::db();
    let binding = match ClientRegistry::resolve_pre_auth(
        &req.data.client_id,
        req.data.x_tenant_id.as_deref(),
        exec,
    ) {
        Ok(binding) => binding,
        Err(ClientRegistryError::Unknown | ClientRegistryError::NotActive) => {
            return invalid_client();
        }
        Err(ClientRegistryError::InvalidPolicy(error)) => {
            tracing::error!(%error, client_id = %req.data.client_id, "auth_login: invalid registered client policy");
            return internal_error();
        }
        Err(ClientRegistryError::Db(error)) => {
            tracing::error!(%error, "auth_login: client registry lookup failed");
            return internal_error();
        }
    };
    let tenant_id = binding.tenant_id;

    // Platform suspend / unknown tenant must fail closed before credential checks.
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    // Gate A2: locked identities get the SAME generic 401 as wrong
    // credentials — no oracle for lock state or account existence. The
    // check keys on (tenant, identifier) regardless of whether the account
    // exists, so unknown identifiers lock out identically to real ones.
    if abuse_guard::login_locked(&tenant_id, &email).is_some() {
        emit_login_audit(&tenant_id, None, false, "account_locked");
        return invalid_credentials();
    }

    let user = match sesame_idam_database::with_pre_auth_tenant(&tenant_id, |exec| {
        UserService::find_by_tenant_and_email(&tenant_id, &email, exec)
    }) {
        Ok(user) => user,
        Err(e) => {
            tracing::error!(error = %e, "auth_login: user lookup failed");
            return internal_error();
        }
    };

    // Unknown user, wrong password, and disabled account all produce the
    // same 401 to prevent user enumeration.
    let Some(user) = user else {
        // Count failures for unknown identifiers too: lockout behaviour must
        // not differ between real and nonexistent accounts (enumeration).
        abuse_guard::record_login_failure(&tenant_id, &email);
        emit_login_audit(&tenant_id, None, false, "user_not_found");
        return invalid_credentials();
    };

    if user.status != STATUS_ACTIVE {
        abuse_guard::record_login_failure(&tenant_id, &email);
        emit_login_audit(&tenant_id, Some(user.id), false, "account_not_active");
        return invalid_credentials();
    }

    if !password::verify_password(&req.data.password, &user.password_hash) {
        abuse_guard::record_login_failure(&tenant_id, &email);
        emit_login_audit(&tenant_id, Some(user.id), false, "wrong_password");
        return invalid_credentials();
    }

    abuse_guard::record_login_success(&tenant_id, &email);

    // The MFA gate (ADR-008): an account with a second factor, or whose organisation requires one, gets 202 and the
    // next step (no token); otherwise 200 with tokens whose amr/acr say "password".
    let response = mfa::after_first_factor(
        &Subject {
            tenant_id: tenant_id.clone(),
            user_id: user.id,
            email: user.email.clone(),
            portal: binding.portal.clone(),
            organization_id: req.data.organization_id.clone(),
        },
        Method::Password,
    );
    match response.status {
        200 => emit_login_audit(&tenant_id, Some(user.id), true, "password"),
        202 => emit_login_audit(
            &tenant_id,
            Some(user.id),
            true,
            "password_second_factor_pending",
        ),
        _ => {}
    }
    response
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

fn invalid_client() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        401,
        serde_json::json!({
            "error": "invalid_client",
            "error_description": "Unknown or inactive client"
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
