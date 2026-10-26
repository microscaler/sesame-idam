// BRRTRouter: user-owned

//! `POST /auth/passkeys/register` — verify the browser's registration response and store the passkey.
//!
//! The ceremony must belong to the same account as the `mfa_token`. The credential id must be new in the tenant
//! (409 otherwise). The account holder is emailed; the account's first factor also gets its recovery codes; in a
//! sign-in that had to enrol, the tokens come back too ([`mfa::enrollment_done`]).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::passkey_register::Request;

use crate::services::mfa::context::{self, CeremonyKind, Mode};
use crate::services::mfa::factors::{self, Factor, Kind};
use crate::services::mfa::notify::{self, Event};
use crate::services::mfa::{self, passkey};

#[handler(PasskeyRegisterController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let token = req.data.mfa_token.trim().to_string();
    let ctx = match mfa::context_for(&token, &[Mode::Manage, Mode::Enroll]) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let ceremony = match context::take_ceremony(req.data.challenge_id.trim()) {
        Ok(Some(c))
            if c.kind == CeremonyKind::Register
                && c.user_id == Some(ctx.user_id)
                && c.tenant_id == ctx.tenant_id =>
        {
            c
        }
        Ok(_) => return mfa::invalid_token(),
        Err(e) => {
            tracing::error!(error = %e, "passkey_register: ceremony store unavailable");
            return mfa::unavailable();
        }
    };
    let credential = req.data.credential.clone();
    let registered = match passkey::register_finish(&ceremony.state, &credential) {
        Ok(p) => p,
        Err(e) => {
            tracing::info!(error = %e, "passkey_register: registration refused");
            notify::audit_denied(
                &ctx.tenant_id,
                Some(&ctx.user_id),
                "passkey_registration_refused",
            );
            return mfa::error(
                400,
                "invalid_request",
                "The passkey could not be verified. Try again.",
            );
        }
    };
    let credential_id = passkey::credential_id(&registered);
    let synced = passkey::synced(&registered);
    let label = req
        .data
        .label
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map_or_else(
            || {
                if synced {
                    "Synced passkey"
                } else {
                    "Security key"
                }
                .to_string()
            },
            |l| l.chars().take(64).collect(),
        );
    let Ok(json) = serde_json::to_string(&registered) else {
        return mfa::unavailable();
    };
    let id = match factors::insert_passkey(
        &ctx.tenant_id,
        ctx.user_id,
        &label,
        &credential_id,
        &json,
        synced,
    ) {
        Ok(id) => id,
        Err(e) if e.to_string().contains("uq_user_mfa_factors_credential") => {
            return mfa::error(409, "factor_exists", "This passkey is already registered.");
        }
        Err(e) => {
            tracing::error!(error = %e, "passkey_register: store failed");
            return mfa::unavailable();
        }
    };
    notify::announce(
        &ctx.tenant_id,
        &ctx.user_id,
        &ctx.email,
        Event::PasskeyAdded,
        &label,
    );
    let now = chrono::Utc::now().timestamp();
    let factor = Factor {
        id,
        user_id: ctx.user_id,
        kind: Kind::Passkey,
        active: true,
        label,
        totp_secret_sealed: None,
        totp_last_step: None,
        credential_id: Some(credential_id),
        passkey: None,
        passkey_synced: Some(synced),
        created_at: now,
        last_used_at: None,
    };
    mfa::enrollment_done(&token, &ctx, &factor)
}
