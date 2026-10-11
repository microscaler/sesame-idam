//! The verification verdict over `dns-probe`'s answers (ADR-007 amendment of 2026-10-11).
//!
//! `dns-probe` only reports what each DNS-over-HTTPS provider said; the decision is made here, by the caller that
//! holds the keyring. A domain is verified only when every provider answered and each provider's records contain
//! the expected token: one provider can never verify a domain on its own.

use super::dns::{rcode, DecodedAnswer};
use super::keyring::{EnrollKeyring, KeyringError};
use super::token::{verify_record, ClaimSubject};
use super::Domain;

/// Fewest providers whose agreement can verify a domain.
pub const QUORUM: usize = 2;

/// One provider's result, as `dns-probe` returns it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProviderAnswer {
    /// Provider label from `dns-probe`'s configuration (`cloudflare`, `google`, `quad9`).
    pub provider: String,
    /// The decoded answer, or why there is none.
    #[serde(flatten)]
    pub outcome: ProviderOutcome,
}

/// A provider's answer or failure.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProviderOutcome {
    /// The provider answered.
    Answered(DecodedAnswer),
    /// No usable answer (`timeout`, `tls`, `http`, `malformed`).
    Failed {
        /// Fixed reason.
        error: String,
    },
}

/// The verification result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
    /// Every provider returned a record matching the expected token.
    Match {
        /// Key epoch that matched (stored with the claim).
        epoch: u32,
        /// Every provider validated the answer with DNSSEC.
        dnssec: bool,
    },
    /// The providers agree there is no matching record.
    NoMatch,
    /// The providers disagree (possible attack, or propagation in progress): fail closed, retry later.
    Disagree,
    /// A provider could not be asked or did not answer usefully: could not check.
    Unavailable,
    /// A provider reported a DNSSEC validation failure, or validated what another could not.
    DnssecFailure,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Seen {
    Match(u32),
    Missing,
}

/// Decide the verdict for `subject` and `domain` from the providers' answers.
///
/// # Errors
///
/// [`KeyringError`] when the expected token cannot be computed; callers treat it as "could not check".
pub fn quorum(
    keyring: &dyn EnrollKeyring,
    subject: &ClaimSubject,
    domain: &Domain,
    answers: &[ProviderAnswer],
) -> Result<Verdict, KeyringError> {
    let mut providers: Vec<&str> = answers.iter().map(|a| a.provider.as_str()).collect();
    providers.sort_unstable();
    providers.dedup();
    if providers.len() < QUORUM || providers.len() != answers.len() {
        return Ok(Verdict::Unavailable);
    }

    let mut decoded = Vec::with_capacity(answers.len());
    for answer in answers {
        match &answer.outcome {
            ProviderOutcome::Failed { .. } => return Ok(Verdict::Unavailable),
            ProviderOutcome::Answered(a) => decoded.push(a),
        }
    }
    if decoded.iter().any(|a| a.dnssec_failed()) {
        return Ok(Verdict::DnssecFailure);
    }
    if decoded
        .iter()
        .any(|a| a.rcode != rcode::NOERROR && a.rcode != rcode::NXDOMAIN)
    {
        return Ok(Verdict::Unavailable);
    }

    let mut seen = Vec::with_capacity(decoded.len());
    for a in &decoded {
        let found = if a.rcode == rcode::NXDOMAIN {
            None
        } else {
            verify_record(keyring, subject, domain, &a.records)?
        };
        seen.push(found.map_or(Seen::Missing, Seen::Match));
    }

    let matches: Vec<u32> = seen
        .iter()
        .filter_map(|s| match s {
            Seen::Match(e) => Some(*e),
            Seen::Missing => None,
        })
        .collect();
    if matches.is_empty() {
        return Ok(Verdict::NoMatch);
    }
    if matches.len() != seen.len() {
        return Ok(Verdict::Disagree);
    }
    let ad: Vec<bool> = decoded.iter().map(|a| a.ad).collect();
    if ad.iter().any(|v| *v) && !ad.iter().all(|v| *v) {
        // One provider validated the zone with DNSSEC and another did not: something is in the way.
        return Ok(Verdict::DnssecFailure);
    }
    Ok(Verdict::Match {
        epoch: matches.iter().copied().max().unwrap_or(0),
        dnssec: ad.iter().all(|v| *v),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_verify::{expected_record, normalize_domain, LocalKeyring, Scope};
    use uuid::Uuid;

    fn setup() -> (LocalKeyring, ClaimSubject, Domain, String) {
        let ring = LocalKeyring::new((3, vec![9u8; 32]), Some((2, vec![8u8; 32]))).expect("key");
        let subject = ClaimSubject::organisation(
            "acme",
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            Scope::Exact,
        )
        .expect("subject");
        let domain = normalize_domain("bigbank.com").expect("domain");
        let record = expected_record(&ring, &subject, &domain)
            .expect("mac")
            .value;
        (ring, subject, domain, record)
    }

    fn answered(provider: &str, records: &[&str], ad: bool, rcode: u8) -> ProviderAnswer {
        ProviderAnswer {
            provider: provider.to_string(),
            outcome: ProviderOutcome::Answered(DecodedAnswer {
                rcode,
                ad,
                records: records.iter().map(ToString::to_string).collect(),
                ..DecodedAnswer::default()
            }),
        }
    }

    fn failed(provider: &str) -> ProviderAnswer {
        ProviderAnswer {
            provider: provider.to_string(),
            outcome: ProviderOutcome::Failed {
                error: "timeout".to_string(),
            },
        }
    }

    #[test]
    fn verdicts() {
        let (ring, s, d, rec) = setup();
        let r = rec.as_str();
        let v = |answers: &[ProviderAnswer]| quorum(&ring, &s, &d, answers).expect("keyring");
        assert_eq!(
            v(&[
                answered("cloudflare", &[r], true, 0),
                answered("quad9", &["x", r], true, 0)
            ]),
            Verdict::Match {
                epoch: 3,
                dnssec: true
            }
        );
        assert_eq!(
            v(&[
                answered("cloudflare", &[r], false, 0),
                answered("quad9", &[r], false, 0)
            ]),
            Verdict::Match {
                epoch: 3,
                dnssec: false
            }
        );
        assert_eq!(
            v(&[
                answered("cloudflare", &[r], false, 0),
                answered("quad9", &[], false, 0)
            ]),
            Verdict::Disagree
        );
        assert_eq!(
            v(&[
                answered("cloudflare", &[], false, 3),
                answered("quad9", &["v=spf1"], false, 0)
            ]),
            Verdict::NoMatch
        );
        assert_eq!(
            v(&[answered("cloudflare", &[r], false, 0), failed("quad9")]),
            Verdict::Unavailable
        );
        assert_eq!(
            v(&[
                answered("cloudflare", &[r], true, 0),
                answered("quad9", &[r], false, 0)
            ]),
            Verdict::DnssecFailure
        );
        let mut bogus = answered("quad9", &[], false, 2);
        if let ProviderOutcome::Answered(a) = &mut bogus.outcome {
            a.ede = Some(6);
        }
        assert_eq!(
            v(&[answered("cloudflare", &[r], true, 0), bogus]),
            Verdict::DnssecFailure
        );
        assert_eq!(
            v(&[
                answered("cloudflare", &[r], false, 0),
                answered("quad9", &[], false, 2)
            ]),
            Verdict::Unavailable
        );
    }

    #[test]
    fn a_single_provider_never_verifies() {
        let (ring, s, d, rec) = setup();
        let r = rec.as_str();
        assert_eq!(
            quorum(&ring, &s, &d, &[answered("cloudflare", &[r], true, 0)]).expect("k"),
            Verdict::Unavailable
        );
        // The same provider twice is still one provider.
        assert_eq!(
            quorum(
                &ring,
                &s,
                &d,
                &[
                    answered("cloudflare", &[r], true, 0),
                    answered("cloudflare", &[r], true, 0)
                ]
            )
            .expect("k"),
            Verdict::Unavailable
        );
        assert_eq!(quorum(&ring, &s, &d, &[]).expect("k"), Verdict::Unavailable);
    }

    #[test]
    fn previous_epoch_records_still_match() {
        let (ring, s, d, _) = setup();
        let old = LocalKeyring::new((2, vec![8u8; 32]), None).expect("key");
        let rec = expected_record(&old, &s, &d).expect("mac").value;
        assert_eq!(
            quorum(
                &ring,
                &s,
                &d,
                &[
                    answered("google", &[&rec], false, 0),
                    answered("quad9", &[&rec], false, 0)
                ]
            )
            .expect("k"),
            Verdict::Match {
                epoch: 2,
                dnssec: false
            }
        );
    }

    #[test]
    fn wire_shape_round_trips() {
        let a = answered("google", &["x"], true, 0);
        let json = serde_json::to_value(&a).expect("json");
        assert_eq!(json["status"], "answered");
        assert_eq!(json["provider"], "google");
        let back: ProviderAnswer = serde_json::from_value(json).expect("back");
        assert_eq!(back, a);
        let f = serde_json::to_value(failed("quad9")).expect("json");
        assert_eq!(f["status"], "failed");
        assert_eq!(f["error"], "timeout");
    }
}
