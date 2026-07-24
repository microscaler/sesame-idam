// BRRTRouter: user-owned

//! `POST /auth/magic-link/sms` — request an SMS magic link.
//!
//! Gate A3 wiring: SMS channel — tenant opt-in (ADR-008 interim) + global
//! daily spend ceiling + per-recipient caps, shared with phone OTP so the two
//! endpoints cannot be combined to defeat the meter. Generic success
//! response regardless of suppression.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::sms_magic_link_send::Request;

use crate::services::abuse_guard::{self, Channel};
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;

#[handler(SmsMagicLinkSendController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let phone = req.data.phone.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    let decision = abuse_guard::gate_otp_send(&tenant_id, Channel::Sms, &phone);
    if decision.should_send() {
        // TODO(magic-link flow): mint single-use token + SMS provider.
        tracing::info!(tenant = %tenant_id, "sms magic link send allowed (provider not yet wired)");
    } else {
        tracing::info!(tenant = %tenant_id, ?decision, "sms magic link send suppressed by abuse guard");
    }

    generic_success()
}

fn generic_success() -> HttpJson<serde_json::Value> {
    HttpJson::ok(serde_json::json!({
        "success": true,
        "message": "If the account exists, a link has been sent"
    }))
}
