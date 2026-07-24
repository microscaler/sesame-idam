//! SMS delivery for transactional auth messages (phone OTP).
//!
//! NOTE (ADR-009): this module currently reads ONE process-wide credential
//! from env. The multi-tenant sender-identity + credential-custody model
//! (which Twilio account pays/sends per purpose, per-owner spend ceilings,
//! Connect vs envelope custody) is specified in
//! `docs/ADR-009-multi-tenant-sms-sender-identity-and-credential-custody.md`
//! and `docs/DESIGN-multi-tenant-sms-sender-identity.md`. `send_sms` will take
//! a resolved `SmsSender` from `resolve_sms_sender(tenant, environment,
//! purpose)` rather than env credentials once that lands.
//!
//! Provider selection via `SMS_PROVIDER` (default `mock`):
//! - `twilio` — Twilio Messages API (real, globally available). Env:
//!   `TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`, and a sender —
//!   `TWILIO_MESSAGING_SERVICE_SID` (preferred) or `TWILIO_FROM_NUMBER`.
//! - `mock` — records the message to a Redis outbox
//!   (`sms:outbox:<hash(to)>`, short TTL) instead of sending. This is the
//!   dev/CI transport and the e2e-test seam: tests read the code back from
//!   the outbox exactly as they read email from Mailpit. No creds, no spend.
//!
//! Like the SMTP client, sends FAIL CLOSED into a logged/audited error on
//! the auth path — the caller keeps the HTTP response generic (no
//! provider-status oracle), and the abuse guard has already metered the send.
//!
//! # Cost policy (purpose allow-list)
//!
//! SMS is ~100-1000× the cost of email per message and the prime toll-fraud
//! target, so it is restricted to high-value purposes only. `send_sms` takes
//! an [`SmsPurpose`]; only purposes in `SMS_ALLOWED_PURPOSES` (default
//! `registration,password_reset`) actually dispatch. Everything else —
//! notably per-login phone OTP — is refused by policy (audited, generic
//! response to the caller). Enable a purpose per-environment only with a
//! deliberate cost decision; email OTP covers routine second-factor needs.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::Engine;
use redis::Commands;
use sha2::{Digest, Sha256};

use sesame_common::{fetch_post, HttpFetchOptions};

/// Why an SMS is being sent — the unit of BOTH the cost allow-list and the
/// billing-owner map (ADR-009 §2.1, see `sms_sender::billing_owner_for`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmsPurpose {
    // ── tenant-billed: the tenant's own end-users ──
    /// Verifying an end-user's phone at account creation.
    Registration,
    /// Delivering a password-reset code/link to an end-user.
    PasswordReset,
    /// End-user changing or re-verifying their phone number.
    PhoneReverification,
    /// Per-login second factor. OFF by default (use email OTP instead).
    Login,
    /// End-user account recovery.
    AccountRecovery,

    // ── platform-billed: Sesame's own relationship ──
    /// Onboarding a TENANT to the platform (owner phone verification).
    TenantRegistration,
    /// Registering a new environment for a tenant.
    EnvironmentRegistration,
    /// Tenant OWNER recovering access to the Sesame console — platform-billed
    /// because it restores access to the tenant *on the platform*.
    TenantOwnerRecovery,
    /// Platform operator MFA / break-glass.
    PlatformOperator,
}

impl SmsPurpose {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SmsPurpose::Registration => "registration",
            SmsPurpose::PasswordReset => "password_reset",
            SmsPurpose::PhoneReverification => "phone_reverification",
            SmsPurpose::Login => "login",
            SmsPurpose::AccountRecovery => "account_recovery",
            SmsPurpose::TenantRegistration => "tenant_registration",
            SmsPurpose::EnvironmentRegistration => "environment_registration",
            SmsPurpose::TenantOwnerRecovery => "tenant_owner_recovery",
            SmsPurpose::PlatformOperator => "platform_operator",
        }
    }
}

/// Whether SMS is permitted for `purpose` under the current policy.
///
/// `SMS_ALLOWED_PURPOSES` (comma-separated) overrides the default. Defaults
/// cover the high-value purposes only: end-user registration + password
/// reset, and the platform's own onboarding/recovery paths. Per-login SMS is
/// deliberately excluded (email OTP covers routine 2FA at ~1/100th the cost).
#[must_use]
pub fn purpose_allowed(purpose: SmsPurpose) -> bool {
    let allowed = std::env::var("SMS_ALLOWED_PURPOSES").unwrap_or_else(|_| {
        "registration,password_reset,tenant_registration,environment_registration,tenant_owner_recovery,platform_operator"
            .to_string()
    });
    allowed
        .split(',')
        .map(str::trim)
        .any(|p| p.eq_ignore_ascii_case(purpose.as_str()))
}

/// Which provider `SMS_PROVIDER` selects.
fn provider() -> String {
    std::env::var("SMS_PROVIDER").unwrap_or_else(|_| "mock".to_string())
}

/// Error returned when a purpose is refused by the cost policy — distinct so
/// callers can treat it as "not sent by design", not a transport failure.
#[derive(Debug, thiserror::Error)]
#[error("SMS not permitted for purpose '{0}' (cost policy: SMS_ALLOWED_PURPOSES)")]
pub struct PurposeNotAllowed(pub &'static str);

/// Send an SMS for a given [`SmsPurpose`]. Refused (with
/// [`PurposeNotAllowed`]) when the purpose is not in the cost allow-list;
/// otherwise dispatches to the configured provider.
///
/// # Errors
///
/// [`PurposeNotAllowed`] when policy forbids the purpose, or a transport /
/// provider error on an allowed send.
pub fn send_sms(to: &str, body: &str, purpose: SmsPurpose) -> Result<()> {
    if !purpose_allowed(purpose) {
        return Err(PurposeNotAllowed(purpose.as_str()).into());
    }
    match provider().as_str() {
        "twilio" => send_twilio(to, body),
        "mock" | "" => send_mock(to, body),
        other => bail!("unknown SMS_PROVIDER '{other}' (expected twilio|mock)"),
    }
}

// ── Twilio ───────────────────────────────────────────────────────────────────

fn send_twilio(to: &str, body: &str) -> Result<()> {
    let sid = std::env::var("TWILIO_ACCOUNT_SID")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .context("TWILIO_ACCOUNT_SID not set")?;
    let token = std::env::var("TWILIO_AUTH_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .context("TWILIO_AUTH_TOKEN not set")?;

    // Sender: a Messaging Service SID (recommended — handles number pools,
    // compliance, geo-routing) or a single From number.
    let mut form: Vec<(String, String)> = vec![
        ("To".to_string(), to.to_string()),
        ("Body".to_string(), body.to_string()),
    ];
    if let Ok(mss) = std::env::var("TWILIO_MESSAGING_SERVICE_SID") {
        if !mss.trim().is_empty() {
            form.push(("MessagingServiceSid".to_string(), mss));
        }
    }
    if !form.iter().any(|(k, _)| k == "MessagingServiceSid") {
        let from = std::env::var("TWILIO_FROM_NUMBER")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .context("neither TWILIO_MESSAGING_SERVICE_SID nor TWILIO_FROM_NUMBER set")?;
        form.push(("From".to_string(), from));
    }

    let base = std::env::var("TWILIO_API_BASE")
        .unwrap_or_else(|_| "https://api.twilio.com".to_string());
    let url = format!("{base}/2010-04-01/Accounts/{sid}/Messages.json");
    let payload = form_urlencode(&form);

    let auth = base64::engine::general_purpose::STANDARD.encode(format!("{sid}:{token}"));
    let options = HttpFetchOptions {
        timeout: Duration::from_millis(
            std::env::var("SMS_TIMEOUT_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(8000),
        ),
        max_body_bytes: 64 * 1024,
        extra_headers: vec![
            ("authorization".to_string(), format!("Basic {auth}")),
            (
                "content-type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            ),
        ],
    };

    let (status, body_bytes) = fetch_post(&url, payload.as_bytes(), &options)
        .map_err(|e| anyhow::anyhow!("twilio POST: {e}"))?;
    // Twilio returns 201 Created on success.
    if !(200..300).contains(&status) {
        let snippet = String::from_utf8_lossy(&body_bytes);
        bail!("twilio send failed: HTTP {status}: {}", snippet.chars().take(200).collect::<String>());
    }
    Ok(())
}

/// Minimal application/x-www-form-urlencoded encoder (no external dep).
fn form_urlencode(pairs: &[(String, String)]) -> String {
    fn enc(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char);
                }
                b' ' => out.push('+'),
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", enc(k), enc(v)))
        .collect::<Vec<_>>()
        .join("&")
}

// ── Mock (Redis outbox) ──────────────────────────────────────────────────────

fn outbox_key(to: &str) -> String {
    let digest = Sha256::digest(to.trim().as_bytes());
    let mut hex = String::with_capacity(32);
    for b in &digest[..16] {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    format!("sms:outbox:{hex}")
}

fn send_mock(to: &str, body: &str) -> Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let client = redis::Client::open(redis_url.as_str())?;
    let mut conn = client.get_connection()?;
    let ttl = std::env::var("SMS_OUTBOX_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600u64);
    // Newest-first list so tests read [0] for the latest message.
    let key = outbox_key(to);
    conn.lpush::<_, _, ()>(&key, body)?;
    let _: Result<(), _> = conn.expire(&key, i64::try_from(ttl).unwrap_or(600));
    tracing::info!(target: "sms.mock", "mock SMS recorded to outbox (SMS_PROVIDER=mock)");
    Ok(())
}

/// Read the most recent mock-outbox message for a recipient (test helper /
/// mock inspection). Returns `None` when the provider is not `mock`, the
/// outbox is empty, or Redis is unavailable.
#[must_use]
pub fn mock_outbox_latest(to: &str) -> Option<String> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let client = redis::Client::open(redis_url.as_str()).ok()?;
    let mut conn = client.get_connection().ok()?;
    conn.lindex(outbox_key(to), 0).ok().flatten()
}
