# Story 17.2 — DNS-over-HTTPS quorum resolver

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** A. Domains · **Depends on:** 17.1 · **Size:** M  
> **Status:** Ready

## Goal

Look up the challenge TXT record through two independent DNS-over-HTTPS providers, validate DNSSEC where the zone is signed, and return a verdict that fails closed on any disagreement.

## Context

[DESIGN-tenant-domain-verification.md §4](../../../DESIGN-tenant-domain-verification.md); [DESIGN-org-domain-verification.md §4](../../../DESIGN-org-domain-verification.md).

## Scope

- `domain_verify::resolver`: RFC 8484 DoH (GET, `application/dns-message`, DNS wire format) over rustls with WebPKI, to two providers chosen by configuration from Cloudflare, Google and Quad9 (distinct operators enforced).
- Query with the DO bit; read the AD bit; follow CNAME at the challenge name (bounded depth); collect TXT character-strings (joined per RR as RFC 7208 does).
- Verdict: `Match{dnssec}` only if both providers return a matching record; `NoMatch`, `Disagree`, `Unavailable` otherwise; DNSSEC: if either provider reports the zone signed and validation fails (SERVFAIL with DO), the verdict is `DnssecFailure`.
- Timeouts (5 s per query), response size limit, TTL and negative-cache TTL surfaced for the scheduler.

## Out of scope

- Scheduling and state (17.3, 17.5).

## Design notes

- A small DNS message encoder/decoder for the query and TXT/CNAME answers only, or a vetted crate (`hickory-proto` wire format only, no resolver runtime) — decided in the story with a dependency review; no system resolver.
- Synchronous API (services run on may coroutines); the HTTP client is the one Sesame already uses for outbound HTTPS.
- A `Resolver` trait so 17.3–17.5 test against a mock pair.

## Acceptance criteria

- [ ] Both providers matching → `Match`; one matching and one not → `Disagree`; either unreachable → `Unavailable`; never `Match` from a single provider.
- [ ] A signed zone with a broken signature yields `DnssecFailure`; an unsigned zone yields `Match{dnssec: false}`.
- [ ] CNAME at the challenge name is followed (depth ≤ 8); a loop fails.
- [ ] Unrelated TXT strings at the name are ignored; long TXT records split into several strings are joined correctly.
- [ ] Configuration with two providers of the same operator is refused at start-up.

## Tests

- Wire-format unit tests with captured responses; a mock DoH server pair for verdicts; an ignored-by-default live test against a real signed and unsigned zone.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
