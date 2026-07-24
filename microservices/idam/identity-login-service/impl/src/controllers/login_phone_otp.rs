// BRRTRouter: user-owned

//! `POST /auth/login/phone-otp` — request an SMS OTP.
//!
//! COST POLICY: per-login SMS is OFF by default. SMS is ~100-1000× email and
//! the prime toll-fraud target, so it's reserved for high-value purposes
//! (registration, password reset) — routine second-factor login uses email
//! OTP. This route is retained (spec surface + deliberate per-env opt-in via
//! `SMS_ALLOWED_PURPOSES`), but by default it returns the generic success
//! WITHOUT minting a code or spending. When enabled, it runs the same path
//! as email OTP:
//!
//! Gate A3 + the SMS slice: the abuse guard runs FIRST — SMS additionally
//! requires tenant opt-in (ADR-008 interim: `SMS_OPTED_IN_TENANTS`) and is
//! metered against the global daily SMS spend ceiling. When allowed and the
//! account exists+is active with a matching phone, a 6-digit code is minted
//! (hashed in Redis, TTL'd, attempt-capped, single-use) and delivered via
//! the SMS provider (Twilio in prod; a Redis-outbox mock in dev/CI). The
//! response is the same generic success whether the account exists, the send
//! was suppressed, or the provider failed — loud in logs+audit, silent to
//! callers.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::login_phone_otp::Request;

use crate::services::abuse_guard::{self, Channel};
use crate::services::sms::SmsPurpose;
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;
use crate::services::user_service::{UserService, STATUS_ACTIVE};
use crate::services::{otp, sms};

#[handler(LoginPhoneOtpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let phone = req.data.phone.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    // Cost policy: per-login SMS is off unless explicitly enabled. Short-
    // circuit BEFORE the guard/lookup/mint so nothing is spent — generic
    // response, no enumeration signal.
    if !sms::purpose_allowed(SmsPurpose::Login) {
        tracing::info!(tenant = %tenant_id, "phone OTP login disabled by cost policy — use email OTP");
        return generic_success();
    }

    let decision = abuse_guard::gate_otp_send(&tenant_id, Channel::Sms, &phone);
    if !decision.should_send() {
        tracing::info!(tenant = %tenant_id, ?decision, "phone OTP send suppressed by abuse guard");
        return generic_success();
    }

    let user = sesame_idam_database::with_pre_auth_tenant(&tenant_id, |exec| {
        UserService::find_by_tenant_and_phone(&tenant_id, &phone, exec)
    })
    .ok()
    .flatten();

    match user {
        Some(user) if user.status == STATUS_ACTIVE => {
            match otp::create_phone_otp(&tenant_id, &phone, &user.id.to_string()) {
                Ok(code) => {
                    let body = format!("Your Sesame verification code is {code}. It expires in 5 minutes.");
                    if let Err(e) = sms::send_sms(&phone, &body, SmsPurpose::Login) {
                        tracing::error!(error = %e, tenant = %tenant_id, "phone OTP delivery failed");
                    }
                }
                Err(e) => tracing::error!(error = %e, tenant = %tenant_id, "phone OTP mint failed"),
            }
        }
        _ => tracing::info!(tenant = %tenant_id, "phone OTP requested for unknown/inactive account — no send"),
    }

    generic_success()
}

fn generic_success() -> HttpJson<serde_json::Value> {
    HttpJson::ok(serde_json::json!({
        "success": true,
        "message": "If the account exists, a code has been sent"
    }))
}
