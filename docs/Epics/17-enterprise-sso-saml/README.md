# Epic 17: Enterprise SSO — SAML and verified organisation domains

> **Status:** In progress (17.1 done 2026-10-10, 17.2 service done, 17.3 and 17.4 done 2026-10-11)
> **Program:** Enterprise identity
> **Decisions:** [ADR-014](../../ADR-014-saml-signature-verification.md) (samael and libxmlsec1, isolated verifier),
> [ADR-007](../../ADR-007-tenant-domain-verification.md) as amended 2026-10-10 (organisation domains) and 2026-10-11
> (`dns-probe`)
> **Designs:** [DESIGN-saml-sp.md](../../DESIGN-saml-sp.md), [DESIGN-org-domain-verification.md](../../DESIGN-org-domain-verification.md)
> **Dependencies:** Epic 10 (tenancy), Epic 11 (client registry), Epic 12 (authorization server), ADR-005 (RLS),
> ADR-006 (secrets born in the backend)

## Outcome

A member of an organisation signs in to any Sesame relying party through the organisation's own SAML identity
provider — Microsoft Entra ID, Okta or Google Workspace first — and receives ordinary Sesame tokens. The organisation
proves the email domains it vouches for with a DNS TXT record that Sesame keeps re-checking daily, and an organisation
admin configures all of it without help from the platform.

## Why this epic exists

- Organisations buying from Sesame's tenants expect to sign in with their corporate identity provider. Without SAML,
  every such customer needs local passwords or social accounts.
- SAML trusts what an IdP asserts about an email address. Without proof that the organisation controls the domain,
  any organisation could configure an IdP that asserts `@bigbank.com` users and take over their accounts. Today
  `org_domains.verified` is a writable flag with no proof behind it, and the domain API is a stub.
- XML signature verification has a long record of authentication bypasses in mature libraries (ADR-014). It needs a
  vetted engine, isolation and an adversarial corpus, not ad-hoc code.

## Scope

- Organisation domain claims proven by DNS TXT, with daily re-verification, grace and revocation; DNS lookups made by
  `dns-probe`, an isolated service whose only egress is two fixed DNS-over-HTTPS providers.
- `saml-verifier`: an isolated, stateless service verifying responses with samael and libxmlsec1, with Sesame's own
  pre- and post-checks and a positive and negative corpus.
- Organisation-scoped SAML connection management, IdP metadata parsing, refresh and certificate rollover.
- SP-initiated sign-in: signed AuthnRequest, assertion consumer service, replay protection, identity linking and
  just-in-time provisioning through verified domains, one-time code exchange, SP metadata, SSO discovery.
- Hosted "Sign in with SSO", tenant- and organisation-console administration, `sesame-idam-client` contract.
- Deployment of the new pieces and live acceptance with the three IdPs.

## Non-goals

- IdP-initiated SAML, single logout, encrypted assertions, SCIM (DESIGN-saml-sp.md §13).
- Sesame acting as a SAML IdP for other services.
- The tenant issuer registry (ADR-004); it reuses 17.1 and 17.2 later.
- A pure-Rust XML signature engine (xml-sec is a separate side project, ADR-014 §6).

## Stories

| Story | Title | Track | Depends on | Size |
|---|---|---|---|---|
| [17.1](stories/story-17.1.md) | Domain claim primitives: normalisation, eligibility, tokens | A. Domains | — | M ✅ |
| [17.2](stories/story-17.2.md) | `dns-probe` service and quorum verdict | A. Domains | 17.1 | M |
| [17.3](stories/story-17.3.md) | Domain claim schema and lifecycle | A. Domains | 17.1 | M ✅ |
| [17.4](stories/story-17.4.md) | Organisation domain API | A. Domains | 17.2, 17.3 | M ✅ |
| [17.5](stories/story-17.5.md) | Daily re-verification job and notifications | A. Domains | 17.4 | M |
| [17.6](stories/story-17.6.md) | `saml-verifier` service | B. Verifier | — | L |
| [17.7](stories/story-17.7.md) | Verification corpus and fuzzing | B. Verifier | 17.6 | L |
| [17.8](stories/story-17.8.md) | SAML connection schema and organisation-scoped API | C. Connections | 17.3 | M |
| [17.9](stories/story-17.9.md) | IdP metadata intake, refresh and certificate rollover | C. Connections | 17.6, 17.8 | M |
| [17.10](stories/story-17.10.md) | SP key and signed AuthnRequest (`saml_login`) | D. Sign-in | 17.8 | M |
| [17.11](stories/story-17.11.md) | Assertion consumer service and replay protection | D. Sign-in | 17.6, 17.10 | M |
| [17.12](stories/story-17.12.md) | Identity linking and just-in-time provisioning | D. Sign-in | 17.4, 17.11 | M |
| [17.13](stories/story-17.13.md) | Code exchange, SP metadata and SSO discovery | D. Sign-in | 17.12 | M |
| [17.14](stories/story-17.14.md) | Test sign-in and connection enablement | D. Sign-in | 17.13 | S |
| [17.15](stories/story-17.15.md) | Hosted "Sign in with SSO" and console administration | E. Surfaces | 17.4, 17.14 | L |
| [17.16](stories/story-17.16.md) | `sesame-idam-client` contract | E. Surfaces | 17.13 | S |
| [17.17](stories/story-17.17.md) | Deployment: verifier, probe, job, secrets, policies | F. Delivery | 17.5, 17.6 | M |
| [17.18](stories/story-17.18.md) | Live acceptance and security evidence | F. Delivery | all | L |

Tracks A and B have no dependency on each other and can proceed in parallel.

```
A  17.1 ─► 17.2 ─┐
        └► 17.3 ─┴► 17.4 ─► 17.5 ───────────────────────────────┐
B  17.6 ─► 17.7                                                 │
C          17.3 ─► 17.8 ─► 17.9 (needs 17.6)                    ├► 17.17 ─► 17.18
D                  17.8 ─► 17.10 ─► 17.11 ─► 17.12 ─► 17.13 ─► 17.14
E                              17.4 + 17.14 ─► 17.15;  17.13 ─► 17.16
```

## Required design decisions (settled)

- **Verifier**: samael with libxmlsec1, pinned exactly, in `saml-verifier` (no database, Redis, keys or egress).
  Sesame enforces what samael does not: certificates always configured, algorithm allow-list, no DOCTYPE, comments,
  processing instructions, CDATA or encrypted assertions, exactly one assertion, replay (ADR-014 §1–3).
- **Flow**: SP-initiated only; HTTP-Redirect out (signed with ring, no XML signing), HTTP-POST back; one SP entity and
  ACS per connection.
- **Trust in email**: first-time linking and provisioning only through a domain in state VERIFIED; grace binds nothing
  new; existing links bind to (connection, NameID), not to DNS.
- **Domains**: DNS TXT at `_sesame-challenge.<domain>`, token bound to tenant, organisation, scope and purpose; DoH
  quorum of two providers, queried by `dns-probe` (no secrets; the caller decides); daily re-check, 72 h grace; public suffixes and consumer mail providers never claimable;
  one holder per tenant.
- **Reachability**: browser routes need only browser reachability; a private (VPN-only) deployment is supported.

## Acceptance gate

- [ ] No API can mark a domain verified; only a quorum DNS match does, and daily re-checks move a lapsed domain to
      GRACE and then REVOKED.
- [ ] Public suffixes and consumer mail providers cannot be claimed; a record for one organisation never verifies
      another's claim.
- [ ] The verifier refuses every case of the negative corpus (one per rule, XSW1–8, comment injection, XXE, parser
      differentials) and accepts the positive corpus from all three IdPs.
- [ ] `saml-verifier` runs with no secrets, no database or Redis access and no egress; only identity-login and
      org-mgmt can reach it.
- [ ] `dns-probe` runs with no secrets, no database or Redis access and egress only to its two DoH providers; only
      org-mgmt and the re-verification job can reach it; org-mgmt's metadata fetches cannot reach private addresses.
- [ ] A user is linked or provisioned only through a VERIFIED domain of the connection's organisation, never across
      tenants.
- [ ] Replayed responses, unsolicited responses and responses for another connection are refused.
- [ ] The pending SAML contract tests in sesame-idam and sesame-idam-client pass and gate.
- [ ] Live sign-in through Entra ID, Okta and Google Workspace to a relying party in pp, each test organisation
      holding a verified domain.
- [ ] Every connection and domain lifecycle change emits an audit event; admins are notified of GRACE and REVOKED.

## Implementation evidence

Recorded per story as it lands.

| Story | Evidence |
|---|---|
| 17.1 | `sesame_common::domain_verify`; amended 2026-10-11 with per-claim tokens and the Cloud KMS HSM keyring (fake KMS and metadata server tests for round trip, integrity refusals, outages); 29 unit tests and 7 integration tests (`tests/domain_verify.rs`: published vectors reproduced byte for byte against an independent Python implementation, 4 proptest properties over field binding, record binding, idempotent normalisation and arbitrary TXT input); clippy pedantic and rustfmt clean on the new code. |
