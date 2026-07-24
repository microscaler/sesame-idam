// BRRTRouter: user-owned

//! `POST /auth/login/email-otp` — request an email OTP.
//!
//! Gate A3 + the email slice: the abuse guard (dedupe, per-recipient
//! window/day caps) runs BEFORE any lookup or delivery; when allowed and the
//! account exists+is active, a 6-digit code is minted (hashed in Redis,
//! TTL'd, attempt-capped, single-use) and delivered via SMTP (Mailpit in the
//! `data` namespace for non-prod). The response is the same generic success
//! whether the account exists, the send was suppressed, or the provider
//! failed — suppression/failures are loud in logs+audit, silent to callers.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::login_email_otp::Request;

use crate::services::abuse_guard::{self, Channel};
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;
use crate::services::user_service::{UserService, STATUS_ACTIVE};
use crate::services::{email, otp};

#[handler(LoginEmailOtpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let recipient = req.data.email.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    let decision = abuse_guard::gate_otp_send(&tenant_id, Channel::Email, &recipient);
    if !decision.should_send() {
        tracing::info!(tenant = %tenant_id, ?decision, "email OTP send suppressed by abuse guard");
        return generic_success();
    }

    // Existence is invisible from outside: unknown/inactive accounts take the
    // same path to the same response, minus the actual send.
    let user = sesame_idam_database::with_pre_auth_tenant(&tenant_id, |exec| {
        UserService::find_by_tenant_and_email(&tenant_id, &recipient, exec)
    })
    .ok()
    .flatten();

    match user {
        Some(user) if user.status == STATUS_ACTIVE => {
            match otp::create_email_otp(&tenant_id, &recipient, &user.id.to_string()) {
                Ok(code) => {
                    let body = format!(
                        "Your Sesame verification code is: {code}\n\nIt expires in 5 minutes. If you did not request it, ignore this email."
                    );
                    if let Err(e) = email::send_email(&recipient, "Your verification code", &body) {
                        tracing::error!(error = %e, tenant = %tenant_id, "email OTP delivery failed");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, tenant = %tenant_id, "email OTP mint failed");
                }
            }
        }
        _ => {
            tracing::info!(tenant = %tenant_id, "email OTP requested for unknown/inactive account — no send");
        }
    }

    generic_success()
}

fn generic_success() -> HttpJson<serde_json::Value> {
    HttpJson::ok(serde_json::json!({
        "success": true,
        "message": "If the account exists, a code has been sent"
    }))
}
