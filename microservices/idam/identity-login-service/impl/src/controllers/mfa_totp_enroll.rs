// BRRTRouter: user-owned

//! `POST /auth/mfa/totp/enroll` — start adding an authenticator app: a new secret and its `otpauth://` URI.
//!
//! Allowed in a management session, or in a sign-in that must enrol (`enroll` mode) unless its policy requires a
//! passkey. One authenticator app per account (remove the old one to replace it). The secret is sealed at rest
//! (`services/mfa/totp.rs`) and the enrolment stays pending until a first code confirms it.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::mfa_totp_enroll::Request;

use crate::services::mfa::context::Mode;
use crate::services::mfa::factors::{self, Kind};
use crate::services::mfa::policy::Requirement;
use crate::services::mfa::{self, totp};

#[handler(MfaTotpEnrollController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let ctx = match mfa::context_for(req.data.mfa_token.trim(), &[Mode::Manage, Mode::Enroll]) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if ctx.mode == Mode::Enroll && ctx.required == Requirement::Passkey {
        return mfa::error(
            409,
            "passkey_required",
            "Your organisation requires a passkey. Add a passkey to continue.",
        );
    }
    if !totp::available() {
        return mfa::error(
            503,
            "mfa_unavailable",
            "Authenticator apps are not available here yet.",
        );
    }
    match factors::list_active(&ctx.tenant_id, ctx.user_id) {
        Ok(active) if active.iter().any(|f| f.kind == Kind::Totp) => {
            return mfa::error(
                409,
                "factor_exists",
                "An authenticator app is already set up. Remove it to set up a new one.",
            );
        }
        Ok(_) => {}
        Err(e) => {
            tracing::error!(error = %e, "mfa_totp_enroll: lookup failed");
            return mfa::unavailable();
        }
    }
    let label = req
        .data
        .label
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .unwrap_or("Authenticator app")
        .chars()
        .take(64)
        .collect::<String>();
    let factor_id = uuid::Uuid::new_v4();
    let sealed = totp::generate_secret().and_then(|secret| {
        totp::seal(&secret, &ctx.tenant_id, &ctx.user_id, &factor_id).map(|s| (secret, s))
    });
    let (secret, sealed) = match sealed {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "mfa_totp_enroll: secret could not be sealed");
            return mfa::unavailable();
        }
    };
    if let Err(e) =
        factors::insert_pending_totp(&ctx.tenant_id, ctx.user_id, factor_id, &label, &sealed)
    {
        tracing::error!(error = %e, "mfa_totp_enroll: store failed");
        return mfa::unavailable();
    }
    let secret_b32 = totp::base32(&secret);
    let issuer = std::env::var("SESAME_WEBAUTHN_RP_NAME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "Sesame".to_string());
    HttpJson::ok(serde_json::json!({
        "factor_id": factor_id.to_string(),
        "otpauth_uri": totp::otpauth_uri(&issuer, &ctx.email, &secret_b32),
        "secret": secret_b32,
        "digits": totp::DIGITS,
        "period": totp::STEP_SECS,
    }))
}
