// BRRTRouter: user-owned

//! `POST /auth/mfa/totp/confirm` — activate a pending authenticator app with its first code.
//!
//! A wrong code counts like any second-factor guess (account lockout, and the `mfa_token` burns after five). On
//! success the account holder is emailed; the account's first factor also gets its recovery codes (shown once); in a
//! sign-in that had to enrol, the tokens come back too ([`mfa::enrollment_done`]).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::mfa_totp_confirm::Request;

use crate::services::abuse_guard;
use crate::services::mfa::context::{self, Mode};
use crate::services::mfa::factors::{self, Kind};
use crate::services::mfa::notify::{self, Event};
use crate::services::mfa::{self, totp};

#[handler(MfaTotpConfirmController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let token = req.data.mfa_token.trim().to_string();
    let mut ctx = match mfa::context_for(&token, &[Mode::Manage, Mode::Enroll]) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let guard = mfa::guard_identity(&ctx.user_id);
    if abuse_guard::login_locked(&ctx.tenant_id, &guard).is_some() {
        return mfa::error(
            401,
            "account_locked",
            "Too many attempts. Wait a few minutes and try again.",
        );
    }
    let Ok(factor_id) = uuid::Uuid::parse_str(req.data.factor_id.trim()) else {
        return mfa::error(404, "not_found", "No such pending authenticator app.");
    };
    let pending = match factors::list(&ctx.tenant_id, ctx.user_id) {
        Ok(all) => all
            .into_iter()
            .find(|f| f.id == factor_id && f.kind == Kind::Totp && !f.active),
        Err(e) => {
            tracing::error!(error = %e, "mfa_totp_confirm: lookup failed");
            return mfa::unavailable();
        }
    };
    let Some(mut factor) = pending else {
        return mfa::error(404, "not_found", "No such pending authenticator app.");
    };
    let Some(Ok(secret)) = factor
        .totp_secret_sealed
        .as_deref()
        .map(|s| totp::open(s, &ctx.tenant_id, &ctx.user_id, &factor.id))
    else {
        tracing::error!("mfa_totp_confirm: pending secret does not open");
        return mfa::unavailable();
    };
    let now = chrono::Utc::now().timestamp();
    let Some(step) = totp::matching_step(&secret, &req.data.code, now, None) else {
        abuse_guard::record_login_failure(&ctx.tenant_id, &guard);
        notify::audit_denied(&ctx.tenant_id, Some(&ctx.user_id), "mfa_enrol_wrong_code");
        return match context::record_failure(&token, &mut ctx) {
            Ok(true) => mfa::invalid_code(),
            Ok(false) => mfa::invalid_token(),
            Err(_) => mfa::unavailable(),
        };
    };
    match factors::confirm_totp(&ctx.tenant_id, ctx.user_id, factor.id, step) {
        Ok(true) => {}
        Ok(false) => {
            return mfa::error(
                409,
                "factor_exists",
                "An authenticator app is already set up. Remove it to set up a new one.",
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "mfa_totp_confirm: confirm failed");
            return mfa::unavailable();
        }
    }
    abuse_guard::record_login_success(&ctx.tenant_id, &guard);
    factor.active = true;
    factor.last_used_at = Some(now);
    notify::announce(
        &ctx.tenant_id,
        &ctx.user_id,
        &ctx.email,
        Event::TotpAdded,
        &factor.label,
    );
    mfa::enrollment_done(&token, &ctx, &factor)
}
