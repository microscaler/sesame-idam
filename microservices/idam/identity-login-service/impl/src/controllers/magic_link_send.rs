// BRRTRouter: user-owned

//! `POST /auth/magic-link` — request an email magic link.
//!
//! Gate A3 wiring: same abuse-guard gate as email OTP (dedupe + per-recipient
//! caps share the email channel budget, so mixing endpoints cannot multiply a
//! mailbox flood). Generic success response regardless of suppression.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::magic_link_send::Request;

use crate::services::abuse_guard::{self, Channel};
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;

#[handler(MagicLinkSendController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let email = req.data.email.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    let decision = abuse_guard::gate_otp_send(&tenant_id, Channel::Email, &email);
    if decision.should_send() {
        // TODO(magic-link flow): mint single-use token, store hashed in Redis,
        // send via the email provider once one exists.
        tracing::info!(tenant = %tenant_id, "magic link send allowed (provider not yet wired)");
    } else {
        tracing::info!(tenant = %tenant_id, ?decision, "magic link send suppressed by abuse guard");
    }

    generic_success()
}

fn generic_success() -> HttpJson<serde_json::Value> {
    HttpJson::ok(serde_json::json!({
        "success": true,
        "message": "If the account exists, a link has been sent"
    }))
}
