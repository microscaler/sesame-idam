// BRRTRouter: user-owned

//! `POST /auth/verify/email-otp` — verify an email OTP and issue tokens.
//!
//! Second half of the email-OTP factor: checks the A2 lockout gate first
//! (locked identities get the same generic 401 as a wrong code), verifies
//! the single-use attempt-capped code from Redis, records failures into the
//! SHARED lockout counter (guessing codes locks the identity exactly like
//! guessing passwords), and on success issues the same token response as
//! password login via `login_success::issue_login_response`.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::verify_email_otp::Request;

use crate::audit::EMITTER;
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;
use crate::services::{abuse_guard, login_success, otp};
use sesame_common::audit::{AuditEventType, AuditLogEntry};

#[handler(VerifyEmailOtpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let email = req.data.email.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    // Gate A2: same generic 401 for locked as for wrong code.
    if abuse_guard::login_locked(&tenant_id, &email).is_some() {
        emit_audit(&tenant_id, None, false, "account_locked");
        return invalid_code();
    }

    let Some(user_id) = otp::verify_email_otp(&tenant_id, &email, &req.data.code) else {
        abuse_guard::record_login_failure(&tenant_id, &email);
        emit_audit(&tenant_id, None, false, "invalid_or_expired_code");
        return invalid_code();
    };

    abuse_guard::record_login_success(&tenant_id, &email);

    match login_success::issue_login_response(&user_id, &tenant_id, None) {
        Ok(body) => {
            emit_audit(&tenant_id, Some(&user_id), true, "email_otp");
            HttpJson::ok(body)
        }
        Err(e) => {
            tracing::error!(error = %e, "verify_email_otp: issuance failed");
            HttpJson::new(
                500,
                serde_json::json!({
                    "error": "internal_error",
                    "error_description": "An unexpected error occurred"
                }),
            )
        }
    }
}

/// Wrong code, expired code, unknown account, locked identity — one
/// indistinguishable 401 (mirrors `auth_login::invalid_credentials`).
fn invalid_code() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        401,
        serde_json::json!({
            "error": "invalid_credentials",
            "error_description": "Invalid email or code"
        }),
    )
}

fn emit_audit(tenant_id: &str, user_id: Option<&str>, success: bool, reason: &str) {
    let event_type = if success {
        AuditEventType::JwtIssued
    } else {
        AuditEventType::ValidationFailed
    };
    let mut builder = AuditLogEntry::new(event_type, "identity-login-service")
        .tenant_id(tenant_id.to_string())
        .decision_source("email_otp")
        .result(if success { "allowed" } else { "denied" })
        .reason(reason.to_string());
    if let Some(id) = user_id {
        builder = builder.user_id(id.to_string());
    }
    match builder.build() {
        Ok(entry) => EMITTER.emit(entry),
        Err(e) => tracing::warn!(error = %e, "verify_email_otp: audit build failed"),
    }
}
