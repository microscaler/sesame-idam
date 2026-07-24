// BRRTRouter: user-owned

//! `POST /auth/verify-magic` — consume a magic-link token and issue tokens.
//!
//! The "click" half of the magic-link round trip: the emailed URL carries a
//! single-use 256-bit token whose hash is burned atomically (GETDEL) on
//! first use — replay returns the same generic 401 as an unknown token.
//! Token guessing is not lockout-tracked: at 256 bits of entropy the A2
//! counter would only add noise; failed consumes are still audited.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::magic_link_verify::Request;

use crate::audit::EMITTER;
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;
use crate::services::{login_success, otp};
use sesame_common::audit::{AuditEventType, AuditLogEntry};

#[handler(MagicLinkVerifyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    let Some(user_id) = otp::consume_magic_link(&tenant_id, &req.data.token) else {
        emit_audit(&tenant_id, None, false, "invalid_expired_or_reused_token");
        return invalid_token();
    };

    match login_success::issue_login_response(&user_id, &tenant_id, None) {
        Ok(body) => {
            emit_audit(&tenant_id, Some(&user_id), true, "magic_link");
            HttpJson::ok(body)
        }
        Err(e) => {
            tracing::error!(error = %e, "magic_link_verify: issuance failed");
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

/// Unknown, expired, and already-used tokens are indistinguishable.
fn invalid_token() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        401,
        serde_json::json!({
            "error": "invalid_credentials",
            "error_description": "Invalid or expired link"
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
        .decision_source("magic_link")
        .result(if success { "allowed" } else { "denied" })
        .reason(reason.to_string());
    if let Some(id) = user_id {
        builder = builder.user_id(id.to_string());
    }
    match builder.build() {
        Ok(entry) => EMITTER.emit(entry),
        Err(e) => tracing::warn!(error = %e, "magic_link_verify: audit build failed"),
    }
}
