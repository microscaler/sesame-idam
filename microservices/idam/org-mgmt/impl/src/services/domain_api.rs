//! The organisation domain API (story 17.4; DESIGN-org-domain-verification.md §6), independent of HTTP.
//!
//! Every operation takes the tenant and the caller from the validated token and the organisation and claim from the
//! path; nothing a request carries can set a claim's state (only [`domain_claims`] applying a lifecycle event does).
//! Authorisation: members of the organisation list, its owners and admins change, tenant admins revoke. A missing
//! organisation, an organisation of another tenant, a caller who is not a member and a claim of another
//! organisation all answer [`DomainApiError::NotFound`]: nothing tells one organisation about another's claims.
//!
//! The controllers bind this to HTTP with the process-wide [`DomainContext`] built from the environment
//! ([`DomainContext::from_env`]); tests pass their own keyring and a mock [`DnsProbe`].

use std::time::Duration;

use chrono::{DateTime, Utc};
use lifeguard::LifeError;
use serde_json::{json, Value};
use sesame_common::domain_verify::{
    challenge_name, check_eligible, expected_record, normalize_domain, quorum, ClaimState,
    ClaimSubject, DomainError, EligibilityPolicy, EnrollKeyring, KeyringConfig, LifecycleError,
    LifecycleEvent, ProviderAnswer, Scope, Verdict, Withdrawer,
};
use sesame_common::{fetch_post, HttpFetchOptions};
use uuid::Uuid;

use crate::services::domain_claims::{
    apply_event, create_claim_with_id, list_claims, load_claim, open_claims_on, record_verdict,
    ClaimError, StoredClaim,
};
use crate::services::org_lifecycle::{self, OrgLifecycleError};

/// Fewest seconds between two checks of one claim requested through the API.
pub const VERIFY_INTERVAL_SECS: i64 = 60;

/// Base URL of `dns-probe` (its `servers` entry: `http://dns-probe:8080/dns-probe/v1`).
pub const DNS_PROBE_URL_ENV: &str = "SESAME_DNS_PROBE_URL";
/// File naming the `K_enroll` keyring (`KeyringConfig` JSON).
pub const KEYRING_ENV: &str = "SESAME_DOMAIN_VERIFY_KEYRING";
/// The deployment's own domains (comma-separated); never claimable.
pub const RESERVED_ENV: &str = "SESAME_DOMAIN_RESERVED";
/// Consumer mail providers beyond the built-in list (comma-separated, `label.*` allowed).
pub const EXTRA_PROVIDERS_ENV: &str = "SESAME_DOMAIN_EXTRA_PROVIDERS";

const DEFAULT_DNS_PROBE_URL: &str = "http://dns-probe:8080/dns-probe/v1";

/// Why an operation failed. Each maps to one status and one stable code ([`Self::status`], [`Self::code`]).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainApiError {
    /// No such organisation in the tenant, not a member, or no such claim in it.
    #[error("not found")]
    NotFound,
    /// A member without the role the operation needs.
    #[error("forbidden")]
    Forbidden,
    /// Not a domain name.
    #[error("invalid domain")]
    InvalidDomain,
    /// A malformed field other than the domain (fixed text).
    #[error("invalid request: {0}")]
    InvalidRequest(&'static str),
    /// Not claimable, or held by another organisation of the tenant: deliberately one answer.
    #[error("domain unavailable")]
    DomainUnavailable,
    /// The organisation already has an open claim on the domain.
    #[error("claim exists")]
    ClaimExists(Uuid),
    /// Auto-join on a claim that is not VERIFIED.
    #[error("not verified")]
    NotVerified,
    /// The claim is REVOKED, EXPIRED or CONFLICT.
    #[error("claim final")]
    ClaimFinal,
    /// Checked less than [`VERIFY_INTERVAL_SECS`] ago.
    #[error("rate limited")]
    RateLimited {
        /// Seconds until the next check is allowed.
        retry_after: i64,
    },
    /// The verification key cannot be used (unconfigured, or the key service failed).
    #[error("keyring unavailable")]
    KeyringUnavailable,
    /// `dns-probe` could not be asked.
    #[error("dns unavailable")]
    DnsUnavailable,
    /// Database or stored-data failure (logged, never shown).
    #[error("internal: {0}")]
    Internal(String),
}

impl DomainApiError {
    /// HTTP status.
    #[must_use]
    pub const fn status(&self) -> u16 {
        match self {
            Self::NotFound => 404,
            Self::Forbidden => 403,
            Self::InvalidDomain | Self::InvalidRequest(_) => 400,
            Self::DomainUnavailable => 422,
            Self::ClaimExists(_) | Self::NotVerified | Self::ClaimFinal => 409,
            Self::RateLimited { .. } => 429,
            Self::KeyringUnavailable | Self::DnsUnavailable => 503,
            Self::Internal(_) => 500,
        }
    }

    /// Stable machine-readable code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::Forbidden => "forbidden",
            Self::InvalidDomain => "invalid_domain",
            Self::InvalidRequest(_) => "invalid_request",
            Self::DomainUnavailable => "domain_unavailable",
            Self::ClaimExists(_) => "claim_exists",
            Self::NotVerified => "not_verified",
            Self::ClaimFinal => "claim_final",
            Self::RateLimited { .. } => "rate_limited",
            Self::KeyringUnavailable => "keyring_unavailable",
            Self::DnsUnavailable => "dns_unavailable",
            Self::Internal(_) => "internal_error",
        }
    }

    /// The response body (`error`, `message`, and for `claim_exists` the existing claim's id).
    #[must_use]
    pub fn body(&self) -> Value {
        let message = match self {
            Self::NotFound => "Not found",
            Self::Forbidden => "Organisation admin role required",
            Self::InvalidDomain => "Not a valid domain name",
            Self::InvalidRequest(text) => text,
            Self::DomainUnavailable => "This domain cannot be claimed by this organisation",
            Self::ClaimExists(_) => "The organisation already has an open claim on this domain",
            Self::NotVerified => "Only a verified domain can be used for auto-join",
            Self::ClaimFinal => "This claim is closed; claim the domain again",
            Self::RateLimited { .. } => "Checked less than a minute ago",
            Self::KeyringUnavailable | Self::DnsUnavailable => {
                "Verification is temporarily unavailable; nothing was changed"
            }
            Self::Internal(_) => "An unexpected error occurred",
        };
        let mut body = json!({ "error": self.code(), "message": message });
        match self {
            Self::ClaimExists(id) => body["claim_id"] = json!(id.to_string()),
            Self::RateLimited { retry_after } => body["retry_after"] = json!(retry_after),
            _ => {}
        }
        body
    }
}

impl From<ClaimError> for DomainApiError {
    fn from(e: ClaimError) -> Self {
        match e {
            ClaimError::NotFound => Self::NotFound,
            ClaimError::DomainUnavailable => Self::DomainUnavailable,
            ClaimError::Lifecycle(LifecycleError::Final(_)) => Self::ClaimFinal,
            ClaimError::Lifecycle(LifecycleError::NotApplicable(_)) => Self::NotVerified,
            ClaimError::Db(text) => Self::Internal(text),
            ClaimError::Malformed(what) => Self::Internal(format!("malformed claim: {what}")),
        }
    }
}

impl From<LifeError> for DomainApiError {
    fn from(e: LifeError) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<OrgLifecycleError> for DomainApiError {
    fn from(e: OrgLifecycleError) -> Self {
        match e {
            OrgLifecycleError::NotFound | OrgLifecycleError::Forbidden => Self::NotFound,
            other => Self::Internal(other.to_string()),
        }
    }
}

/// Looks up the TXT records at a challenge name (`dns-probe`'s `lookup_challenge`).
pub trait DnsProbe: Send + Sync {
    /// Every provider's answer, as `dns-probe` reports them.
    ///
    /// # Errors
    ///
    /// A short reason when `dns-probe` cannot be asked or answers something unusable.
    fn lookup(&self, name: &str) -> Result<Vec<ProviderAnswer>, String>;
}

/// [`DnsProbe`] over HTTP, through `sesame_common::http`.
#[derive(Debug, Clone)]
pub struct HttpDnsProbe {
    base_url: String,
}

#[derive(serde::Deserialize)]
struct LookupResponse {
    answers: Vec<ProviderAnswer>,
}

impl HttpDnsProbe {
    /// A probe at `base_url` (`.../dns-probe/v1`).
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// From [`DNS_PROBE_URL_ENV`], defaulting to the in-cluster service.
    #[must_use]
    pub fn from_env() -> Self {
        let url = std::env::var(DNS_PROBE_URL_ENV)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_DNS_PROBE_URL.to_string());
        Self::new(&url)
    }
}

impl DnsProbe for HttpDnsProbe {
    fn lookup(&self, name: &str) -> Result<Vec<ProviderAnswer>, String> {
        let url = format!("{}/lookup", self.base_url);
        let body = json!({ "name": name }).to_string();
        let options = HttpFetchOptions {
            // dns-probe asks its providers in parallel with its own deadline; this bounds the whole call.
            timeout: Duration::from_secs(8),
            max_body_bytes: 64 * 1024,
            extra_headers: vec![("content-type".to_string(), "application/json".to_string())],
        };
        let (status, response) =
            fetch_post(&url, body.as_bytes(), &options).map_err(|e| format!("dns-probe: {e}"))?;
        if status != 200 {
            return Err(format!("dns-probe answered {status}"));
        }
        serde_json::from_slice::<LookupResponse>(&response)
            .map(|r| r.answers)
            .map_err(|_| "dns-probe answer malformed".to_string())
    }
}

/// What the operations need besides the database: the key, the probe and the eligibility rules.
pub struct DomainContext {
    /// `K_enroll`; `None` when unconfigured or unloadable (every operation that needs it answers 503).
    pub keyring: Option<Box<dyn EnrollKeyring>>,
    /// `dns-probe`.
    pub probe: Box<dyn DnsProbe>,
    /// Which domains may be claimed.
    pub policy: EligibilityPolicy,
}

fn env_list(name: &str) -> Vec<String> {
    std::env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

impl DomainContext {
    /// Build from the environment ([`KEYRING_ENV`], [`DNS_PROBE_URL_ENV`], [`RESERVED_ENV`],
    /// [`EXTRA_PROVIDERS_ENV`]). A keyring that cannot be loaded is logged and left out (503 on use), so the rest of
    /// the service keeps working; an invalid reserved or provider entry is logged and skipped.
    #[must_use]
    pub fn from_env() -> Self {
        let keyring = match std::env::var(KEYRING_ENV)
            .ok()
            .filter(|v| !v.trim().is_empty())
        {
            None => {
                tracing::warn!(
                    "{KEYRING_ENV} unset: organisation domain claims cannot be created or checked"
                );
                None
            }
            Some(path) => match std::fs::read(&path)
                .map_err(|e| e.kind().to_string())
                .and_then(|bytes| KeyringConfig::from_json(&bytes).map_err(|e| e.to_string()))
                .and_then(|config| config.build().map_err(|e| e.to_string()))
            {
                Ok(keyring) => Some(keyring),
                Err(error) => {
                    tracing::error!(%error, "domain verification keyring could not be loaded");
                    None
                }
            },
        };

        let mut policy = EligibilityPolicy::default();
        for entry in env_list(RESERVED_ENV) {
            match policy.clone().with_reserved([entry.as_str()]) {
                Ok(next) => policy = next,
                Err(_) => tracing::error!("{RESERVED_ENV}: skipping an invalid entry"),
            }
        }
        for entry in env_list(EXTRA_PROVIDERS_ENV) {
            match policy.clone().with_providers([entry.as_str()]) {
                Ok(next) => policy = next,
                Err(_) => tracing::error!("{EXTRA_PROVIDERS_ENV}: skipping an invalid entry"),
            }
        }

        Self {
            keyring,
            probe: Box::new(HttpDnsProbe::from_env()),
            policy,
        }
    }

    fn keyring(&self) -> Result<&dyn EnrollKeyring, DomainApiError> {
        self.keyring
            .as_deref()
            .ok_or(DomainApiError::KeyringUnavailable)
    }
}

/// What the caller needs to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    Member,
    Admin,
}

fn authorize<E: lifeguard::LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: Uuid,
    caller: Uuid,
    access: Access,
) -> Result<(), DomainApiError> {
    let role = org_lifecycle::member_role(exec, tenant_id, org_id, caller)?
        .ok_or(DomainApiError::NotFound)?;
    if access == Access::Admin && role != "owner" && role != "admin" {
        return Err(DomainApiError::Forbidden);
    }
    Ok(())
}

fn subject(stored: &StoredClaim) -> Result<ClaimSubject, DomainApiError> {
    ClaimSubject::organisation(
        &stored.tenant_id,
        stored.org_id,
        stored.id,
        stored.claim.scope,
    )
    .map_err(|e| DomainApiError::Internal(e.to_string()))
}

fn ts(t: Option<DateTime<Utc>>) -> Value {
    t.map_or(Value::Null, |t| json!(t.to_rfc3339()))
}

/// A claim as the API shows it. `record` is the TXT record to (keep) published, for a claim that is not final and
/// when the keyring answers; the token never appears anywhere else.
#[must_use]
pub fn claim_json(stored: &StoredClaim, keyring: Option<&dyn EnrollKeyring>) -> Value {
    let c = &stored.claim;
    let record = if c.state.is_final() {
        None
    } else {
        keyring.and_then(|k| {
            let subject = subject(stored).ok()?;
            match expected_record(k, &subject, &c.domain) {
                Ok(r) => Some(json!({ "name": r.name, "type": "TXT", "value": r.value })),
                Err(error) => {
                    tracing::warn!(%error, "record not shown: keyring unavailable");
                    None
                }
            }
        })
    };
    json!({
        "id": stored.id.to_string(),
        "org_id": stored.org_id.to_string(),
        "domain": c.domain.as_str(),
        "scope": c.scope.as_str(),
        "state": c.state.as_str(),
        "dnssec": c.dnssec,
        "auto_join": c.auto_join,
        "pending_until": ts(c.pending_until),
        "verified_at": ts(c.verified_at),
        "last_checked_at": ts(c.last_checked_at),
        "last_verified_at": ts(c.last_verified_at),
        "grace_until": ts(c.grace_until),
        "created_at": stored.created_at.to_rfc3339(),
        "record": record,
    })
}

/// Run `op` in one transaction under the tenant's RLS context; an error from `op` is returned as is (writes made
/// before it are committed only if `op` returned `Ok`).
fn in_tenant<T>(
    tenant_id: &str,
    op: impl FnOnce(
        &lifeguard::pool::pooled::ExclusivePrimaryLifeExecutor<'_>,
    ) -> Result<T, DomainApiError>,
) -> Result<T, DomainApiError> {
    let mut failure = None;
    let result = sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| match op(exec) {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            failure = Some(error);
            // Abort the transaction: nothing the operation wrote survives its failure.
            Err(LifeError::Other("operation failed".to_string()))
        }
    });
    match (result, failure) {
        (_, Some(error)) => Err(error),
        (Ok(Some(value)), None) => Ok(value),
        (Ok(None), None) => Err(DomainApiError::Internal("no result".to_string())),
        (Err(error), None) => Err(error.into()),
    }
}

/// The claim `claim_id` of organisation `org_id` (another organisation's claim is not found).
fn org_claim<E: lifeguard::LifeExecutor>(
    exec: &E,
    org_id: Uuid,
    claim_id: Uuid,
) -> Result<StoredClaim, DomainApiError> {
    let stored = load_claim(exec, claim_id)?;
    if stored.org_id != org_id {
        return Err(DomainApiError::NotFound);
    }
    Ok(stored)
}

/// `list_org_domains`: the organisation's claims, newest first. Members only.
///
/// # Errors
///
/// [`DomainApiError::NotFound`] for a non-member or an organisation outside the tenant; internal errors.
pub fn list(
    ctx: &DomainContext,
    tenant_id: &str,
    org_id: Uuid,
    caller: Uuid,
) -> Result<Value, DomainApiError> {
    let claims = in_tenant(tenant_id, |exec| {
        authorize(exec, tenant_id, org_id, caller, Access::Member)?;
        Ok(list_claims(exec, org_id)?)
    })?;
    let keyring = ctx.keyring.as_deref();
    Ok(json!({ "items": claims.iter().map(|c| claim_json(c, keyring)).collect::<Vec<_>>() }))
}

/// `create_org_domain`: a PENDING claim and its record. Admins only.
///
/// # Errors
///
/// [`DomainApiError::InvalidDomain`], [`DomainApiError::InvalidRequest`] (scope),
/// [`DomainApiError::DomainUnavailable`] (ineligible, or held by another organisation of the tenant),
/// [`DomainApiError::ClaimExists`], [`DomainApiError::KeyringUnavailable`] (nothing stored), authorisation errors.
pub fn create(
    ctx: &DomainContext,
    tenant_id: &str,
    org_id: Uuid,
    caller: Uuid,
    domain: &str,
    scope: Option<&str>,
    now: DateTime<Utc>,
) -> Result<Value, DomainApiError> {
    let scope = match scope.map(str::trim).filter(|s| !s.is_empty()) {
        None => Scope::Exact,
        Some(s) => Scope::parse(s).ok_or(DomainApiError::InvalidRequest("unknown scope"))?,
    };
    let domain = normalize_domain(domain).map_err(|_| DomainApiError::InvalidDomain)?;

    let stored = in_tenant(tenant_id, |exec| {
        // Authorisation before eligibility: an outsider learns nothing about which domains are claimable here.
        authorize(exec, tenant_id, org_id, caller, Access::Admin)?;
        let keyring = ctx.keyring()?;
        match check_eligible(&domain, &ctx.policy) {
            Ok(()) => {}
            Err(DomainError::InvalidDomain(_)) => return Err(DomainApiError::InvalidDomain),
            Err(_) => return Err(DomainApiError::DomainUnavailable),
        }
        let open = open_claims_on(exec, &domain)?;
        if let Some(own) = open.iter().find(|c| c.org_id == org_id) {
            return Err(DomainApiError::ClaimExists(own.id));
        }
        if open.iter().any(|c| c.claim.state.holds()) {
            return Err(DomainApiError::DomainUnavailable);
        }
        // The record is derived before anything is stored: a key service failure leaves no claim behind.
        let id = Uuid::new_v4();
        let subject = ClaimSubject::organisation(tenant_id, org_id, id, scope)
            .map_err(|e| DomainApiError::Internal(e.to_string()))?;
        expected_record(keyring, &subject, &domain).map_err(|error| {
            tracing::error!(%error, "create_org_domain: keyring unavailable");
            DomainApiError::KeyringUnavailable
        })?;
        Ok(create_claim_with_id(
            exec,
            id,
            tenant_id,
            org_id,
            domain.clone(),
            scope,
            Some(caller),
            now,
        )?)
    })?;
    Ok(claim_json(&stored, ctx.keyring.as_deref()))
}

fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Match { .. } => "match",
        Verdict::NoMatch => "no_match",
        Verdict::Disagree => "disagree",
        Verdict::Unavailable => "unavailable",
        Verdict::DnssecFailure => "dnssec_failure",
    }
}

/// `verify_org_domain`: check the record now and apply the verdict. Admins only; one check per claim per
/// [`VERIFY_INTERVAL_SECS`].
///
/// # Errors
///
/// [`DomainApiError::RateLimited`], [`DomainApiError::ClaimFinal`], [`DomainApiError::KeyringUnavailable`] and
/// [`DomainApiError::DnsUnavailable`] (claim unchanged), [`DomainApiError::DomainUnavailable`] (another organisation
/// of the tenant verified first: the claim is now CONFLICT), authorisation errors.
pub fn verify(
    ctx: &DomainContext,
    tenant_id: &str,
    org_id: Uuid,
    claim_id: Uuid,
    caller: Uuid,
    now: DateTime<Utc>,
) -> Result<Value, DomainApiError> {
    let stored = in_tenant(tenant_id, |exec| {
        authorize(exec, tenant_id, org_id, caller, Access::Admin)?;
        org_claim(exec, org_id, claim_id)
    })?;
    let keyring = ctx.keyring()?;
    if stored.claim.state.is_final() {
        return Err(DomainApiError::ClaimFinal);
    }
    if let Some(last) = stored.claim.last_checked_at {
        let elapsed = (now - last).num_seconds();
        if elapsed < VERIFY_INTERVAL_SECS {
            return Err(DomainApiError::RateLimited {
                retry_after: VERIFY_INTERVAL_SECS - elapsed.max(0),
            });
        }
    }

    let answers = ctx
        .probe
        .lookup(&challenge_name(&stored.claim.domain))
        .map_err(|error| {
            tracing::warn!(%error, "verify_org_domain: dns-probe unavailable");
            DomainApiError::DnsUnavailable
        })?;
    let verdict =
        quorum(keyring, &subject(&stored)?, &stored.claim.domain, &answers).map_err(|error| {
            tracing::error!(%error, "verify_org_domain: keyring unavailable");
            DomainApiError::KeyringUnavailable
        })?;

    let after = record_verdict(tenant_id, claim_id, verdict, now)?;
    if after.claim.state == ClaimState::Conflict {
        return Err(DomainApiError::DomainUnavailable);
    }
    Ok(json!({ "claim": claim_json(&after, Some(keyring)), "verdict": verdict_name(verdict) }))
}

/// `update_org_domain`: auto-join on or off, VERIFIED claims only. Admins only.
///
/// # Errors
///
/// [`DomainApiError::NotVerified`], [`DomainApiError::ClaimFinal`], authorisation errors.
pub fn set_auto_join(
    ctx: &DomainContext,
    tenant_id: &str,
    org_id: Uuid,
    claim_id: Uuid,
    caller: Uuid,
    auto_join: bool,
    now: DateTime<Utc>,
) -> Result<Value, DomainApiError> {
    let stored = in_tenant(tenant_id, |exec| {
        authorize(exec, tenant_id, org_id, caller, Access::Admin)?;
        org_claim(exec, org_id, claim_id)?;
        match apply_event(exec, claim_id, &LifecycleEvent::SetAutoJoin(auto_join), now) {
            Err(ClaimError::Lifecycle(LifecycleError::Final(_))) => {
                Err(DomainApiError::NotVerified)
            }
            other => Ok(other?),
        }
    })?;
    Ok(claim_json(&stored, ctx.keyring.as_deref()))
}

/// `delete_org_domain`: the claim is withdrawn (REVOKED) at once. Admins only. A claim already final stays as it is.
///
/// # Errors
///
/// Authorisation errors and [`DomainApiError::NotFound`].
pub fn delete(
    tenant_id: &str,
    org_id: Uuid,
    claim_id: Uuid,
    caller: Uuid,
    now: DateTime<Utc>,
) -> Result<(), DomainApiError> {
    in_tenant(tenant_id, |exec| {
        authorize(exec, tenant_id, org_id, caller, Access::Admin)?;
        if org_claim(exec, org_id, claim_id)?.claim.state.is_final() {
            return Ok(());
        }
        apply_event(
            exec,
            claim_id,
            &LifecycleEvent::Withdrawn {
                by: Withdrawer::OrgAdmin,
                reason: "org_admin",
            },
            now,
        )?;
        Ok(())
    })
}

/// Reasons a tenant admin may give for a revocation (fixed codes, never free text).
pub const REVOKE_REASONS: &[&str] = &["policy", "dispute", "security", "other"];

/// `revoke_org_domain`: a tenant admin revokes any organisation's claim in the tenant. The caller's tenant-admin
/// role is checked by the controller (from the token).
///
/// # Errors
///
/// [`DomainApiError::InvalidRequest`] (unknown reason), [`DomainApiError::NotFound`] (no such claim in the tenant),
/// [`DomainApiError::ClaimFinal`].
pub fn revoke(
    ctx: &DomainContext,
    tenant_id: &str,
    claim_id: Uuid,
    reason: &str,
    now: DateTime<Utc>,
) -> Result<Value, DomainApiError> {
    let reason = REVOKE_REASONS
        .iter()
        .copied()
        .find(|r| *r == reason)
        .ok_or(DomainApiError::InvalidRequest("unknown reason"))?;
    let stored = in_tenant(tenant_id, |exec| {
        Ok(apply_event(
            exec,
            claim_id,
            &LifecycleEvent::Withdrawn {
                by: Withdrawer::TenantAdmin,
                reason,
            },
            now,
        )?)
    })?;
    Ok(claim_json(&stored, ctx.keyring.as_deref()))
}

/// Tenant-admin check for the tenant console (`revoke_org_domain`): the token's tenant, and the `tenant_admin` or
/// `owner` role in it (ADR-011 §2.3). Same rules as identity-login-service's tenant admin principal.
///
/// # Errors
///
/// `Err(401)` without a usable subject or tenant, `Err(403)` without the role.
pub fn tenant_admin_tenant(claims: Option<&Value>) -> Result<String, u16> {
    let claims = claims.ok_or(401u16)?;
    claims
        .get("sub")
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or(401u16)?;
    let nonempty = |v: Option<&Value>| {
        v.and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let sx = claims.get("sx");
    let tenant = match (
        nonempty(sx.and_then(|s| s.get("tenant"))),
        nonempty(claims.get("tenant_id")),
    ) {
        (Some(a), Some(b)) if a != b => return Err(401),
        (Some(a), _) | (None, Some(a)) => a,
        (None, None) => return Err(401),
    };
    let is_admin = [sx.and_then(|s| s.get("roles")), claims.get("roles")]
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(Value::as_str)
        .map(|r| r.trim().to_ascii_lowercase())
        .any(|r| r == "tenant_admin" || r == "owner");
    if is_admin {
        Ok(tenant)
    } else {
        Err(403)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn statuses_and_codes() {
        assert_eq!(DomainApiError::DomainUnavailable.status(), 422);
        assert_eq!(DomainApiError::NotVerified.code(), "not_verified");
        assert_eq!(
            DomainApiError::RateLimited { retry_after: 12 }.body()["retry_after"],
            12
        );
        assert_eq!(DomainApiError::KeyringUnavailable.status(), 503);
        // never echoes internals
        let body = DomainApiError::Internal("secret detail".into()).body();
        assert!(!body.to_string().contains("secret"));
    }

    #[test]
    fn ineligible_and_held_elsewhere_are_one_answer() {
        assert_eq!(
            DomainApiError::DomainUnavailable.body(),
            DomainApiError::from(ClaimError::DomainUnavailable).body()
        );
    }

    #[test]
    fn tenant_admin_from_token_only() {
        let sub = "11111111-1111-4111-8111-111111111111";
        let ok = json!({"sub": sub, "sx": {"tenant": "acme", "roles": ["Tenant_Admin"]}});
        assert_eq!(tenant_admin_tenant(Some(&ok)), Ok("acme".to_string()));
        let owner = json!({"sub": sub, "tenant_id": "acme", "roles": ["owner"]});
        assert_eq!(tenant_admin_tenant(Some(&owner)), Ok("acme".to_string()));
        let member = json!({"sub": sub, "sx": {"tenant": "acme", "roles": ["member"]}});
        assert_eq!(tenant_admin_tenant(Some(&member)), Err(403));
        let split =
            json!({"sub": sub, "tenant_id": "b", "sx": {"tenant": "a", "roles": ["tenant_admin"]}});
        assert_eq!(tenant_admin_tenant(Some(&split)), Err(401));
        assert_eq!(tenant_admin_tenant(None), Err(401));
    }

    #[test]
    fn revoke_reasons_are_fixed() {
        let ctx = DomainContext {
            keyring: None,
            probe: Box::new(HttpDnsProbe::new("http://127.0.0.1:9")),
            policy: EligibilityPolicy::default(),
        };
        assert_eq!(
            revoke(
                &ctx,
                "acme",
                Uuid::new_v4(),
                "because I said so",
                Utc::now()
            )
            .unwrap_err(),
            DomainApiError::InvalidRequest("unknown reason")
        );
    }
}
