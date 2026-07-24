// BRRTRouter: user-owned

//! `POST /auth/login/phone-otp` — request an SMS OTP.
//!
//! Gate A3 wiring: SMS sends additionally require tenant opt-in (ADR-008
//! interim: `SMS_OPTED_IN_TENANTS`) and are metered against the global daily
//! SMS spend ceiling — toll fraud is bounded even if a recipient rotates
//! numbers. Response is generic success regardless of suppression (no
//! enumeration, no cap oracle); denials land in the audit log.
//!
//! OTP generation/storage and the SMS provider are not built yet — they slot
//! in behind `decision.should_send()`.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::login_phone_otp::Request;

use crate::services::abuse_guard::{self, Channel};
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;

#[handler(LoginPhoneOtpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let phone = req.data.phone.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    let decision = abuse_guard::gate_otp_send(&tenant_id, Channel::Sms, &phone);
    if decision.should_send() {
        // TODO(OTP flows): generate code, store hashed in Redis (5m TTL),
        // dispatch via the SMS provider once one exists.
        tracing::info!(tenant = %tenant_id, "phone OTP send allowed (provider not yet wired)");
    } else {
        tracing::info!(tenant = %tenant_id, ?decision, "phone OTP send suppressed by abuse guard");
    }

    generic_success()
}

fn generic_success() -> HttpJson<serde_json::Value> {
    HttpJson::ok(serde_json::json!({
        "success": true,
        "message": "If the account exists, a code has been sent"
    }))
}
