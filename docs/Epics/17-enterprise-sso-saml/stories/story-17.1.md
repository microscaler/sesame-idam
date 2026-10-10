# Story 17.1 — Domain claim primitives: normalisation, eligibility, tokens

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** A. Domains · **Depends on:** — · **Size:** M  
> **Status:** In progress

## Goal

One library function set that every domain-verification path uses: turn user input into a canonical domain, decide whether it may be claimed, and derive and compare the TXT token for organisation and tenant-issuer claims.

## Context

[DESIGN-org-domain-verification.md §2–3](../../../DESIGN-org-domain-verification.md); [ADR-007](../../../ADR-007-tenant-domain-verification.md) decisions 1–2 and amendment; the tenant design §2 for the issuer purpose.

## Scope

- `sesame_common::domain_verify` module.
- `normalize_domain(input)`: trim, strip one trailing dot, lower-case, IDNA to A-label (UTS-46, non-transitional), reject IP literals, empty labels, labels over 63 or names over 253 octets, characters outside LDH after conversion.
- `check_eligible(domain, policy)`: refuse public suffixes and anything above a registrable domain (Public Suffix List, compiled in and pinned), consumer mail providers (a maintained list in the crate, overridable per deployment), and the deployment's own domains (configuration).
- `ClaimSubject` = Organisation{tenant_id, org_id, scope: Exact|Subdomains} or TenantIssuer{tenant_id}; `challenge_name(domain)` = `_sesame-challenge.<domain>`; `expected_record(key, epoch, subject, domain)` = `sesame-domain-verify=v1;t=<token>` with the token per the designs (HMAC-SHA256, 0x00-separated fields, base64url without padding).
- `EnrollKey` holding the current and previous epoch keys (rotation overlap) loaded from a file path (secret mount), never from git; `Debug` redacts.
- `matches(txt_strings, expected_tokens)`: select only `sesame-domain-verify=` strings, ignore unknown versions, constant-time comparison against every acceptable epoch.

## Out of scope

- Network I/O (17.2).
- Persistence and lifecycle (17.3).

## Design notes

- Pure functions plus `EnrollKey`; no global state; `ring::hmac` and `ring::constant_time`-equivalent comparison.
- Errors are a typed enum (`InvalidDomain`, `PublicSuffix`, `ConsumerProvider`, `ReservedDomain`, …) with stable codes the API maps to `domain_unavailable` or `invalid_domain`.
- Test vectors live in `tests/fixtures/domain_verify_vectors.json` so other implementations (or the tenant issuer registry) can reuse them.

## Acceptance criteria

- [ ] `BüCher.Example.` normalises to `xn--bcher-kva.example`; IP literals, `a..b`, 64-octet labels and 254-octet names are refused.
- [ ] `com`, `co.uk`, `github.io` (public suffixes) and `gmail.com`, `outlook.com` are refused with distinct codes; `bigbank.com` and `eu.bigbank.co.uk` are accepted.
- [ ] Tokens differ when any of tenant, organisation, scope, purpose, domain or epoch differs (property test).
- [ ] A record string with another version, extra whitespace inside, or another prefix never matches; a match succeeds with either the current or the previous epoch key and fails with a retired one.
- [ ] Published vectors reproduce the designs' construction byte for byte.
- [ ] No secret appears in `Debug` output or errors.

## Tests

- Unit tests and property tests (proptest) in sesame-common; vectors file.
- Clippy and rustfmt clean.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
