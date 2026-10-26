// BRRTRouter: user-owned

//! `POST /auth/mfa/verify` — the second step of a sign-in: an authenticator-app code or a recovery code (ADR-008).
//!
//! Only for a context in `challenge` mode, and only with a method the sign-in offers (recomputed here, never taken
//! from the client). Wrong codes count towards the account's second-factor lockout (shared by every second-factor
//! endpoint, so code guessing is bounded per account whatever the route) and burn the `mfa_token` after five. A
//! TOTP code is accepted once (its step is recorded atomically). A recovery code works once, is announced to the
//! account holder, and under a passkey policy leads to enrolling a new passkey rather than to tokens (§6).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_common::authn::Method;
use sesame_idam_identity_login_service_gen::handlers::mfa_verify::Request;

use crate::services::abuse_guard;
use crate::services::mfa::context::{self, Mode};
use crate::services::mfa::factors::{self, Kind};
use crate::services::mfa::notify::{self, Event};
use crate::services::mfa::{self, recovery, totp};

#[handler(MfaVerifyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let token = req.data.mfa_token.trim().to_string();
    let mut ctx = match mfa::context_for(&token, &[Mode::Challenge]) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let guard = mfa::guard_identity(&ctx.user_id);
    if abuse_guard::login_locked(&ctx.tenant_id, &guard).is_some() {
        notify::audit_denied(&ctx.tenant_id, Some(&ctx.user_id), "mfa_locked");
        return mfa::error(
            401,
            "account_locked",
            "Too many attempts. Wait a few minutes and try again.",
        );
    }
    let method = req.data.method.trim();
    let offered = match mfa::offered(&ctx) {
        Ok(o) => o,
        Err(e) => {
            tracing::error!(error = %e, "mfa_verify: factor lookup failed");
            return mfa::unavailable();
        }
    };
    if !offered.contains(&method) {
        return mfa::error(
            400,
            "invalid_request",
            "That method cannot be used for this sign-in.",
        );
    }

    let verified = match method {
        "totp" => verify_totp(&ctx, &req.data.code),
        "recovery_code" => verify_recovery_code(&ctx, &req.data.code),
        _ => Ok(None),
    };
    let proved = match verified {
        Ok(Some(m)) => m,
        Ok(None) => {
            abuse_guard::record_login_failure(&ctx.tenant_id, &guard);
            notify::audit_denied(&ctx.tenant_id, Some(&ctx.user_id), "mfa_wrong_code");
            return match context::record_failure(&token, &mut ctx) {
                Ok(true) => mfa::invalid_code(),
                Ok(false) => mfa::invalid_token(),
                Err(_) => mfa::unavailable(),
            };
        }
        Err(response) => return response,
    };
    abuse_guard::record_login_success(&ctx.tenant_id, &guard);
    if proved == Method::RecoveryCode {
        let left = factors::recovery_codes_left(&ctx.tenant_id, ctx.user_id).unwrap_or(0);
        notify::announce(
            &ctx.tenant_id,
            &ctx.user_id,
            &ctx.email,
            Event::RecoveryCodeUsed,
            &left.to_string(),
        );
    }
    ctx.methods.push(proved);
    mfa::next_step(&token, ctx)
}

fn verify_totp(
    ctx: &context::MfaContext,
    code: &str,
) -> Result<Option<Method>, HttpJson<serde_json::Value>> {
    let active = factors::list_active(&ctx.tenant_id, ctx.user_id).map_err(|e| {
        tracing::error!(error = %e, "mfa_verify: factor lookup failed");
        mfa::unavailable()
    })?;
    let Some(app) = active.iter().find(|f| f.kind == Kind::Totp) else {
        return Ok(None);
    };
    let Some(sealed) = app.totp_secret_sealed.as_deref() else {
        return Ok(None);
    };
    let secret = totp::open(sealed, &ctx.tenant_id, &ctx.user_id, &app.id).map_err(|e| {
        tracing::error!(error = %e, "mfa_verify: TOTP secret does not open");
        mfa::unavailable()
    })?;
    let now = chrono::Utc::now().timestamp();
    let Some(step) = totp::matching_step(&secret, code, now, app.totp_last_step) else {
        return Ok(None);
    };
    // The replay check and the record are one statement: a concurrent second use of this code loses here.
    match factors::advance_totp_step(&ctx.tenant_id, ctx.user_id, app.id, step) {
        Ok(true) => Ok(Some(Method::Totp)),
        Ok(false) => Ok(None),
        Err(e) => {
            tracing::error!(error = %e, "mfa_verify: TOTP step update failed");
            Err(mfa::unavailable())
        }
    }
}

fn verify_recovery_code(
    ctx: &context::MfaContext,
    code: &str,
) -> Result<Option<Method>, HttpJson<serde_json::Value>> {
    let Some(normalized) = recovery::normalize(code) else {
        return Ok(None);
    };
    let hash = recovery::hash(&ctx.tenant_id, &ctx.user_id, &normalized);
    match factors::consume_recovery_code(&ctx.tenant_id, ctx.user_id, &hash) {
        Ok(true) => Ok(Some(Method::RecoveryCode)),
        Ok(false) => Ok(None),
        Err(e) => {
            tracing::error!(error = %e, "mfa_verify: recovery code check failed");
            Err(mfa::unavailable())
        }
    }
}
