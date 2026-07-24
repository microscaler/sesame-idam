//! Abuse guard — staging-hardening Gates A2 + A3 (`TASKS-staging-hardening.md`).
//!
//! A2: per-identity failed-attempt tracking with progressive lockout for
//! password login (and OTP verify, when those flows land). A3: OTP/magic-link
//! send-abuse and toll-fraud controls — per-recipient windows, rapid-resend
//! dedupe, tenant SMS opt-in (ADR-008), and a global daily SMS spend ceiling.
//!
//! Backed by Redis (same instance the refresh-token machinery uses). All
//! checks FAIL OPEN on Redis errors with a warning: a Redis outage already
//! breaks token issuance, so failing closed here would only change *which*
//! error the user sees while masking the real problem.
//!
//! Identifiers are normalised (trim + lowercase) and SHA-256-hashed into the
//! Redis keys, so no raw email/phone PII lands in the keyspace.
//!
//! No-enumeration contract: callers must return the SAME response for a
//! locked identity as for wrong credentials (`invalid_credentials()`), and
//! the generic success body for capped/deduped OTP sends. The guard itself
//! only decides and audits; it never shapes responses.
//!
//! Policy is env-tunable (defaults in parentheses):
//! - `LOCKOUT_THRESHOLD` (5)      failures before a lock engages
//! - `LOCKOUT_DECAY_SECS` (900)   sliding window for the failure counter
//! - `LOCKOUT_BASE_SECS` (60)     first lock duration; doubles per failure
//! - `LOCKOUT_MAX_SECS` (3600)    lock duration ceiling
//! - `OTP_SEND_DEDUPE_SECS` (60)  suppress identical re-sends (0 = off)
//! - `OTP_SEND_MAX_PER_WINDOW` (3) sends per recipient per window
//! - `OTP_SEND_WINDOW_SECS` (300)
//! - `OTP_SEND_MAX_PER_DAY` (10)  sends per recipient per day
//! - `SMS_OPTED_IN_TENANTS` ("")  comma-separated tenant slugs allowed SMS
//!   (interim home for the ADR-008 tenant opt-in until a tenants column lands)
//! - `SMS_DAILY_SPEND_CEILING_CENTS` (1000) global daily SMS budget
//! - `SMS_COST_CENTS` (5)         accounted cost per SMS send

use redis::Commands;
use sha2::{Digest, Sha256};

use crate::audit::EMITTER;
use sesame_common::audit::{AuditEventType, AuditLogEntry};

// ── policy ──────────────────────────────────────────────────────────────────

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn lockout_threshold() -> u64 {
    env_u64("LOCKOUT_THRESHOLD", 5)
}
fn lockout_decay_secs() -> u64 {
    env_u64("LOCKOUT_DECAY_SECS", 900)
}
fn lockout_base_secs() -> u64 {
    env_u64("LOCKOUT_BASE_SECS", 60)
}
fn lockout_max_secs() -> u64 {
    env_u64("LOCKOUT_MAX_SECS", 3600)
}

// ── shared plumbing ─────────────────────────────────────────────────────────

fn connection() -> anyhow::Result<redis::Connection> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let client = redis::Client::open(redis_url.as_str())?;
    Ok(client.get_connection()?)
}

/// Normalise + hash an identifier so keys carry no raw PII.
fn ident_hash(identifier: &str) -> String {
    let normalised = identifier.trim().to_lowercase();
    let digest = Sha256::digest(normalised.as_bytes());
    let mut hex = String::with_capacity(32);
    for b in &digest[..16] {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

fn fail_key(tenant: &str, identifier: &str) -> String {
    format!("lockout:fail:{tenant}:{}", ident_hash(identifier))
}

fn lock_key(tenant: &str, identifier: &str) -> String {
    format!("lockout:lock:{tenant}:{}", ident_hash(identifier))
}

// ── A2: lockout / progressive backoff ───────────────────────────────────────

/// Outcome of recording a failed authentication attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureOutcome {
    /// Below threshold — counted, no lock.
    Counted { failures: u64 },
    /// Threshold crossed (or extended) — identity is now locked.
    Locked { failures: u64, lock_secs: u64 },
}

/// Is this identity currently locked out? Returns remaining lock seconds.
///
/// Call BEFORE credential verification; on `Some`, return the exact same
/// generic 401 as wrong credentials.
pub fn login_locked(tenant: &str, identifier: &str) -> Option<u64> {
    let mut conn = match connection() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "abuse_guard: redis unavailable — lockout check skipped");
            return None;
        }
    };
    match conn.ttl::<_, i64>(lock_key(tenant, identifier)) {
        // -2 = no key, -1 = no TTL (shouldn't happen; treat as locked briefly)
        Ok(ttl) if ttl > 0 => Some(u64::try_from(ttl).unwrap_or(0)),
        Ok(-1) => Some(1),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(error = %e, "abuse_guard: lockout check failed — skipped");
            None
        }
    }
}

/// Record a failed login/OTP-verify attempt for (tenant, identifier).
///
/// Counts failures in a sliding window; at `LOCKOUT_THRESHOLD` the identity
/// locks for `LOCKOUT_BASE_SECS`, doubling per further failure up to
/// `LOCKOUT_MAX_SECS`. Emits an audit event on every lock (transition and
/// extension) — lockouts are exactly the threat signal Gate C wants to see.
pub fn record_login_failure(tenant: &str, identifier: &str) -> FailureOutcome {
    let mut conn = match connection() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "abuse_guard: redis unavailable — failure not counted");
            return FailureOutcome::Counted { failures: 0 };
        }
    };

    let fk = fail_key(tenant, identifier);
    let failures: u64 = match conn.incr(&fk, 1u64) {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "abuse_guard: failure counter incr failed");
            return FailureOutcome::Counted { failures: 0 };
        }
    };
    // Sliding decay: quiet time clears the slate.
    let _: Result<(), _> = conn.expire(&fk, i64::try_from(lockout_decay_secs()).unwrap_or(900));

    let threshold = lockout_threshold();
    if failures < threshold {
        return FailureOutcome::Counted { failures };
    }

    // Progressive backoff: base * 2^(failures - threshold), capped.
    let exponent = (failures - threshold).min(16) as u32;
    let lock_secs = lockout_base_secs()
        .saturating_mul(1u64 << exponent)
        .min(lockout_max_secs());
    let lk = lock_key(tenant, identifier);
    if let Err(e) = conn.set_ex::<_, _, ()>(&lk, failures, lock_secs) {
        tracing::warn!(error = %e, "abuse_guard: lock write failed");
        return FailureOutcome::Counted { failures };
    }

    emit_guard_audit(
        tenant,
        "account_lockout",
        &format!("locked_after_{failures}_failures_for_{lock_secs}s"),
    );
    FailureOutcome::Locked {
        failures,
        lock_secs,
    }
}

/// Clear the failure counter after a successful authentication.
///
/// Deliberately does NOT clear an active lock: a correct password during a
/// lock window is still denied (the caller checks `login_locked` first), and
/// the lock must run its course.
pub fn record_login_success(tenant: &str, identifier: &str) {
    let Ok(mut conn) = connection() else { return };
    let _: Result<(), _> = conn.del(fail_key(tenant, identifier));
}

// ── A3: OTP / magic-link send abuse + toll-fraud ────────────────────────────

/// Delivery channel for an OTP or magic-link send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Email,
    Sms,
}

impl Channel {
    fn as_str(self) -> &'static str {
        match self {
            Channel::Email => "email",
            Channel::Sms => "sms",
        }
    }
}

/// Decision for a requested send. Callers return the SAME generic success
/// response for every variant — suppression is silent to the caller of the
/// API, loud in the audit log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendDecision {
    /// Send it.
    Allow,
    /// Identical send within the dedupe window — drop silently.
    Deduped,
    /// Per-recipient window/day cap hit — drop, audited.
    Capped,
    /// SMS to a tenant that has not opted in (ADR-008) — drop, audited.
    TenantNotOptedIn,
    /// Global daily SMS spend ceiling reached — drop, audited.
    BudgetExhausted,
}

impl SendDecision {
    /// Whether the caller should actually dispatch to the provider.
    #[must_use]
    pub fn should_send(self) -> bool {
        matches!(self, SendDecision::Allow)
    }
}

/// Gate one OTP / magic-link send request. Applies, in order: tenant SMS
/// opt-in (SMS only), rapid-resend dedupe, per-recipient window cap,
/// per-recipient daily cap, global SMS budget (SMS only). Records the send
/// against every cap when allowed.
pub fn gate_otp_send(tenant: &str, channel: Channel, recipient: &str) -> SendDecision {
    let mut conn = match connection() {
        Ok(c) => c,
        Err(e) => {
            // Fail open for email (bounded harm), fail CLOSED for SMS —
            // an unmetered SMS path is exactly the toll-fraud scenario.
            tracing::warn!(error = %e, "abuse_guard: redis unavailable for otp send gate");
            return match channel {
                Channel::Email => SendDecision::Allow,
                Channel::Sms => SendDecision::BudgetExhausted,
            };
        }
    };

    if channel == Channel::Sms && !tenant_sms_enabled(tenant) {
        emit_guard_audit(tenant, "otp_send_guard", "sms_tenant_not_opted_in");
        return SendDecision::TenantNotOptedIn;
    }

    let ch = channel.as_str();
    let rh = ident_hash(recipient);

    // Dedupe: swallow identical rapid re-sends without burning window quota.
    let dedupe_secs = env_u64("OTP_SEND_DEDUPE_SECS", 60);
    if dedupe_secs > 0 {
        let dk = format!("otpsend:dedupe:{tenant}:{ch}:{rh}");
        let set: Result<Option<String>, _> = redis::cmd("SET")
            .arg(&dk)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(dedupe_secs)
            .query(&mut conn);
        match set {
            Ok(None) => return SendDecision::Deduped,
            Ok(Some(_)) => {}
            Err(e) => tracing::warn!(error = %e, "abuse_guard: dedupe check failed"),
        }
    }

    // Per-recipient window cap.
    let wk = format!("otpsend:win:{tenant}:{ch}:{rh}");
    let window_secs = env_u64("OTP_SEND_WINDOW_SECS", 300);
    let max_window = env_u64("OTP_SEND_MAX_PER_WINDOW", 3);
    match bounded_incr(&mut conn, &wk, window_secs) {
        Some(n) if n > max_window => {
            emit_guard_audit(tenant, "otp_send_guard", "recipient_window_cap");
            return SendDecision::Capped;
        }
        _ => {}
    }

    // Per-recipient daily cap.
    let dk = format!("otpsend:day:{tenant}:{ch}:{rh}");
    let max_day = env_u64("OTP_SEND_MAX_PER_DAY", 10);
    match bounded_incr(&mut conn, &dk, 86_400) {
        Some(n) if n > max_day => {
            emit_guard_audit(tenant, "otp_send_guard", "recipient_daily_cap");
            return SendDecision::Capped;
        }
        _ => {}
    }

    // Global daily SMS spend ceiling (toll fraud backstop).
    if channel == Channel::Sms {
        let ceiling = env_u64("SMS_DAILY_SPEND_CEILING_CENTS", 1000);
        let cost = env_u64("SMS_COST_CENTS", 5);
        let day = chrono::Utc::now().format("%Y%m%d");
        // SMS_SPEND_SCOPE namespaces the (otherwise global) budget key —
        // "global" in production; tests set a unique scope for isolation.
        let scope =
            std::env::var("SMS_SPEND_SCOPE").unwrap_or_else(|_| "global".to_string());
        let sk = format!("smsspend:{scope}:{day}");
        match conn.incr::<_, _, u64>(&sk, cost) {
            Ok(spent) => {
                let _: Result<(), _> = conn.expire(&sk, 172_800);
                if spent > ceiling {
                    emit_guard_audit(tenant, "otp_send_guard", "sms_daily_budget_exhausted");
                    return SendDecision::BudgetExhausted;
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "abuse_guard: sms spend accounting failed — send blocked");
                return SendDecision::BudgetExhausted;
            }
        }
    }

    SendDecision::Allow
}

/// INCR with expiry (set on first increment). Returns the new count, or
/// `None` on Redis errors (caller treats as allow — fail open).
fn bounded_incr(conn: &mut redis::Connection, key: &str, ttl_secs: u64) -> Option<u64> {
    match conn.incr::<_, _, u64>(key, 1u64) {
        Ok(n) => {
            if n == 1 {
                let _: Result<(), _> = conn.expire(key, i64::try_from(ttl_secs).unwrap_or(300));
            }
            Some(n)
        }
        Err(e) => {
            tracing::warn!(error = %e, key, "abuse_guard: counter incr failed");
            None
        }
    }
}

/// ADR-008 interim: SMS is allowed only for tenants listed in
/// `SMS_OPTED_IN_TENANTS` (comma-separated slugs). Replaced by a tenants
/// column / tenant-settings model when the ADR-008 ceremony lands.
fn tenant_sms_enabled(tenant: &str) -> bool {
    std::env::var("SMS_OPTED_IN_TENANTS")
        .unwrap_or_default()
        .split(',')
        .any(|t| t.trim().eq_ignore_ascii_case(tenant.trim()) && !t.trim().is_empty())
}

// ── audit ───────────────────────────────────────────────────────────────────

/// Lockouts and cap denials are security events (Gate C threat signals) —
/// always emitted as denied `ValidationFailed`.
fn emit_guard_audit(tenant: &str, source: &str, reason: &str) {
    match AuditLogEntry::new(AuditEventType::ValidationFailed, "identity-login-service")
        .tenant_id(tenant.to_string())
        .decision_source(source.to_string())
        .result("denied")
        .reason(reason.to_string())
        .build()
    {
        Ok(entry) => EMITTER.emit(entry),
        Err(e) => tracing::warn!(error = %e, "abuse_guard: audit entry build failed"),
    }
}
