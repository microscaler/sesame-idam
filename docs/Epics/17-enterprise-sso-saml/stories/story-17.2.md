# Story 17.2 — `dns-probe` service and quorum verdict

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** A. Domains · **Depends on:** 17.1 · **Size:** M  
> **Status:** Ready

## Goal

Look up the challenge TXT record through two independent DNS-over-HTTPS providers from a small isolated service that
holds nothing, and decide the verdict in the caller, failing closed on any disagreement.

## Context

[ADR-007 amendment of 2026-10-11](../../../ADR-007-tenant-domain-verification.md) (`dns-probe`);
[DESIGN-org-domain-verification.md §4, §7](../../../DESIGN-org-domain-verification.md);
[DESIGN-tenant-domain-verification.md §4](../../../DESIGN-tenant-domain-verification.md).

## Scope

- New BRRTRouter service `dns-probe` with OpenAPI: `POST /lookup {name, type}` (TXT only for now), health.
  Response: per provider `{provider, rcode, ad, records: [string], cname_chain: [name], ttl, negative_ttl}` or
  `{provider, error: timeout|tls|malformed|servfail}`.
- RFC 8484 DoH (GET, `application/dns-message`, DNS wire format) over rustls with WebPKI, to two providers chosen by
  configuration from Cloudflare, Google and Quad9, addressed by IP with the TLS name pinned per provider; distinct
  operators enforced at start-up.
- Query with the DO bit; report the AD bit; follow CNAME at the name (depth ≤ 8, loops refused); collect TXT
  character-strings joined per record (RFC 7208 §3.3).
- Name accepted only as `_sesame-challenge.<normalised domain>` (17.1 `normalize_domain` on the remainder): the probe
  cannot be used as a general resolver.
- `sesame_common::domain_verify::quorum(key, subject, domain, answers) -> Verdict`: `Match{epoch, dnssec}` only if
  every provider answered and each provider's records contain a matching token; `NoMatch`, `Disagree`, `Unavailable`,
  `DnssecFailure` (a provider reports the zone signed and validation fails: SERVFAIL with DO, or AD missing where the
  other provider set it).
- Client in sesame-common (`DnsProbeClient`, behind a `Probe` trait so 17.3–17.5 test against a mock).
- Timeouts (5 s per provider), response size limit (64 KiB), request body limit.

## Out of scope

- Scheduling and state (17.3, 17.5).
- Fetching anything user-supplied (IdP metadata stays with org-mgmt, 17.9).

## Design notes

- DNS wire format: a vetted crate used for encoding and decoding only (`hickory-proto`, no resolver runtime), or a
  minimal encoder and decoder for the query and TXT, CNAME and RRSIG-presence answers; decided in the story with a
  dependency review. No system resolver, no cluster DNS.
- Synchronous handlers (services run on may coroutines); the HTTP client is the one Sesame already uses for outbound
  HTTPS.
- Logs: name, providers, rcodes, durations; never anything else from the caller.

## Acceptance criteria

- [ ] Both providers matching → `Match`; one matching and one not → `Disagree`; either unreachable → `Unavailable`;
      never `Match` from a single provider.
- [ ] A signed zone with a broken signature yields `DnssecFailure`; an unsigned zone yields `Match{dnssec: false}`.
- [ ] CNAME at the challenge name is followed (depth ≤ 8); a loop fails.
- [ ] Unrelated TXT strings at the name are ignored; long TXT records split into several strings are joined correctly.
- [ ] Configuration with two providers of the same operator is refused at start-up.
- [ ] A lookup for any name not of the form `_sesame-challenge.<eligible domain>` is refused.
- [ ] The service has no secret, database or Redis configuration and starts without them.

## Tests

- Wire-format unit tests with captured responses; a mock DoH server pair for verdicts; `quorum` unit and property tests
  (no single-provider path reaches `Match`); an ignored-by-default live test against a real signed and unsigned zone.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
