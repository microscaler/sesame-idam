//! Short-lived server-side state for multi-step ceremonies, in Redis, addressed by opaque bearer values (32 random
//! bytes, base64url) that are only ever stored hashed.
//!
//! * **MFA context** (`mfa_token`): what a sign-in has proved so far and what it still needs, or a signed-in user's
//!   right to manage their factors. Three modes:
//!   - `challenge`: a first factor passed; a second is required. Only verification is possible.
//!   - `enroll`: a first factor (and maybe a second) passed; policy requires a method the user does not have. Only
//!     enrolment is possible, and completing it finishes the sign-in.
//!   - `manage`: issued to a signed-in user (fresh authentication, MFA if they have factors) for the security page.
//!
//!   No tokens exist until a context completes: a password alone never yields a token for an account with a second
//!   factor.
//! * **`WebAuthn` ceremony** (`challenge_id`): the webauthn-rs registration or authentication state for one browser
//!   ceremony, single use (taken with GETDEL), 5 minutes.

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use redis::Commands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use sesame_common::authn::Method;

use super::policy::Requirement;

/// A sign-in context lives 10 minutes: long enough to find a phone or enrol an authenticator app.
pub const SIGN_IN_TTL_SECS: u64 = 600;
/// A security-page context lives 15 minutes.
pub const MANAGE_TTL_SECS: u64 = 900;
/// A `WebAuthn` ceremony lives 5 minutes (the browser's own default timeout).
pub const CEREMONY_TTL_SECS: u64 = 300;
/// Wrong second-factor attempts one context tolerates before it is burned (the account lockout applies as well).
pub const MAX_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Challenge,
    Enroll,
    Manage,
}

/// The state behind an `mfa_token`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MfaContext {
    pub mode: Mode,
    pub tenant_id: String,
    pub user_id: Uuid,
    pub email: String,
    /// Portal / application the tokens are for (the registered client's portal, or `frontend`).
    pub portal: String,
    pub organization_id: Option<String>,
    /// The methods this ceremony has already proved, in order.
    pub methods: Vec<Method>,
    /// When the first method was proved (the eventual `auth_time`).
    pub auth_time: i64,
    /// What the account's policy requires.
    pub required: Requirement,
    /// Wrong attempts so far.
    #[serde(default)]
    pub attempts: u32,
}

fn redis() -> Result<sesame_common::redis_pool::PooledConnection<'static>> {
    sesame_common::redis_pool::pooled_connection().context("connect to Redis")
}

fn opaque() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn key(prefix: &str, value: &str) -> String {
    format!("{prefix}:{}", hex::encode(Sha256::digest(value.as_bytes())))
}

const CONTEXT_PREFIX: &str = "mfa:ctx";
const CEREMONY_PREFIX: &str = "mfa:webauthn";

fn ttl_for(mode: Mode) -> u64 {
    match mode {
        Mode::Manage => MANAGE_TTL_SECS,
        Mode::Challenge | Mode::Enroll => SIGN_IN_TTL_SECS,
    }
}

/// Store a new context; returns its `mfa_token` and lifetime.
///
/// # Errors
/// Redis unavailable.
pub fn create(ctx: &MfaContext) -> Result<(String, u64)> {
    let token = opaque();
    let ttl = ttl_for(ctx.mode);
    let payload = serde_json::to_string(ctx)?;
    redis()?
        .set_ex::<_, _, ()>(key(CONTEXT_PREFIX, &token), payload, ttl)
        .context("store MFA context")?;
    Ok((token, ttl))
}

/// Read a context without using it up. `Ok(None)` when absent (never issued, expired or completed).
///
/// # Errors
/// Redis unavailable or a corrupt entry.
pub fn get(token: &str) -> Result<Option<MfaContext>> {
    if token.len() < 40 || token.len() > 64 {
        return Ok(None);
    }
    let raw: Option<String> = redis()?
        .get(key(CONTEXT_PREFIX, token))
        .context("read MFA context")?;
    raw.map(|r| serde_json::from_str(&r).context("corrupt MFA context"))
        .transpose()
}

/// Replace a context's state, keeping its remaining lifetime (`KEEPTTL`).
///
/// # Errors
/// Redis unavailable.
pub fn update(token: &str, ctx: &MfaContext) -> Result<()> {
    let payload = serde_json::to_string(ctx)?;
    let _: Option<String> = redis::cmd("SET")
        .arg(key(CONTEXT_PREFIX, token))
        .arg(payload)
        .arg("XX")
        .arg("KEEPTTL")
        .query(&mut redis()?)
        .context("update MFA context")?;
    Ok(())
}

/// End a context (completed, or burned after too many wrong attempts). Returns whether it was still there: when two
/// requests complete the same context at once, only the one that deletes it may issue tokens.
///
/// # Errors
/// Redis unavailable.
pub fn finish(token: &str) -> Result<bool> {
    let n: i64 = redis()?
        .del(key(CONTEXT_PREFIX, token))
        .context("end MFA context")?;
    Ok(n == 1)
}

/// Count a wrong attempt; burns the context at [`MAX_ATTEMPTS`]. Returns whether it is still usable.
///
/// # Errors
/// Redis unavailable.
pub fn record_failure(token: &str, ctx: &mut MfaContext) -> Result<bool> {
    ctx.attempts += 1;
    if ctx.attempts >= MAX_ATTEMPTS {
        finish(token)?;
        return Ok(false);
    }
    update(token, ctx)?;
    Ok(true)
}

/// Seconds left on a context.
///
/// # Errors
/// Redis unavailable.
pub fn remaining(token: &str) -> Result<u64> {
    let ttl: i64 = redis()?
        .ttl(key(CONTEXT_PREFIX, token))
        .context("read MFA context TTL")?;
    Ok(u64::try_from(ttl).unwrap_or(0))
}

// ── WebAuthn ceremonies ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CeremonyKind {
    /// Registering a new passkey for `user_id`.
    Register,
    /// A passkey as the second factor of `user_id`'s sign-in.
    Authenticate,
    /// Username-less sign-in: any passkey of the tenant.
    Discoverable,
}

/// One browser ceremony's server-side state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ceremony {
    pub kind: CeremonyKind,
    pub tenant_id: String,
    pub user_id: Option<Uuid>,
    /// The client the ceremony was started for (username-less sign-in binds tokens to it).
    pub client_id: Option<String>,
    /// webauthn-rs state (`PasskeyRegistration`, `PasskeyAuthentication` or `DiscoverableAuthentication`) as JSON.
    pub state: serde_json::Value,
}

/// Store a ceremony; returns its `challenge_id`.
///
/// # Errors
/// Redis unavailable.
pub fn store_ceremony(ceremony: &Ceremony) -> Result<String> {
    let id = opaque();
    redis()?
        .set_ex::<_, _, ()>(
            key(CEREMONY_PREFIX, &id),
            serde_json::to_string(ceremony)?,
            CEREMONY_TTL_SECS,
        )
        .context("store WebAuthn ceremony")?;
    Ok(id)
}

/// Take (and delete) a ceremony: each challenge is answered once.
///
/// # Errors
/// Redis unavailable or a corrupt entry.
pub fn take_ceremony(challenge_id: &str) -> Result<Option<Ceremony>> {
    if challenge_id.len() < 40 || challenge_id.len() > 64 {
        return Ok(None);
    }
    let raw: Option<String> = redis::cmd("GETDEL")
        .arg(key(CEREMONY_PREFIX, challenge_id))
        .query(&mut redis()?)
        .context("take WebAuthn ceremony")?;
    raw.map(|r| serde_json::from_str(&r).context("corrupt WebAuthn ceremony"))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contexts_serialize_with_their_proven_methods() {
        let ctx = MfaContext {
            mode: Mode::Challenge,
            tenant_id: "acme".into(),
            user_id: Uuid::nil(),
            email: "a@b.co".into(),
            portal: "frontend".into(),
            organization_id: None,
            methods: vec![
                Method::Password,
                Method::Passkey {
                    user_verified: true,
                    synced: false,
                },
            ],
            auth_time: 1,
            required: Requirement::Mfa,
            attempts: 0,
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains(r#""method":"password""#), "{json}");
        assert!(json.contains(r#""required":"mfa""#));
        let back: MfaContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ctx);
    }

    #[test]
    fn opaque_values_are_long_and_keys_never_hold_them() {
        let v = opaque();
        assert_eq!(v.len(), 43);
        let k = key(CONTEXT_PREFIX, &v);
        assert!(!k.contains(&v));
        assert!(k.starts_with("mfa:ctx:"));
    }
}
