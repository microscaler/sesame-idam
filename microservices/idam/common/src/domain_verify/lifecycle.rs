//! Organisation domain claim lifecycle (story 17.3; `DESIGN-org-domain-verification.md` §5).
//!
//! One pure function, [`apply`], decides every state change of a claim, so the API (org-mgmt) and the daily
//! re-verification job cannot diverge. It returns the claim as it must be stored and exactly one [`ClaimEvent`] for the
//! append-only history; the caller writes both in one transaction.
//!
//! ```text
//! (created) ─► PENDING ──quorum match──► VERIFIED ◄──record back within 72 h── GRACE
//!                │  │                       │ re-check fails ───────────────────►│
//!                │  └─ another org first ─► CONFLICT                              │ grace exhausted
//!                └─ not verified in 7 days ─► EXPIRED          VERIFIED, GRACE ──► REVOKED (also: admin)
//! ```
//!
//! REVOKED, EXPIRED and CONFLICT are final for a claim: claiming the domain again creates a new claim (a new row and so a
//! new token), which starts at PENDING. Only a [`Verdict::Match`] reaches VERIFIED. "Could not check"
//! ([`Verdict::Unavailable`]) never changes a state: a DNS or KMS outage must not take a verified domain away, nor count
//! towards revoking it; grace runs on time, not on failures.

use chrono::{DateTime, Duration, Utc};
use serde_json::json;

use super::quorum::Verdict;
use super::token::Scope;
use super::Domain;

/// How long a new claim may stay unverified.
pub const PENDING_FOR: Duration = Duration::days(7);
/// How long a verified claim keeps working after its record stops checking out.
pub const GRACE_FOR: Duration = Duration::hours(72);

/// A claim's state (stored upper-case).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimState {
    /// Created, not yet proven.
    Pending,
    /// Proven by a quorum match, and still checking out.
    Verified,
    /// Was verified; the last re-check failed. Keeps existing behaviour working, binds nothing new.
    Grace,
    /// Withdrawn (grace exhausted, or by an admin). Final.
    Revoked,
    /// Not proven in time. Final.
    Expired,
    /// Another organisation of the tenant verified the domain first. Final.
    Conflict,
}

impl ClaimState {
    /// Stored form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Verified => "VERIFIED",
            Self::Grace => "GRACE",
            Self::Revoked => "REVOKED",
            Self::Expired => "EXPIRED",
            Self::Conflict => "CONFLICT",
        }
    }

    /// Parse the stored form.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "PENDING" => Self::Pending,
            "VERIFIED" => Self::Verified,
            "GRACE" => Self::Grace,
            "REVOKED" => Self::Revoked,
            "EXPIRED" => Self::Expired,
            "CONFLICT" => Self::Conflict,
            _ => return None,
        })
    }

    /// No event changes a claim in this state any more.
    #[must_use]
    pub const fn is_final(self) -> bool {
        matches!(self, Self::Revoked | Self::Expired | Self::Conflict)
    }

    /// Holds the domain for its tenant (the one-holder rule): VERIFIED or GRACE.
    #[must_use]
    pub const fn holds(self) -> bool {
        matches!(self, Self::Verified | Self::Grace)
    }
}

/// The lifecycle fields of a claim (the stored row carries identity and timestamps besides).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// The claimed domain (normalised).
    pub domain: Domain,
    /// Exact domain, or the domain and its subdomains.
    pub scope: Scope,
    /// Current state.
    pub state: ClaimState,
    /// Key epoch of the last match.
    pub epoch: u32,
    /// Whether the last match was DNSSEC-validated by every provider (false until a validated match).
    pub dnssec: bool,
    /// PENDING expires at this instant.
    pub pending_until: Option<DateTime<Utc>>,
    /// First verified.
    pub verified_at: Option<DateTime<Utc>>,
    /// Last checked (any outcome).
    pub last_checked_at: Option<DateTime<Utc>>,
    /// Last check that matched.
    pub last_verified_at: Option<DateTime<Utc>>,
    /// GRACE ends at this instant.
    pub grace_until: Option<DateTime<Utc>>,
    /// Members with an address in the domain may join the organisation on their own (VERIFIED only).
    pub auto_join: bool,
}

impl Claim {
    /// A new claim, PENDING for [`PENDING_FOR`]. Its creation is the event [`ClaimEvent::created`].
    #[must_use]
    pub fn new(domain: Domain, scope: Scope, now: DateTime<Utc>) -> (Self, ClaimEvent) {
        let claim = Self {
            domain,
            scope,
            state: ClaimState::Pending,
            epoch: 1,
            dnssec: false,
            pending_until: Some(now + PENDING_FOR),
            verified_at: None,
            last_checked_at: None,
            last_verified_at: None,
            grace_until: None,
            auto_join: false,
        };
        let event = ClaimEvent::new("created", json!({"scope": scope.as_str()}));
        (claim, event)
    }

    /// Whether this claim, in its current state, lets the organisation vouch for `email_domain` (first-time SAML
    /// linking, just-in-time provisioning, auto-join): VERIFIED only, honouring the scope.
    #[must_use]
    pub fn vouches_for(&self, email_domain: &Domain) -> bool {
        self.state == ClaimState::Verified && self.scope.covers(&self.domain, email_domain)
    }

    /// Whether this claim routes `email_domain` to its organisation in "Sign in with SSO" discovery: VERIFIED or
    /// GRACE, honouring the scope.
    #[must_use]
    pub fn discoverable_for(&self, email_domain: &Domain) -> bool {
        self.state.holds() && self.scope.covers(&self.domain, email_domain)
    }
}

/// Who withdrew a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Withdrawer {
    /// An admin of the organisation deleted it.
    OrgAdmin,
    /// A tenant admin revoked it.
    TenantAdmin,
}

/// Something that happens to a claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// A DNS check finished with this verdict.
    Checked(Verdict),
    /// Time passed without a check (the job's sweep): PENDING may expire, GRACE may run out.
    Tick,
    /// Another organisation of the tenant holds the domain (it verified first, or won the race to the unique index).
    Conflict,
    /// An admin withdrew the claim.
    Withdrawn {
        /// Who.
        by: Withdrawer,
        /// Fixed reason code (never free text from a request).
        reason: &'static str,
    },
    /// The organisation turns domain-based auto-join on or off.
    SetAutoJoin(bool),
}

/// One row of the append-only history: `event` is one of `checked verified grace revoked expired conflict deleted
/// auto_join`, `detail` never carries the token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimEvent {
    /// Event name.
    pub event: &'static str,
    /// Details (verdict, resolver facts, reason).
    pub detail: serde_json::Value,
}

impl ClaimEvent {
    fn new(event: &'static str, detail: serde_json::Value) -> Self {
        Self { event, detail }
    }
}

/// Why an event cannot apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LifecycleError {
    /// The claim is REVOKED, EXPIRED or CONFLICT: claim the domain again instead.
    #[error("claim is final ({0:?})")]
    Final(ClaimState),
    /// The event does not apply in this state (e.g. auto-join on an unverified claim).
    #[error("event does not apply in state {0:?}")]
    NotApplicable(ClaimState),
}

impl LifecycleError {
    /// Stable machine-readable code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Final(_) => "claim_final",
            Self::NotApplicable(_) => "not_applicable",
        }
    }
}

fn verdict_detail(verdict: Verdict) -> serde_json::Value {
    serde_json::to_value(verdict).unwrap_or_else(|_| json!({"verdict": "unknown"}))
}

/// Apply `event` to `claim` at `now`: the claim as it must be stored and the one event to append.
///
/// # Errors
///
/// [`LifecycleError::Final`] for a REVOKED, EXPIRED or CONFLICT claim; [`LifecycleError::NotApplicable`] for an event
/// that has no meaning in the claim's state.
pub fn apply(
    claim: &Claim,
    event: &LifecycleEvent,
    now: DateTime<Utc>,
) -> Result<(Claim, ClaimEvent), LifecycleError> {
    use ClaimState::{Conflict, Expired, Grace, Pending, Revoked, Verified};

    if claim.state.is_final() {
        return Err(LifecycleError::Final(claim.state));
    }
    let mut next = claim.clone();

    let out = match (claim.state, event) {
        (_, LifecycleEvent::Withdrawn { by, reason }) => {
            next.state = Revoked;
            next.grace_until = None;
            next.auto_join = false;
            let name = match by {
                Withdrawer::OrgAdmin => "deleted",
                Withdrawer::TenantAdmin => "revoked",
            };
            ClaimEvent::new(
                name,
                json!({"reason": reason, "from": claim.state.as_str()}),
            )
        }

        (Pending, LifecycleEvent::Conflict) => {
            next.state = Conflict;
            ClaimEvent::new("conflict", json!({}))
        }
        (_, LifecycleEvent::Conflict) => return Err(LifecycleError::NotApplicable(claim.state)),

        (Verified, LifecycleEvent::SetAutoJoin(on)) => {
            next.auto_join = *on;
            ClaimEvent::new("auto_join", json!({"auto_join": on}))
        }
        (_, LifecycleEvent::SetAutoJoin(_)) => {
            return Err(LifecycleError::NotApplicable(claim.state))
        }

        // ---- time
        (Pending, LifecycleEvent::Tick) if expired(claim.pending_until, now) => expire(&mut next),
        (Grace, LifecycleEvent::Tick) if expired(claim.grace_until, now) => {
            grace_exhausted(&mut next)
        }
        (_, LifecycleEvent::Tick) => ClaimEvent::new("checked", json!({"verdict": "none"})),

        // ---- checks
        (Pending, LifecycleEvent::Checked(verdict)) => {
            next.last_checked_at = Some(now);
            match *verdict {
                Verdict::Match { epoch, dnssec } => {
                    matched(&mut next, epoch, dnssec, now);
                    next.verified_at = Some(now);
                    next.pending_until = None;
                    ClaimEvent::new("verified", verdict_detail(*verdict))
                }
                // A late check cannot rescue a claim whose time is up.
                _ if expired(claim.pending_until, now) => expire(&mut next),
                other => ClaimEvent::new("checked", verdict_detail(other)),
            }
        }
        (Verified, LifecycleEvent::Checked(verdict)) => {
            next.last_checked_at = Some(now);
            match *verdict {
                Verdict::Match { epoch, dnssec } => {
                    matched(&mut next, epoch, dnssec, now);
                    ClaimEvent::new("checked", verdict_detail(*verdict))
                }
                Verdict::Unavailable => ClaimEvent::new("checked", verdict_detail(*verdict)),
                // No record, providers disagreeing (possible attack) or a DNSSEC failure: the record does not check
                // out. Grace starts; nothing new is bound to the domain from now on.
                other => {
                    next.state = Grace;
                    next.grace_until = Some(now + GRACE_FOR);
                    ClaimEvent::new("grace", verdict_detail(other))
                }
            }
        }
        (Grace, LifecycleEvent::Checked(verdict)) => {
            next.last_checked_at = Some(now);
            match *verdict {
                Verdict::Match { epoch, dnssec } => {
                    matched(&mut next, epoch, dnssec, now);
                    next.state = Verified;
                    next.grace_until = None;
                    ClaimEvent::new("verified", verdict_detail(*verdict))
                }
                _ if expired(claim.grace_until, now) => grace_exhausted(&mut next),
                other => ClaimEvent::new("checked", verdict_detail(other)),
            }
        }

        (Revoked | Expired | Conflict, _) => unreachable!("final states return early"),
    };
    Ok((next, out))
}

fn expired(until: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    until.is_some_and(|u| now >= u)
}

fn matched(claim: &mut Claim, epoch: u32, dnssec: bool, now: DateTime<Utc>) {
    claim.state = ClaimState::Verified;
    claim.epoch = epoch;
    claim.dnssec = dnssec;
    claim.last_verified_at = Some(now);
}

fn expire(claim: &mut Claim) -> ClaimEvent {
    claim.state = ClaimState::Expired;
    claim.pending_until = None;
    ClaimEvent::new("expired", json!({}))
}

fn grace_exhausted(claim: &mut Claim) -> ClaimEvent {
    claim.state = ClaimState::Revoked;
    claim.grace_until = None;
    claim.auto_join = false;
    ClaimEvent::new(
        "revoked",
        json!({"reason": "grace_exhausted", "from": "GRACE"}),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain_verify::normalize_domain;
    use proptest::prelude::*;

    fn d(s: &str) -> Domain {
        normalize_domain(s).unwrap()
    }

    fn t0() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-11T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    const MATCH: Verdict = Verdict::Match {
        epoch: 2,
        dnssec: true,
    };

    fn verified() -> Claim {
        let (c, _) = Claim::new(d("bigbank.com"), Scope::Exact, t0());
        apply(&c, &LifecycleEvent::Checked(MATCH), t0()).unwrap().0
    }

    #[test]
    fn new_claim_is_pending_for_seven_days() {
        let (c, e) = Claim::new(d("bigbank.com"), Scope::Exact, t0());
        assert_eq!(c.state, ClaimState::Pending);
        assert_eq!(c.pending_until, Some(t0() + PENDING_FOR));
        assert_eq!(e.event, "created");
    }

    #[test]
    fn match_verifies_and_records_epoch_and_dnssec() {
        let c = verified();
        assert_eq!(c.state, ClaimState::Verified);
        assert_eq!((c.epoch, c.dnssec), (2, true));
        assert_eq!(c.verified_at, Some(t0()));
        assert_eq!(c.pending_until, None);
    }

    #[test]
    fn pending_failures_stay_pending_then_expire() {
        let (c, _) = Claim::new(d("bigbank.com"), Scope::Exact, t0());
        let (c, e) = apply(&c, &LifecycleEvent::Checked(Verdict::NoMatch), t0()).unwrap();
        assert_eq!((c.state, e.event), (ClaimState::Pending, "checked"));
        let late = t0() + PENDING_FOR;
        let (x, e) = apply(&c, &LifecycleEvent::Tick, late).unwrap();
        assert_eq!((x.state, e.event), (ClaimState::Expired, "expired"));
        // a non-matching check after the deadline expires it too
        let (x, _) = apply(&c, &LifecycleEvent::Checked(Verdict::Unavailable), late).unwrap();
        assert_eq!(x.state, ClaimState::Expired);
    }

    #[test]
    fn failed_recheck_starts_grace_and_unavailable_does_not() {
        let c = verified();
        let later = t0() + Duration::days(1);
        let (g, e) = apply(&c, &LifecycleEvent::Checked(Verdict::NoMatch), later).unwrap();
        assert_eq!((g.state, e.event), (ClaimState::Grace, "grace"));
        assert_eq!(g.grace_until, Some(later + GRACE_FOR));
        for v in [Verdict::Disagree, Verdict::DnssecFailure] {
            assert_eq!(
                apply(&c, &LifecycleEvent::Checked(v), later)
                    .unwrap()
                    .0
                    .state,
                ClaimState::Grace
            );
        }
        let (u, e) = apply(&c, &LifecycleEvent::Checked(Verdict::Unavailable), later).unwrap();
        assert_eq!((u.state, e.event), (ClaimState::Verified, "checked"));
        assert_eq!(u.last_verified_at, c.last_verified_at);
    }

    #[test]
    fn grace_recovers_on_match_and_revokes_when_exhausted() {
        let c = verified();
        let (g, _) = apply(&c, &LifecycleEvent::Checked(Verdict::NoMatch), t0()).unwrap();
        let (back, e) = apply(
            &g,
            &LifecycleEvent::Checked(MATCH),
            t0() + Duration::hours(10),
        )
        .unwrap();
        assert_eq!(
            (back.state, e.event, back.grace_until),
            (ClaimState::Verified, "verified", None)
        );
        let end = t0() + GRACE_FOR;
        let (r, e) = apply(&g, &LifecycleEvent::Tick, end).unwrap();
        assert_eq!((r.state, e.event), (ClaimState::Revoked, "revoked"));
        let (r, _) = apply(&g, &LifecycleEvent::Checked(Verdict::NoMatch), end).unwrap();
        assert_eq!(r.state, ClaimState::Revoked);
    }

    #[test]
    fn withdrawal_from_any_live_state_and_names() {
        let (p, _) = Claim::new(d("bigbank.com"), Scope::Exact, t0());
        for c in [p, verified()] {
            let (r, e) = apply(
                &c,
                &LifecycleEvent::Withdrawn {
                    by: Withdrawer::OrgAdmin,
                    reason: "deleted",
                },
                t0(),
            )
            .unwrap();
            assert_eq!((r.state, e.event), (ClaimState::Revoked, "deleted"));
            let (_, e) = apply(
                &c,
                &LifecycleEvent::Withdrawn {
                    by: Withdrawer::TenantAdmin,
                    reason: "policy",
                },
                t0(),
            )
            .unwrap();
            assert_eq!(e.event, "revoked");
        }
    }

    #[test]
    fn final_states_refuse_every_event() {
        let (r, _) = apply(
            &verified(),
            &LifecycleEvent::Withdrawn {
                by: Withdrawer::TenantAdmin,
                reason: "x",
            },
            t0(),
        )
        .unwrap();
        assert_eq!(
            apply(&r, &LifecycleEvent::Checked(MATCH), t0()),
            Err(LifecycleError::Final(ClaimState::Revoked))
        );
    }

    #[test]
    fn conflict_only_from_pending_and_auto_join_only_when_verified() {
        let (p, _) = Claim::new(d("bigbank.com"), Scope::Exact, t0());
        assert_eq!(
            apply(&p, &LifecycleEvent::Conflict, t0()).unwrap().0.state,
            ClaimState::Conflict
        );
        assert!(apply(&verified(), &LifecycleEvent::Conflict, t0()).is_err());
        assert!(apply(&p, &LifecycleEvent::SetAutoJoin(true), t0()).is_err());
        assert!(
            apply(&verified(), &LifecycleEvent::SetAutoJoin(true), t0())
                .unwrap()
                .0
                .auto_join
        );
    }

    #[test]
    fn scope_matching() {
        let mut c = verified();
        assert!(c.vouches_for(&d("bigbank.com")));
        assert!(!c.vouches_for(&d("eu.bigbank.com")));
        c.scope = Scope::Subdomains;
        assert!(c.vouches_for(&d("eu.bigbank.com")));
        assert!(!c.vouches_for(&d("notbigbank.com")));
        assert!(!c.vouches_for(&d("bigbank.com.evil.com")));
    }

    #[test]
    fn grace_is_discoverable_but_never_vouches() {
        let (g, _) = apply(
            &verified(),
            &LifecycleEvent::Checked(Verdict::NoMatch),
            t0(),
        )
        .unwrap();
        assert!(g.discoverable_for(&d("bigbank.com")));
        assert!(!g.vouches_for(&d("bigbank.com")));
    }

    fn verdict() -> impl Strategy<Value = Verdict> {
        prop_oneof![
            (1u32..4, any::<bool>()).prop_map(|(epoch, dnssec)| Verdict::Match { epoch, dnssec }),
            Just(Verdict::NoMatch),
            Just(Verdict::Disagree),
            Just(Verdict::Unavailable),
            Just(Verdict::DnssecFailure),
        ]
    }

    fn event() -> impl Strategy<Value = LifecycleEvent> {
        prop_oneof![
            verdict().prop_map(LifecycleEvent::Checked),
            Just(LifecycleEvent::Tick),
            Just(LifecycleEvent::Conflict),
            any::<bool>().prop_map(LifecycleEvent::SetAutoJoin),
            Just(LifecycleEvent::Withdrawn {
                by: Withdrawer::OrgAdmin,
                reason: "deleted"
            }),
        ]
    }

    proptest! {
        /// Over any sequence of events and time steps: VERIFIED is entered only on a quorum match; GRACE and REVOKED
        /// never vouch; REVOKED is never discoverable; a final state never changes; every applied event yields exactly
        /// one history event.
        #[test]
        fn lifecycle_invariants(steps in proptest::collection::vec((event(), 0i64..200), 1..40)) {
            let (mut c, _) = Claim::new(d("bigbank.com"), Scope::Subdomains, t0());
            let mut now = t0();
            for (e, hours) in steps {
                now += Duration::hours(hours);
                let before = c.clone();
                match apply(&c, &e, now) {
                    Ok((next, _event)) => {
                        prop_assert!(!before.state.is_final());
                        if next.state == ClaimState::Verified && before.state != ClaimState::Verified {
                            let via_match = matches!(e, LifecycleEvent::Checked(Verdict::Match { .. }));
                            prop_assert!(via_match);
                        }
                        if next.state != ClaimState::Verified {
                            prop_assert!(!next.vouches_for(&d("bigbank.com")));
                        }
                        if next.state == ClaimState::Revoked {
                            prop_assert!(!next.discoverable_for(&d("x.bigbank.com")));
                        }
                        c = next;
                    }
                    Err(LifecycleError::Final(s)) => prop_assert!(s.is_final() && before == c),
                    Err(LifecycleError::NotApplicable(_)) => prop_assert_eq!(&before, &c),
                }
            }
        }
    }
}
