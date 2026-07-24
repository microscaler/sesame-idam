// BRRTRouter: user-owned

//! `POST /auth/password/forgot` — request a password-reset link.
//!
//! Same guarantees as the other send paths:
//! - Gate A3: the abuse guard meters the send (dedupe + per-recipient window
//!   and daily caps) BEFORE any lookup or delivery.
//! - No enumeration: the response is an identical generic success whether the
//!   account exists, is inactive, was capped, or the provider failed.
//!
//! The token is single-use, hashed in Redis, 15-minute TTL, and lives in its
//! own keyspace so it can never be replayed as a magic-link login.
//!
//! Delivery is EMAIL here. Per ADR-009, `password_reset` is one of the two
//! SMS-permitted purposes — an SMS variant can reuse `sms::send_sms(...,
//! SmsPurpose::PasswordReset)` when a phone-based reset flow is added.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::auth_forgot_password::Request;

use crate::services::abuse_guard::{self, Channel};
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;
use crate::services::user_service::{UserService, STATUS_ACTIVE};
use crate::services::{email, otp};

#[handler(AuthForgotPasswordController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let recipient = req.data.email.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    let decision = abuse_guard::gate_otp_send(&tenant_id, Channel::Email, &recipient);
    if !decision.should_send() {
        tracing::info!(tenant = %tenant_id, ?decision, "password reset suppressed by abuse guard");
        return generic_success();
    }

    let user = sesame_idam_database::with_pre_auth_tenant(&tenant_id, |exec| {
        UserService::find_by_tenant_and_email(&tenant_id, &recipient, exec)
    })
    .ok()
    .flatten();

    match user {
        Some(user) if user.status == STATUS_ACTIVE => {
            match otp::create_password_reset(&tenant_id, &user.id.to_string()) {
                Ok(token) => {
                    let url = otp::password_reset_url(&tenant_id, &token);
                    let body = format!(
                        "Reset your Sesame password with this link:\n\n{url}\n\nIt expires in 15 minutes and can be used once. If you did not request this, you can safely ignore this email — your password has not changed."
                    );
                    if let Err(e) = email::send_email(&recipient, "Reset your password", &body) {
                        tracing::error!(error = %e, tenant = %tenant_id, "password reset delivery failed");
                    }
                }
                Err(e) => tracing::error!(error = %e, tenant = %tenant_id, "password reset mint failed"),
            }
        }
        _ => tracing::info!(tenant = %tenant_id, "password reset for unknown/inactive account — no send"),
    }

    generic_success()
}

fn generic_success() -> HttpJson<serde_json::Value> {
    HttpJson::ok(serde_json::json!({
        "success": true,
        "message": "If the account exists, a reset link has been sent"
    }))
}
