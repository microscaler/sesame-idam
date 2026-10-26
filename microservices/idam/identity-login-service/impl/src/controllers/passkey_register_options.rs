// BRRTRouter: user-owned

//! `POST /auth/passkeys/register/options` — `WebAuthn` creation options to add a passkey (management session, or a
//! sign-in that must enrol). The user's existing passkeys are excluded, so an authenticator holding one is not asked
//! to make another. The user handle is the account id (a random UUID: no personal data on the authenticator).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::passkey_register_options::Request;

use crate::services::mfa::context::{self, Ceremony, CeremonyKind, Mode};
use crate::services::mfa::factors::{self, Kind};
use crate::services::mfa::{self, passkey};

#[handler(PasskeyRegisterOptionsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let ctx = match mfa::context_for(req.data.mfa_token.trim(), &[Mode::Manage, Mode::Enroll]) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if !passkey::available() {
        return mfa::error(
            503,
            "mfa_unavailable",
            "Passkeys are not available here yet.",
        );
    }
    let existing = match factors::list_active(&ctx.tenant_id, ctx.user_id) {
        Ok(list) => list
            .into_iter()
            .filter(|f| f.kind == Kind::Passkey)
            .filter_map(|f| f.passkey.as_deref().and_then(|p| passkey::parse(p).ok()))
            .collect::<Vec<_>>(),
        Err(e) => {
            tracing::error!(error = %e, "passkey_register_options: lookup failed");
            return mfa::unavailable();
        }
    };
    let (options, state) =
        match passkey::register_options(ctx.user_id, &ctx.email, &ctx.email, &existing) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "passkey_register_options: options failed");
                return mfa::unavailable();
            }
        };
    let ceremony = Ceremony {
        kind: CeremonyKind::Register,
        tenant_id: ctx.tenant_id.clone(),
        user_id: Some(ctx.user_id),
        client_id: None,
        state,
    };
    match context::store_ceremony(&ceremony) {
        Ok(challenge_id) => HttpJson::ok(serde_json::json!({
            "challenge_id": challenge_id,
            "options": options,
        })),
        Err(e) => {
            tracing::error!(error = %e, "passkey_register_options: ceremony store failed");
            mfa::unavailable()
        }
    }
}
