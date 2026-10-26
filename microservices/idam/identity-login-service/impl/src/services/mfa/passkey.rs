//! Passkeys (`WebAuthn` Level 3 / FIDO2) with webauthn-rs: registration, sign-in as a second factor, and username-less
//! sign-in.
//!
//! # Relying party (ADR-008 §5)
//!
//! A passkey is bound to an RP ID, a registrable domain the browser checks against the page's origin; it is never
//! exercised on any other site, which is what makes it phishing-resistant. The RP ID is the tenant's verified domain
//! (ADR-007) or a host under it, and every origin the ceremony may run on must be that domain or a subdomain:
//!
//! * `SESAME_WEBAUTHN_RP_ID` — e.g. `pp.pricewhisperer.ai` for `PriceWhisperer` pre-production, whose hosted sign-in is
//!   `sesame-idam.pp.pricewhisperer.ai` and app `trader.pp.pricewhisperer.ai`: one RP ID covers both, so no `WebAuthn`
//!   Related Origin Requests are needed (`docs/DESIGN-mfa-passkeys.md` §4). Default: the host of
//!   `SESAME_AUTH_PUBLIC_URL`.
//! * `SESAME_WEBAUTHN_ORIGINS` — comma-separated origins allowed to run ceremonies. Default: `SESAME_AUTH_PUBLIC_URL`.
//! * `SESAME_WEBAUTHN_RP_NAME` — the name authenticators show. Default `Sesame`.
//!
//! One RP per deployment for now (each Sesame deployment serves one product domain); per-tenant RP IDs read from
//! ADR-007 verified domains are the multi-tenant follow-up.
//!
//! # Policy
//!
//! User verification is required (a biometric or PIN on the authenticator), so a passkey alone is multi-factor and
//! satisfies every MFA policy. Discoverable credentials are preferred at registration (`residentKey: preferred`; every
//! platform authenticator makes one anyway) so the passkey can sign in without a username. Attestation is not
//! requested (privacy; any authenticator is accepted).

use std::sync::LazyLock;

use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::Value;
use uuid::Uuid;
use webauthn_rs::prelude::{
    AuthenticationResult, DiscoverableAuthentication, DiscoverableKey, Passkey,
    PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    Url, Webauthn, WebauthnBuilder,
};

/// The deployment's relying-party settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpConfig {
    pub rp_id: String,
    pub rp_name: String,
    pub origins: Vec<String>,
}

impl RpConfig {
    /// From the environment (see the module docs).
    #[must_use]
    pub fn from_env() -> Self {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let public = env("SESAME_AUTH_PUBLIC_URL").unwrap_or_else(|| "http://localhost".into());
        let public_host = Url::parse(&public)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| "localhost".into());
        let origins = env("SESAME_WEBAUTHN_ORIGINS").map_or_else(
            || vec![public.trim_end_matches('/').to_string()],
            |v| {
                v.split(',')
                    .map(|o| o.trim().trim_end_matches('/').to_string())
                    .filter(|o| !o.is_empty())
                    .collect()
            },
        );
        Self {
            rp_id: env("SESAME_WEBAUTHN_RP_ID").unwrap_or(public_host),
            rp_name: env("SESAME_WEBAUTHN_RP_NAME").unwrap_or_else(|| "Sesame".into()),
            origins,
        }
    }

    /// The webauthn-rs instance. Fails when an origin is not the RP ID or under it.
    ///
    /// # Errors
    /// Malformed origins, or an origin outside the RP ID.
    pub fn build(&self) -> Result<Webauthn> {
        let mut origins = self.origins.iter().map(|o| {
            let url = Url::parse(o).with_context(|| format!("WebAuthn origin {o:?}"))?;
            let host = url.host_str().unwrap_or_default();
            if host != self.rp_id && !host.ends_with(&format!(".{}", self.rp_id)) {
                return Err(anyhow!(
                    "WebAuthn origin {o} is not {rp} or a subdomain of it",
                    rp = self.rp_id
                ));
            }
            Ok(url)
        });
        let first = origins
            .next()
            .ok_or_else(|| anyhow!("no WebAuthn origin configured"))??;
        let mut builder = WebauthnBuilder::new(&self.rp_id, &first)
            .map_err(|e| anyhow!("WebAuthn RP configuration: {e:?}"))?
            .rp_name(&self.rp_name);
        for origin in origins {
            builder = builder.append_allowed_origin(&origin?);
        }
        builder
            .build()
            .map_err(|e| anyhow!("WebAuthn RP configuration: {e:?}"))
    }
}

static WEBAUTHN: LazyLock<std::result::Result<Webauthn, String>> = LazyLock::new(|| {
    let config = RpConfig::from_env();
    match config.build() {
        Ok(w) => {
            tracing::info!(rp_id = %config.rp_id, origins = ?config.origins, "WebAuthn relying party configured");
            Ok(w)
        }
        Err(e) => {
            tracing::error!(error = %e, "WebAuthn relying party misconfigured: passkeys are unavailable");
            Err(e.to_string())
        }
    }
});

fn webauthn() -> Result<&'static Webauthn> {
    WEBAUTHN.as_ref().map_err(|e| anyhow!("{e}"))
}

/// Whether passkeys can be used in this deployment.
#[must_use]
pub fn available() -> bool {
    WEBAUTHN.is_ok()
}

/// A stored passkey, parsed.
///
/// # Errors
/// The stored JSON is not a webauthn-rs passkey.
pub fn parse(stored: &str) -> Result<Passkey> {
    serde_json::from_str(stored).context("stored passkey")
}

/// A passkey's credential id as stored and looked up (base64url, no padding).
#[must_use]
pub fn credential_id(passkey: &Passkey) -> String {
    URL_SAFE_NO_PAD.encode(passkey.cred_id().as_ref() as &[u8])
}

/// Whether the authenticator said the credential can be backed up / synced (a "synced passkey", `swk`), from the
/// stored credential's own flags.
#[must_use]
pub fn synced(passkey: &Passkey) -> bool {
    serde_json::to_value(passkey)
        .ok()
        .and_then(|v| v.pointer("/cred/backup_eligible").and_then(Value::as_bool))
        .unwrap_or(false)
}

/// Options for `navigator.credentials.create()` and the state to keep. `exclude` are the user's existing passkeys
/// (an authenticator that already holds one refuses to make a second).
///
/// # Errors
/// Passkeys unavailable, or webauthn-rs refused.
pub fn register_options(
    user_id: Uuid,
    user_name: &str,
    display_name: &str,
    exclude: &[Passkey],
) -> Result<(Value, Value)> {
    let exclude = exclude
        .iter()
        .map(|p| p.cred_id().clone())
        .collect::<Vec<_>>();
    let (ccr, state) = webauthn()?
        .start_passkey_registration(
            user_id,
            user_name,
            display_name,
            (!exclude.is_empty()).then_some(exclude),
        )
        .map_err(|e| anyhow!("start passkey registration: {e:?}"))?;
    let mut options = serde_json::to_value(&ccr)?;
    // Ask for a discoverable credential where the authenticator can make one, so the passkey also signs in without a
    // username (webauthn-rs defaults to "discouraged" to spare old security keys' storage; it does not check this on
    // completion).
    if let Some(sel) = options.pointer_mut("/publicKey/authenticatorSelection") {
        sel["residentKey"] = Value::from("preferred");
        sel["requireResidentKey"] = Value::from(false);
    }
    Ok((options, serde_json::to_value(&state)?))
}

/// Complete a registration: the verified passkey.
///
/// # Errors
/// Unavailable, malformed input, or the browser's response did not verify (wrong origin, challenge or RP ID).
pub fn register_finish(state: &Value, credential: &Value) -> Result<Passkey> {
    let state: PasskeyRegistration =
        serde_json::from_value(state.clone()).context("registration state")?;
    let credential: RegisterPublicKeyCredential =
        serde_json::from_value(credential.clone()).context("registration response")?;
    webauthn()?
        .finish_passkey_registration(&credential, &state)
        .map_err(|e| anyhow!("passkey registration did not verify: {e:?}"))
}

/// Options for `navigator.credentials.get()` limited to the user's passkeys (passkey as the second factor).
///
/// # Errors
/// Unavailable or no passkeys.
pub fn authenticate_options(passkeys: &[Passkey]) -> Result<(Value, Value)> {
    let (rcr, state) = webauthn()?
        .start_passkey_authentication(passkeys)
        .map_err(|e| anyhow!("start passkey authentication: {e:?}"))?;
    Ok((serde_json::to_value(&rcr)?, serde_json::to_value(&state)?))
}

/// Complete a second-factor passkey authentication against the user's passkeys.
///
/// # Errors
/// Malformed input, or the assertion did not verify.
pub fn authenticate_finish(state: &Value, credential: &Value) -> Result<AuthenticationResult> {
    let state: PasskeyAuthentication =
        serde_json::from_value(state.clone()).context("authentication state")?;
    let credential: PublicKeyCredential =
        serde_json::from_value(credential.clone()).context("authentication response")?;
    webauthn()?
        .finish_passkey_authentication(&credential, &state)
        .map_err(|e| anyhow!("passkey authentication did not verify: {e:?}"))
}

/// Options for a username-less sign-in: no allow-list, the browser offers the user's passkeys for this RP. The
/// `mediation` member says `conditional` (autofill); a page offering a button removes it.
///
/// # Errors
/// Unavailable.
pub fn discoverable_options() -> Result<(Value, Value)> {
    let (rcr, state) = webauthn()?
        .start_discoverable_authentication()
        .map_err(|e| anyhow!("start discoverable authentication: {e:?}"))?;
    Ok((serde_json::to_value(&rcr)?, serde_json::to_value(&state)?))
}

/// The user handle and credential id a username-less response names (before it is verified).
///
/// # Errors
/// Malformed input, or the response carries no user handle.
pub fn discoverable_identify(credential: &Value) -> Result<(Uuid, String)> {
    let credential: PublicKeyCredential =
        serde_json::from_value(credential.clone()).context("authentication response")?;
    let (user, cred) = webauthn()?
        .identify_discoverable_authentication(&credential)
        .map_err(|e| anyhow!("passkey response names no user: {e:?}"))?;
    Ok((user, URL_SAFE_NO_PAD.encode(cred)))
}

/// Verify a username-less response against the passkey it named.
///
/// # Errors
/// Malformed input, or the assertion did not verify.
pub fn discoverable_finish(
    state: &Value,
    credential: &Value,
    passkey: &Passkey,
) -> Result<AuthenticationResult> {
    let state: DiscoverableAuthentication =
        serde_json::from_value(state.clone()).context("authentication state")?;
    let credential: PublicKeyCredential =
        serde_json::from_value(credential.clone()).context("authentication response")?;
    webauthn()?
        .finish_discoverable_authentication(&credential, state, &[DiscoverableKey::from(passkey)])
        .map_err(|e| anyhow!("passkey authentication did not verify: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(rp: &str, origins: &[&str]) -> RpConfig {
        RpConfig {
            rp_id: rp.into(),
            rp_name: "PriceWhisperer".into(),
            origins: origins.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// `PriceWhisperer` pp: the hosted sign-in and the trader app share one RP ID; no Related Origin Requests.
    #[test]
    fn one_rp_id_covers_the_auth_host_and_the_app() {
        let c = config(
            "pp.pricewhisperer.ai",
            &[
                "https://sesame-idam.pp.pricewhisperer.ai",
                "https://trader.pp.pricewhisperer.ai",
            ],
        );
        assert!(c.build().is_ok());
    }

    #[test]
    fn an_origin_outside_the_rp_id_is_refused() {
        for origins in [
            &[
                "https://sesame-idam.pp.pricewhisperer.ai",
                "https://evil.example",
            ][..],
            &["https://pricewhisperer.ai"][..],
            &["https://notpp.pricewhisperer.ai"][..],
        ] {
            assert!(
                config("pp.pricewhisperer.ai", origins).build().is_err(),
                "{origins:?}"
            );
        }
        assert!(config("pp.pricewhisperer.ai", &[]).build().is_err());
    }

    #[test]
    fn registration_options_prefer_a_discoverable_credential_and_require_verification() {
        let w = config("localhost", &["http://localhost:8080"])
            .build()
            .unwrap();
        let (ccr, _state) = w
            .start_passkey_registration(Uuid::new_v4(), "a@b.co", "A", None)
            .unwrap();
        let mut options = serde_json::to_value(&ccr).unwrap();
        if let Some(sel) = options.pointer_mut("/publicKey/authenticatorSelection") {
            sel["residentKey"] = Value::from("preferred");
        }
        assert_eq!(
            options.pointer("/publicKey/authenticatorSelection/userVerification"),
            Some(&Value::from("required"))
        );
        assert_eq!(
            options.pointer("/publicKey/rp/id"),
            Some(&Value::from("localhost"))
        );
    }
}
