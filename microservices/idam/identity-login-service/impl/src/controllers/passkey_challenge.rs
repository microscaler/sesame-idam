// BRRTRouter: user-owned

//! `POST /auth/passkeys/challenge` — `WebAuthn` request options for a passkey sign-in.
//!
//! * With `mfa_token` (a sign-in in `challenge` mode that offers `passkey`): the options list that account's passkeys.
//! * Without, with `client_id`: a username-less sign-in for the client's tenant. The browser offers the user's
//!   passkeys for this RP; the options say `mediation: conditional`, so the page can also offer them as autofill on
//!   the email field.
//!
//! The ceremony state is kept server-side for 5 minutes and answered once ([`context::take_ceremony`]).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::passkey_challenge::Request;

use crate::services::client_registry::ClientRegistry;
use crate::services::mfa::context::{self, Ceremony, CeremonyKind, Mode};
use crate::services::mfa::factors::{self, Kind};
use crate::services::mfa::{self, passkey};
use crate::services::tenant_service::TenantService;

#[handler(PasskeyChallengeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    if !passkey::available() {
        return mfa::error(
            503,
            "mfa_unavailable",
            "Passkeys are not available here yet.",
        );
    }
    let mfa_token = req
        .data
        .mfa_token
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty());
    let client_id = req
        .data
        .client_id
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty());

    let (ceremony, options) = if let Some(token) = mfa_token {
        let ctx = match mfa::context_for(token, &[Mode::Challenge]) {
            Ok(ctx) => ctx,
            Err(response) => return response,
        };
        match mfa::offered(&ctx) {
            Ok(o) if o.contains(&"passkey") => {}
            Ok(_) => {
                return mfa::error(
                    400,
                    "invalid_request",
                    "This sign-in does not use a passkey.",
                );
            }
            Err(e) => {
                tracing::error!(error = %e, "passkey_challenge: factor lookup failed");
                return mfa::unavailable();
            }
        }
        let passkeys = match factors::list_active(&ctx.tenant_id, ctx.user_id) {
            Ok(list) => list
                .into_iter()
                .filter(|f| f.kind == Kind::Passkey)
                .filter_map(|f| f.passkey.as_deref().and_then(|p| passkey::parse(p).ok()))
                .collect::<Vec<_>>(),
            Err(e) => {
                tracing::error!(error = %e, "passkey_challenge: factor lookup failed");
                return mfa::unavailable();
            }
        };
        if passkeys.is_empty() {
            return mfa::error(400, "invalid_request", "This account has no passkey.");
        }
        let (options, state) = match passkey::authenticate_options(&passkeys) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "passkey_challenge: options failed");
                return mfa::unavailable();
            }
        };
        (
            Ceremony {
                kind: CeremonyKind::Authenticate,
                tenant_id: ctx.tenant_id,
                user_id: Some(ctx.user_id),
                client_id: None,
                state,
            },
            options,
        )
    } else if let Some(client_id) = client_id {
        let exec = sesame_idam_database::db();
        let Ok(binding) = ClientRegistry::resolve_pre_auth(client_id, None, exec) else {
            return mfa::error(401, "invalid_client", "Unknown or inactive client");
        };
        if TenantService::require_active(binding.tenant_id.trim(), exec).is_err() {
            return mfa::error(401, "invalid_client", "Unknown or inactive client");
        }
        let (options, state) = match passkey::discoverable_options() {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "passkey_challenge: options failed");
                return mfa::unavailable();
            }
        };
        (
            Ceremony {
                kind: CeremonyKind::Discoverable,
                tenant_id: binding.tenant_id,
                user_id: None,
                client_id: Some(binding.client_id),
                state,
            },
            options,
        )
    } else {
        return mfa::error(400, "invalid_request", "client_id or mfa_token is required");
    };

    match context::store_ceremony(&ceremony) {
        Ok(challenge_id) => HttpJson::ok(serde_json::json!({
            "challenge_id": challenge_id,
            "options": options,
        })),
        Err(e) => {
            tracing::error!(error = %e, "passkey_challenge: ceremony store failed");
            mfa::unavailable()
        }
    }
}
