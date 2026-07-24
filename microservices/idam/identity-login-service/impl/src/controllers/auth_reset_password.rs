// BRRTRouter: user-owned

//! `POST /auth/password/reset` — consume a reset token and set a new password.
//!
//! Security properties:
//! - The token is single-use (atomic GETDEL) and lives in its own keyspace —
//!   unknown, expired and already-used tokens are indistinguishable.
//! - Password strength is validated BEFORE the token is burned would be
//!   friendlier, but burning first prevents a strength-probe oracle from
//!   keeping a stolen token alive; we validate first and only then consume,
//!   because the token holder is already proven and a weak-password retry
//!   must not cost them the link.
//! - A successful reset CLEARS the A2 lockout counter for the identity: the
//!   legitimate owner has re-proven control, so stale failures shouldn't keep
//!   them locked out.
//! - Success does not sign the user in — they return to the hosted sign-in
//!   page. That keeps this endpoint free of token issuance and forces the new
//!   password to be exercised.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::auth_reset_password::Request;

use crate::audit::EMITTER;
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;
use crate::services::{abuse_guard, otp, password, user_service::UserService};
use sesame_common::audit::{AuditEventType, AuditLogEntry};

#[handler(AuthResetPasswordController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    // Strength first: a weak-password retry must not burn the user's link.
    if let Err(reason) = password::validate_password_strength(&req.data.new_password) {
        return HttpJson::new(
            400,
            serde_json::json!({ "error": "weak_password", "error_description": reason }),
        );
    }

    let Some(user_id) = otp::consume_password_reset(&tenant_id, &req.data.token) else {
        emit_audit(&tenant_id, None, false, "invalid_expired_or_reused_token");
        return invalid_token();
    };

    let Ok(hash) = password::hash_password(&req.data.new_password) else {
        tracing::error!("auth_reset_password: hashing failed");
        return internal_error();
    };

    let Ok(uuid) = user_id.parse::<uuid::Uuid>() else {
        tracing::error!("auth_reset_password: stored user_id is not a UUID");
        return internal_error();
    };

    let updated = sesame_idam_database::with_pre_auth_tenant(&tenant_id, |exec| {
        UserService::update_password_hash(uuid, &hash, exec)
    });
    if let Err(e) = updated {
        tracing::error!(error = %e, "auth_reset_password: password update failed");
        return internal_error();
    }

    // The owner has re-proven control — don't leave them locked out (A2).
    abuse_guard::record_login_success(&tenant_id, &user_id);
    emit_audit(&tenant_id, Some(&user_id), true, "password_reset");

    HttpJson::ok(serde_json::json!({
        "success": true,
        "message": "Password updated. You can now sign in."
    }))
}

/// Unknown, expired and already-used tokens are indistinguishable.
fn invalid_token() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        400,
        serde_json::json!({
            "error": "invalid_token",
            "error_description": "This reset link is invalid or has expired"
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

fn emit_audit(tenant_id: &str, user_id: Option<&str>, success: bool, reason: &str) {
    let event_type = if success {
        AuditEventType::VersionBump
    } else {
        AuditEventType::ValidationFailed
    };
    let mut builder = AuditLogEntry::new(event_type, "identity-login-service")
        .tenant_id(tenant_id.to_string())
        .decision_source("password_reset")
        .result(if success { "allowed" } else { "denied" })
        .reason(reason.to_string());
    if let Some(id) = user_id {
        builder = builder.user_id(id.to_string());
    }
    match builder.build() {
        Ok(entry) => EMITTER.emit(entry),
        Err(e) => tracing::warn!(error = %e, "auth_reset_password: audit build failed"),
    }
}
