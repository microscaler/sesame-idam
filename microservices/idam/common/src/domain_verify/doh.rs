//! DNS-over-HTTPS lookups (RFC 8484) for `dns-probe`, and the client org-mgmt and the re-verification job use to
//! call `dns-probe` (ADR-007 amendment of 2026-10-11).
//!
//! `dns-probe` asks every configured provider in parallel and reports each answer as it came; it never sees a key
//! and never decides. The caller turns the answers into a [`super::Verdict`] with [`super::quorum`].

use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

use super::dns::{decode_txt_response, encode_query, TYPE_TXT};
use super::quorum::{ProviderAnswer, ProviderOutcome};
use super::{check_eligible, normalize_domain, EligibilityPolicy, CHALLENGE_LABEL};
use crate::http::{fetch_get, fetch_post, HttpFetchOptions};

/// Environment variable listing the providers: `label=https://…/dns-query`, comma-separated.
pub const PROVIDERS_ENV: &str = "SESAME_DNS_PROBE_PROVIDERS";
/// Default providers, addressed by IP so `dns-probe` needs no cluster DNS and its egress policy is a list of
/// addresses (the providers' certificates carry these IPs).
pub const DEFAULT_PROVIDERS: &str =
    "cloudflare=https://1.1.1.1/dns-query,quad9=https://9.9.9.9/dns-query";
/// Per-provider timeout.
pub const PROVIDER_TIMEOUT: Duration = Duration::from_secs(5);
/// Largest `DoH` response read.
const MAX_DOH_RESPONSE: usize = 64 * 1024;

/// A configured `DoH` provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    /// Operator label (`cloudflare`, `google`, `quad9`).
    pub label: String,
    /// RFC 8484 endpoint.
    pub url: String,
}

/// Why the provider configuration was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderConfigError {
    /// Not `label=https://…` pairs.
    #[error("DoH provider list malformed: {0}")]
    Malformed(&'static str),
    /// Fewer than two distinct operators: no quorum is possible.
    #[error("DoH providers must be at least two distinct operators")]
    NotDistinct,
}

/// Parse `label=url,label=url`. Requires HTTPS (localhost excepted, for tests) and two distinct operators.
///
/// # Errors
///
/// [`ProviderConfigError`].
pub fn parse_providers(spec: &str) -> Result<Vec<Provider>, ProviderConfigError> {
    let mut providers = Vec::new();
    for item in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (label, url) = item
            .split_once('=')
            .ok_or(ProviderConfigError::Malformed("expected label=url"))?;
        let label = label.trim().to_ascii_lowercase();
        let url = url.trim().to_string();
        let parsed = url::Url::parse(&url).map_err(|_| ProviderConfigError::Malformed("url"))?;
        let local = matches!(parsed.host_str(), Some("127.0.0.1" | "localhost"));
        if label.is_empty() || (parsed.scheme() != "https" && !local) {
            return Err(ProviderConfigError::Malformed(
                "label and https url required",
            ));
        }
        providers.push(Provider { label, url });
    }
    let mut labels: Vec<&str> = providers.iter().map(|p| p.label.as_str()).collect();
    labels.sort_unstable();
    labels.dedup();
    if labels.len() < 2 || labels.len() != providers.len() {
        return Err(ProviderConfigError::NotDistinct);
    }
    Ok(providers)
}

/// The challenge name for a lookup request, or `None` when the name is not `_sesame-challenge.<claimable domain>`:
/// `dns-probe` is not a general resolver.
#[must_use]
pub fn checked_challenge_name(name: &str) -> Option<String> {
    let rest = name
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase()
        .strip_prefix(&format!("{CHALLENGE_LABEL}."))?
        .to_string();
    let domain = normalize_domain(&rest).ok()?;
    check_eligible(&domain, &EligibilityPolicy::default()).ok()?;
    Some(format!("{CHALLENGE_LABEL}.{domain}"))
}

fn failed(provider: &Provider, error: &str) -> ProviderAnswer {
    ProviderAnswer {
        provider: provider.label.clone(),
        outcome: ProviderOutcome::Failed {
            error: error.to_string(),
        },
    }
}

/// Ask one provider for the TXT records at `name` (RFC 8484 GET, `application/dns-message`).
#[must_use]
pub fn query_provider(provider: &Provider, name: &str, timeout: Duration) -> ProviderAnswer {
    let Ok(query) = encode_query(name, TYPE_TXT) else {
        return failed(provider, "bad_name");
    };
    let sep = if provider.url.contains('?') { '&' } else { '?' };
    let url = format!("{}{sep}dns={}", provider.url, URL_SAFE_NO_PAD.encode(query));
    let options = HttpFetchOptions {
        timeout,
        max_body_bytes: MAX_DOH_RESPONSE,
        extra_headers: vec![("Accept".to_string(), "application/dns-message".to_string())],
    };
    let Ok((status, body)) = fetch_get(&url, &options) else {
        return failed(provider, "unreachable");
    };
    if status != 200 {
        return failed(provider, "http");
    }
    match decode_txt_response(&body, name) {
        Ok(answer) => ProviderAnswer {
            provider: provider.label.clone(),
            outcome: ProviderOutcome::Answered(answer),
        },
        Err(_) => failed(provider, "malformed"),
    }
}

/// Ask every provider, in parallel (one coroutine each), and return their answers in configuration order.
#[must_use]
pub fn query_all(providers: &[Provider], name: &str, timeout: Duration) -> Vec<ProviderAnswer> {
    let handles: Vec<_> = providers
        .iter()
        .cloned()
        .map(|p| {
            let name = name.to_string();
            // SAFETY: the coroutine owns its data and touches no thread-local state.
            unsafe { may::coroutine::spawn(move || query_provider(&p, &name, timeout)) }
        })
        .collect();
    handles
        .into_iter()
        .zip(providers)
        .map(|(h, p)| h.join().unwrap_or_else(|_| failed(p, "internal")))
        .collect()
}

/// Client for `dns-probe` (`POST /lookup`), used by org-mgmt and the re-verification job.
#[derive(Debug, Clone)]
pub struct DnsProbeClient {
    base_url: String,
    timeout: Duration,
}

/// Why `dns-probe` could not be asked. Always "could not check", never "record missing".
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProbeError {
    /// Unreachable, refused, or an answer not understood.
    #[error("dns-probe unavailable: {0}")]
    Unavailable(String),
}

#[derive(serde::Deserialize)]
struct LookupResponse {
    answers: Vec<ProviderAnswer>,
}

impl DnsProbeClient {
    /// `base_url` is `dns-probe`'s in-cluster URL, e.g. `http://dns-probe:8080/dns-probe/v1`.
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            timeout: PROVIDER_TIMEOUT + Duration::from_secs(2),
        }
    }

    /// The providers' answers for `name`.
    ///
    /// # Errors
    ///
    /// [`ProbeError::Unavailable`].
    pub fn lookup(&self, name: &str) -> Result<Vec<ProviderAnswer>, ProbeError> {
        let body = serde_json::to_vec(&serde_json::json!({ "name": name }))
            .map_err(|_| ProbeError::Unavailable("request encoding".to_string()))?;
        let options = HttpFetchOptions {
            timeout: self.timeout,
            max_body_bytes: 2 * MAX_DOH_RESPONSE,
            extra_headers: vec![("Content-Type".to_string(), "application/json".to_string())],
        };
        let (status, resp) = fetch_post(&format!("{}/lookup", self.base_url), &body, &options)
            .map_err(|_| ProbeError::Unavailable("unreachable".to_string()))?;
        if status != 200 {
            return Err(ProbeError::Unavailable(format!("status {status}")));
        }
        serde_json::from_slice::<LookupResponse>(&resp)
            .map(|r| r.answers)
            .map_err(|_| ProbeError::Unavailable("answer not understood".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::dns::build::{response, txt, Rr};
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    const NAME: &str = "_sesame-challenge.bigbank.com";

    /// A fake `DoH` server answering every query with `body` and `status`.
    fn fake_doh(status: &'static str, body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}/dns-query", listener.local_addr().expect("addr"));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { break };
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
                assert!(req.starts_with("get /dns-query?dns="));
                assert!(req.contains("accept: application/dns-message"));
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/dns-message\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                s.write_all(head.as_bytes()).ok();
                s.write_all(&body).ok();
            }
        });
        url
    }

    fn provider(label: &str, url: String) -> Provider {
        Provider {
            label: label.to_string(),
            url,
        }
    }

    #[test]
    fn provider_configuration() {
        let p = parse_providers(DEFAULT_PROVIDERS).expect("defaults parse");
        assert_eq!(p.len(), 2);
        assert!(parse_providers("cloudflare=https://1.1.1.1/dns-query").is_err());
        assert_eq!(
            parse_providers("a=https://1.1.1.1/dns-query,a=https://1.0.0.1/dns-query"),
            Err(ProviderConfigError::NotDistinct)
        );
        assert!(parse_providers("a=http://1.1.1.1/dns-query,b=https://9.9.9.9/dns-query").is_err());
    }

    #[test]
    fn only_challenge_names_of_claimable_domains() {
        assert_eq!(
            checked_challenge_name("_Sesame-Challenge.BigBank.com."),
            Some(NAME.to_string())
        );
        assert_eq!(checked_challenge_name("bigbank.com"), None);
        assert_eq!(checked_challenge_name("_sesame-challenge.gmail.com"), None);
        assert_eq!(checked_challenge_name("_sesame-challenge.com"), None);
        assert_eq!(checked_challenge_name("_other.bigbank.com"), None);
    }

    #[test]
    fn answers_and_failures_per_provider() {
        let ok = response(
            0x0180 | 0x0020,
            NAME,
            &[Rr {
                name: NAME,
                rtype: 16,
                ttl: 300,
                rdata: txt(&["sesame-domain-verify=v1;t=x"]),
            }],
            &[],
            None,
        );
        let a = query_provider(
            &provider("cloudflare", fake_doh("200 OK", ok)),
            NAME,
            PROVIDER_TIMEOUT,
        );
        match a.outcome {
            ProviderOutcome::Answered(d) => {
                assert_eq!(d.records, vec!["sesame-domain-verify=v1;t=x"]);
                assert!(d.ad);
            }
            ProviderOutcome::Failed { error } => panic!("failed: {error}"),
        }
        let bad = query_provider(
            &provider("quad9", fake_doh("200 OK", vec![1, 2, 3])),
            NAME,
            PROVIDER_TIMEOUT,
        );
        assert_eq!(
            bad.outcome,
            ProviderOutcome::Failed {
                error: "malformed".to_string()
            }
        );
        let refused = query_provider(
            &provider("google", fake_doh("503 Service Unavailable", vec![])),
            NAME,
            PROVIDER_TIMEOUT,
        );
        assert_eq!(
            refused.outcome,
            ProviderOutcome::Failed {
                error: "http".to_string()
            }
        );
        let dead = TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}/dns-query", dead.local_addr().expect("addr"));
        drop(dead);
        let down = query_provider(&provider("x", url), NAME, PROVIDER_TIMEOUT);
        assert_eq!(
            down.outcome,
            ProviderOutcome::Failed {
                error: "unreachable".to_string()
            }
        );
    }

    #[test]
    fn all_providers_are_asked_and_reported_in_order() {
        let ok = response(0x0180, NAME, &[], &[], None);
        let providers = vec![
            provider("cloudflare", fake_doh("200 OK", ok.clone())),
            provider("quad9", fake_doh("200 OK", ok)),
        ];
        let answers = query_all(&providers, NAME, PROVIDER_TIMEOUT);
        let labels: Vec<&str> = answers.iter().map(|a| a.provider.as_str()).collect();
        assert_eq!(labels, vec!["cloudflare", "quad9"]);
        assert!(answers
            .iter()
            .all(|a| matches!(a.outcome, ProviderOutcome::Answered(_))));
    }

    /// Live check against the default providers (network needed): `--ignored`.
    #[test]
    #[ignore = "live network: run with --ignored where egress to the DoH providers is allowed"]
    fn live_default_providers_answer_over_ip_urls() {
        let providers = parse_providers(DEFAULT_PROVIDERS).expect("defaults");
        let name = "_sesame-challenge.example.com";
        let answers = query_all(&providers, name, PROVIDER_TIMEOUT);
        for a in &answers {
            assert!(
                matches!(a.outcome, ProviderOutcome::Answered(_)),
                "{} did not answer: {:?}",
                a.provider,
                a.outcome
            );
        }
    }
}
