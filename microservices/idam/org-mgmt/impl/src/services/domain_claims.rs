//! Organisation domain claims in the database (story 17.3; DESIGN-org-domain-verification.md §8).
//!
//! The state machine is `sesame_common::domain_verify::lifecycle` (pure); this module loads a claim, applies one event
//! and writes the claim and its history row in the caller's transaction (tenant RLS on both tables). The partial
//! unique index `org_domains_one_holder` is the final arbiter of a verification race: [`record_verdict`] turns a match
//! that loses it into CONFLICT.

use chrono::{DateTime, Utc};
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use sesame_common::domain_verify::lifecycle::{
    apply, Claim, ClaimEvent, ClaimState, LifecycleError, LifecycleEvent,
};
use sesame_common::domain_verify::{normalize_domain, Domain, Scope, Verdict};
use uuid::Uuid;

use crate::models::org_domain::{
    Column as DomainColumn, Entity as DomainEntity, OrgDomainModel, OrgDomainRecord,
};
use crate::models::org_domain_event::OrgDomainEventRecord;

/// Why a claim operation failed. Messages never carry a token.
#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    /// Database failure (the message is the driver's, for logs only).
    #[error("database: {0}")]
    Db(String),
    /// No such claim in this tenant.
    #[error("claim not found")]
    NotFound,
    /// The stored row is not a valid claim (unknown state or scope, unparsable domain).
    #[error("stored claim is malformed: {0}")]
    Malformed(&'static str),
    /// The lifecycle refused the event.
    #[error(transparent)]
    Lifecycle(#[from] LifecycleError),
    /// Another organisation of the tenant holds the domain (the one-holder index).
    #[error("domain held by another organisation")]
    DomainUnavailable,
}

impl ClaimError {
    fn from_db(text: String) -> Self {
        if text.contains("org_domains_one_holder") {
            Self::DomainUnavailable
        } else {
            Self::Db(text)
        }
    }
}

impl From<LifeError> for ClaimError {
    fn from(e: LifeError) -> Self {
        Self::from_db(e.to_string())
    }
}

impl From<lifeguard::active_model::ActiveModelError> for ClaimError {
    fn from(e: lifeguard::active_model::ActiveModelError) -> Self {
        Self::from_db(e.to_string())
    }
}

/// A stored claim: its identity and its lifecycle fields.
#[derive(Debug, Clone)]
pub struct StoredClaim {
    /// Claim (row) id.
    pub id: Uuid,
    /// Owning organisation.
    pub org_id: Uuid,
    /// Tenant (RLS).
    pub tenant_id: String,
    /// Lifecycle fields.
    pub claim: Claim,
    /// Who created it.
    pub created_by: Option<Uuid>,
    /// When.
    pub created_at: DateTime<Utc>,
}

fn epoch_u32(epoch: i32) -> Result<u32, ClaimError> {
    u32::try_from(epoch).map_err(|_| ClaimError::Malformed("epoch"))
}

impl TryFrom<OrgDomainModel> for StoredClaim {
    type Error = ClaimError;

    fn try_from(m: OrgDomainModel) -> Result<Self, Self::Error> {
        let claim = Claim {
            domain: normalize_domain(&m.domain).map_err(|_| ClaimError::Malformed("domain"))?,
            scope: Scope::parse(&m.scope).ok_or(ClaimError::Malformed("scope"))?,
            state: ClaimState::parse(&m.state).ok_or(ClaimError::Malformed("state"))?,
            epoch: epoch_u32(m.epoch)?,
            dnssec: m.dnssec,
            pending_until: m.pending_until,
            verified_at: m.verified_at,
            last_checked_at: m.last_checked_at,
            last_verified_at: m.last_verified_at,
            grace_until: m.grace_until,
            auto_join: m.auto_join,
        };
        Ok(Self {
            id: m.id,
            org_id: m.org_id,
            tenant_id: m.tenant_id,
            claim,
            created_by: m.created_by,
            created_at: m.created_at,
        })
    }
}

fn record_for(stored: &StoredClaim, now: DateTime<Utc>) -> Result<OrgDomainRecord, ClaimError> {
    let c = &stored.claim;
    let mut rec = OrgDomainRecord::new();
    rec.set_id(stored.id)
        .set_org_id(stored.org_id)
        .set_domain(c.domain.as_str().to_string())
        .set_tenant_id(stored.tenant_id.clone())
        .set_scope(c.scope.as_str().to_string())
        .set_state(c.state.as_str().to_string())
        .set_epoch(i32::try_from(c.epoch).map_err(|_| ClaimError::Malformed("epoch"))?)
        .set_dnssec(c.dnssec)
        .set_pending_until(c.pending_until)
        .set_verified_at(c.verified_at)
        .set_last_checked_at(c.last_checked_at)
        .set_last_verified_at(c.last_verified_at)
        .set_grace_until(c.grace_until)
        .set_auto_join(c.auto_join)
        .set_created_by(stored.created_by)
        .set_created_at(stored.created_at)
        .set_updated_at(now);
    Ok(rec)
}

fn append_event<E: LifeExecutor>(
    exec: &E,
    stored: &StoredClaim,
    event: &ClaimEvent,
    now: DateTime<Utc>,
) -> Result<(), ClaimError> {
    let mut rec = OrgDomainEventRecord::new();
    rec.set_id(Uuid::new_v4())
        .set_tenant_id(stored.tenant_id.clone())
        .set_org_domain_id(stored.id)
        .set_at(now)
        .set_event(event.event.to_string())
        .set_detail(event.detail.clone());
    rec.insert(exec)?;
    Ok(())
}

/// Create a PENDING claim and its `created` event. Eligibility and the caller's right to claim are checked before.
///
/// # Errors
///
/// [`ClaimError::Db`] on a database failure.
pub fn create_claim<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: Uuid,
    domain: Domain,
    scope: Scope,
    created_by: Option<Uuid>,
    now: DateTime<Utc>,
) -> Result<StoredClaim, ClaimError> {
    create_claim_with_id(
        exec,
        Uuid::new_v4(),
        tenant_id,
        org_id,
        domain,
        scope,
        created_by,
        now,
    )
}

/// [`create_claim`] with the claim id chosen by the caller: the id is part of the record's MAC input, so the API
/// derives the record before it stores anything.
///
/// # Errors
///
/// [`ClaimError::Db`] on a database failure.
#[allow(clippy::too_many_arguments)]
pub fn create_claim_with_id<E: LifeExecutor>(
    exec: &E,
    id: Uuid,
    tenant_id: &str,
    org_id: Uuid,
    domain: Domain,
    scope: Scope,
    created_by: Option<Uuid>,
    now: DateTime<Utc>,
) -> Result<StoredClaim, ClaimError> {
    let (claim, event) = Claim::new(domain, scope, now);
    let stored = StoredClaim {
        id,
        org_id,
        tenant_id: tenant_id.to_string(),
        claim,
        created_by,
        created_at: now,
    };
    record_for(&stored, now)?.insert(exec)?;
    append_event(exec, &stored, &event, now)?;
    Ok(stored)
}

/// Load one claim of the tenant (RLS hides other tenants' claims).
///
/// # Errors
///
/// [`ClaimError::NotFound`], or a database or malformed-row error.
pub fn load_claim<E: LifeExecutor>(exec: &E, claim_id: Uuid) -> Result<StoredClaim, ClaimError> {
    DomainEntity::find()
        .filter(DomainColumn::Id.eq(claim_id))
        .find_one(exec)?
        .ok_or(ClaimError::NotFound)?
        .try_into()
}

/// The organisation's claims, newest first.
///
/// # Errors
///
/// A database or malformed-row error.
pub fn list_claims<E: LifeExecutor>(
    exec: &E,
    org_id: Uuid,
) -> Result<Vec<StoredClaim>, ClaimError> {
    let mut rows: Vec<StoredClaim> = DomainEntity::find()
        .filter(DomainColumn::OrgId.eq(org_id))
        .all(exec)?
        .into_iter()
        .map(StoredClaim::try_from)
        .collect::<Result<_, _>>()?;
    rows.sort_by_key(|c| std::cmp::Reverse(c.created_at));
    Ok(rows)
}

/// Apply one lifecycle event to a claim and store the result with its history row.
///
/// # Errors
///
/// [`ClaimError::Lifecycle`] when the event does not apply; [`ClaimError::DomainUnavailable`] when the claim would
/// become a second holder (the transaction is then aborted: see [`record_verdict`]); database errors.
pub fn apply_event<E: LifeExecutor>(
    exec: &E,
    claim_id: Uuid,
    event: &LifecycleEvent,
    now: DateTime<Utc>,
) -> Result<StoredClaim, ClaimError> {
    let mut stored = load_claim(exec, claim_id)?;
    let (next, history) = apply(&stored.claim, event, now)?;
    stored.claim = next;
    record_for(&stored, now)?.update(exec)?;
    append_event(exec, &stored, &history, now)?;
    Ok(stored)
}

/// Record a DNS verdict for a claim in its own tenant transaction. A match that loses the race for the one-holder
/// index makes the claim CONFLICT (in a second transaction, the first having been aborted by the violation).
///
/// # Errors
///
/// As [`apply_event`], except that a lost race is not an error but a CONFLICT claim.
pub fn record_verdict(
    tenant_id: &str,
    claim_id: Uuid,
    verdict: Verdict,
    now: DateTime<Utc>,
) -> Result<StoredClaim, ClaimError> {
    let first = sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| {
        Ok(apply_event(
            exec,
            claim_id,
            &LifecycleEvent::Checked(verdict),
            now,
        ))
    })
    .map_err(ClaimError::from)
    .and_then(|r| r);
    match first {
        Err(ClaimError::DomainUnavailable) => {
            sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| {
                Ok(apply_event(exec, claim_id, &LifecycleEvent::Conflict, now))
            })
            .map_err(ClaimError::from)
            .and_then(|r| r)
        }
        other => other,
    }
}

/// The tenant's claims on exactly this domain that are not final (PENDING, VERIFIED, GRACE), any organisation. Runs in
/// the caller's tenant transaction.
///
/// # Errors
///
/// A database or malformed-row error.
pub fn open_claims_on<E: LifeExecutor>(
    exec: &E,
    domain: &Domain,
) -> Result<Vec<StoredClaim>, ClaimError> {
    DomainEntity::find()
        .filter(DomainColumn::Domain.eq(domain.as_str().to_string()))
        .filter(DomainColumn::State.is_in(vec![
            ClaimState::Pending.as_str().to_string(),
            ClaimState::Verified.as_str().to_string(),
            ClaimState::Grace.as_str().to_string(),
        ]))
        .all(exec)?
        .into_iter()
        .map(StoredClaim::try_from)
        .collect()
}

/// The candidate claim names for an address domain: the domain itself and every parent (`eu.bigbank.com` →
/// `eu.bigbank.com`, `bigbank.com`, `com`); scope decides which of them actually cover it.
fn candidate_names(email_domain: &Domain) -> Vec<String> {
    let labels: Vec<&str> = email_domain.labels().collect();
    (0..labels.len()).map(|i| labels[i..].join(".")).collect()
}

fn holding_claims<E: LifeExecutor>(
    exec: &E,
    email_domain: &Domain,
) -> Result<Vec<StoredClaim>, ClaimError> {
    DomainEntity::find()
        .filter(DomainColumn::Domain.is_in(candidate_names(email_domain)))
        .filter(DomainColumn::State.is_in(vec![
            ClaimState::Verified.as_str().to_string(),
            ClaimState::Grace.as_str().to_string(),
        ]))
        .all(exec)?
        .into_iter()
        .map(StoredClaim::try_from)
        .collect()
}

/// The organisation's claim that lets it vouch for an address at `email_domain` (first-time SAML linking, just-in-time
/// provisioning, auto-join): VERIFIED only, scope honoured. Runs in the caller's tenant transaction.
///
/// # Errors
///
/// A database or malformed-row error.
pub fn verified_domain_for<E: LifeExecutor>(
    exec: &E,
    org_id: Uuid,
    email_domain: &Domain,
) -> Result<Option<StoredClaim>, ClaimError> {
    Ok(holding_claims(exec, email_domain)?
        .into_iter()
        .find(|c| c.org_id == org_id && c.claim.vouches_for(email_domain)))
}

/// The organisation an address at `email_domain` is routed to by "Sign in with SSO" discovery: a VERIFIED or GRACE
/// claim, scope honoured, the most specific name first. Runs in the caller's tenant transaction.
///
/// # Errors
///
/// A database or malformed-row error.
pub fn discoverable_domain<E: LifeExecutor>(
    exec: &E,
    email_domain: &Domain,
) -> Result<Option<Uuid>, ClaimError> {
    let mut claims: Vec<StoredClaim> = holding_claims(exec, email_domain)?
        .into_iter()
        .filter(|c| c.claim.discoverable_for(email_domain))
        .collect();
    claims.sort_by_key(|c| std::cmp::Reverse(c.claim.domain.as_str().len()));
    Ok(claims.first().map(|c| c.org_id))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn candidates_are_the_domain_and_its_parents() {
        let d = normalize_domain("eu.bigbank.com").unwrap();
        assert_eq!(
            candidate_names(&d),
            vec!["eu.bigbank.com", "bigbank.com", "com"]
        );
    }
}
