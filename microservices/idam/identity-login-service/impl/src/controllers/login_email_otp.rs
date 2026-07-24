// BRRTRouter: user-owned

//! `POST /auth/login/email-otp` — request an email OTP.
//!
//! Gate A3 wiring: every send request passes through the abuse guard
//! (dedupe, per-recipient window/day caps) BEFORE any delivery. The response
//! is the same generic success whether the account exists, the send was
//! deduped, or a cap suppressed it — suppression is silent to the caller and
//! loud in the audit log (no enumeration, no cap oracle).
//!
//! OTP generation/storage and the email provider are not built yet — when
//! they land they slot in behind `decision.should_send()`.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::login_email_otp::Request;

use crate::services::abuse_guard::{self, Channel};
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;

#[handler(LoginEmailOtpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let email = req.data.email.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    let decision = abuse_guard::gate_otp_send(&tenant_id, Channel::Email, &email);
    if decision.should_send() {
        // TODO(OTP flows): generate 6-digit code, store hashed in Redis with
        // 5-minute TTL, dispatch via the email provider once one exists.
        tracing::info!(tenant = %tenant_id, "email OTP send allowed (provider not yet wired)");
    } else {
        tracing::info!(tenant = %tenant_id, ?decision, "email OTP send suppressed by abuse guard");
    }

    generic_success()
}

fn generic_success() -> HttpJson<serde_json::Value> {
    HttpJson::ok(serde_json::json!({
        "success": true,
        "message": "If the account exists, a code has been sent"
    }))
}
