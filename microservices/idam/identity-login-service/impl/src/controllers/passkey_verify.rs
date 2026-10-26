// BRRTRouter: user-owned

//! `POST /auth/passkeys/verify` — finish a passkey sign-in (`challenge_id` from `/auth/passkeys/challenge`).
//!
//! * Second factor (`mfa_token`): the assertion must come from one of that account's passkeys; the sign-in's methods
//!   gain the passkey and the gate completes it.
//! * Username-less: the response names its credential and user handle; the credential must be an active passkey of
//!   that user in the ceremony's tenant, and the account active. A passkey verified with user verification is
//!   phishing-resistant MFA on its own, so this signs in without a password.
//!
//! The signature counter and backup flags are updated after every use (webauthn-rs refuses a counter that went
//! backwards: a cloned authenticator).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_common::authn::Method;
use sesame_idam_identity_login_service_gen::handlers::passkey_verify::Request;
use webauthn_rs::prelude::AuthenticationResult;

use crate::services::abuse_guard;
use crate::services::client_registry::ClientRegistry;
use crate::services::mfa::context::{self, CeremonyKind, Mode};
use crate::services::mfa::factors::{self, Factor, Kind};
use crate::services::mfa::notify;
use crate::services::mfa::{self, passkey, Subject};
use crate::services::user_service::{UserService, STATUS_ACTIVE};

#[handler(PasskeyVerifyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let ceremony = match context::take_ceremony(req.data.challenge_id.trim()) {
        Ok(Some(c)) => c,
        Ok(None) => return mfa::invalid_token(),
        Err(e) => {
            tracing::error!(error = %e, "passkey_verify: ceremony store unavailable");
            return mfa::unavailable();
        }
    };
    let credential = req.data.credential.clone();

    match ceremony.kind {
        CeremonyKind::Authenticate => {
            let Some(token) = req.data.mfa_token.as_deref().map(str::trim) else {
                return mfa::invalid_token();
            };
            let mut ctx = match mfa::context_for(token, &[Mode::Challenge]) {
                Ok(ctx) => ctx,
                Err(response) => return response,
            };
            if Some(ctx.user_id) != ceremony.user_id || ctx.tenant_id != ceremony.tenant_id {
                return mfa::invalid_token();
            }
            let guard = mfa::guard_identity(&ctx.user_id);
            if abuse_guard::login_locked(&ctx.tenant_id, &guard).is_some() {
                return mfa::error(
                    401,
                    "account_locked",
                    "Too many attempts. Wait a few minutes and try again.",
                );
            }
            let result = match passkey::authenticate_finish(&ceremony.state, &credential) {
                Ok(r) => r,
                Err(e) => {
                    tracing::info!(error = %e, "passkey_verify: assertion refused");
                    abuse_guard::record_login_failure(&ctx.tenant_id, &guard);
                    notify::audit_denied(
                        &ctx.tenant_id,
                        Some(&ctx.user_id),
                        "passkey_assertion_refused",
                    );
                    let _ = context::record_failure(token, &mut ctx);
                    return refused();
                }
            };
            let Some(method) = record_use(&ctx.tenant_id, ctx.user_id, &result) else {
                return refused();
            };
            abuse_guard::record_login_success(&ctx.tenant_id, &guard);
            ctx.methods.push(method);
            mfa::next_step(token, ctx)
        }
        CeremonyKind::Discoverable => {
            username_less(&ceremony, &credential, req.data.organization_id.clone())
        }
        CeremonyKind::Register => mfa::invalid_token(),
    }
}

fn username_less(
    ceremony: &context::Ceremony,
    credential: &serde_json::Value,
    organization_id: Option<String>,
) -> HttpJson<serde_json::Value> {
    let tenant_id = ceremony.tenant_id.as_str();
    let Ok((user_id, credential_id)) = passkey::discoverable_identify(credential) else {
        return refused();
    };
    let factor = match factors::find_passkey(tenant_id, &credential_id) {
        Ok(Some(f)) if f.user_id == user_id => f,
        Ok(_) => {
            notify::audit_denied(tenant_id, Some(&user_id), "passkey_unknown_credential");
            return refused();
        }
        Err(e) => {
            tracing::error!(error = %e, "passkey_verify: passkey lookup failed");
            return mfa::unavailable();
        }
    };
    let Some(stored) = factor
        .passkey
        .as_deref()
        .and_then(|p| passkey::parse(p).ok())
    else {
        return refused();
    };
    let result = match passkey::discoverable_finish(&ceremony.state, credential, &stored) {
        Ok(r) => r,
        Err(e) => {
            tracing::info!(error = %e, "passkey_verify: assertion refused");
            notify::audit_denied(tenant_id, Some(&user_id), "passkey_assertion_refused");
            return refused();
        }
    };
    let user = match sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| {
        UserService::find_by_tenant_and_id(tenant_id, user_id, exec)
    }) {
        Ok(Some(u)) if u.status == STATUS_ACTIVE => u,
        Ok(_) => return refused(),
        Err(e) => {
            tracing::error!(error = %e, "passkey_verify: user lookup failed");
            return mfa::unavailable();
        }
    };
    let Some(method) = update_passkey(tenant_id, &factor, stored, &result) else {
        return refused();
    };
    let exec = sesame_idam_database::db();
    let portal = ceremony
        .client_id
        .as_deref()
        .and_then(|c| ClientRegistry::resolve_pre_auth(c, None, exec).ok())
        .map_or_else(
            || crate::services::login_success::DEFAULT_PORTAL.to_string(),
            |b| b.portal,
        );
    mfa::after_first_factor(
        &Subject {
            tenant_id: tenant_id.to_string(),
            user_id,
            email: user.email,
            portal,
            organization_id,
        },
        method,
    )
}

/// After a second-factor passkey assertion: find which of the user's passkeys answered and update it.
fn record_use(
    tenant_id: &str,
    user_id: uuid::Uuid,
    result: &AuthenticationResult,
) -> Option<Method> {
    let used = passkey_cred_id(result);
    let factor = factors::list_active(tenant_id, user_id)
        .ok()?
        .into_iter()
        .find(|f| f.kind == Kind::Passkey && f.credential_id.as_deref() == Some(used.as_str()))?;
    let stored = passkey::parse(factor.passkey.as_deref()?).ok()?;
    update_passkey(tenant_id, &factor, stored, result)
}

fn update_passkey(
    tenant_id: &str,
    factor: &Factor,
    mut stored: webauthn_rs::prelude::Passkey,
    result: &AuthenticationResult,
) -> Option<Method> {
    let changed = stored.update_credential(result)?;
    let json = changed
        .then(|| serde_json::to_string(&stored).ok())
        .flatten();
    if let Err(e) = factors::touch_passkey(tenant_id, factor.user_id, factor.id, json.as_deref()) {
        tracing::warn!(error = %e, "passkey_verify: passkey update failed");
    }
    Some(Method::Passkey {
        user_verified: result.user_verified(),
        synced: result.backup_eligible(),
    })
}

fn passkey_cred_id(result: &AuthenticationResult) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(result.cred_id().as_ref() as &[u8])
}

fn refused() -> HttpJson<serde_json::Value> {
    mfa::error(
        401,
        "invalid_credentials",
        "That passkey could not be verified. Try again, or use another sign-in method.",
    )
}
