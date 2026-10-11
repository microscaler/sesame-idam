# ADR-007: Tenant domain-control verification (DNS TXT challenge with continuous re-verification)

Status: ACCEPTED (2026-10-10; proposed 2026-07-22), amended 2026-10-10 to cover organisation email domains (below)
Related: ADR-004 (platform-tenant provisioning), ADR-006 (shared signing keys), ADR-014 (SAML),
`docs/DESIGN-org-domain-verification.md` (organisation domains),
`docs/DESIGN-tenant-domain-verification.md` (accompanying design, diagrams,
threat model, RFC compliance matrix).

## Context

Sesame's SaaS-of-SaaS tenancy (ADR-004) lets a tenant (e.g. `loadlinker.com`)
operate its own token issuer. Verifiers trust an issuer because the platform's
tenant registry says "issuer `https://idam.loadlinker.com` belongs to tenant
loadlinker" — so the registry entry is a trust anchor, and the ceremony that
creates it is where tenant trust is *born*. Cryptographic material (signing
keys, JWKS) cannot bootstrap this trust: keys prove nothing about ownership;
they only extend trust established by other means.

The question this ADR answers: **how does Sesame prove, at enrollment and
continuously thereafter, that the party registering an issuer actually
controls the domain that issuer lives under?**

## Decision

Adopt **DNS TXT domain-control verification** in the style of ACME DNS-01
(RFC 8555 §8.4), with deliberate enhancements, specified fully in the design
document:

1. **Challenge token = HMAC, not ciphertext, not a signature.**
   `token = base64url(HMAC-SHA256(K_enroll, tenant_id ‖ domain ‖ purpose ‖ epoch))`
   (RFC 2104). The record's security comes from *zone control* — only the
   domain owner can publish it — plus unguessability. A PGP-signed string was
   considered and REJECTED: a signature proves possession of a key, but the
   tenant has no trusted key yet (the bootstrapping gap this ceremony exists
   to close), Sesame gains nothing from verifying its own signature, and
   armored signatures exceed practical TXT sizing for zero added security.
   HMAC derivation additionally makes verification **stateless** — any
   replica recomputes the expected value; nothing is stored or synced.

2. **Record placement per RFC 8552 (underscored node names):**
   `_sesame-challenge.<issuer-host>` TXT `"sesame-domain-verify=v1;t=<token>"`.
   Verification is scoped to the exact issuer host (least privilege); apex
   verification is an explicit opt-in granting org-wide scope.

3. **Hardened resolution path**: quorum of independent DoH resolvers
   (RFC 8484), DNSSEC validation where the zone is signed (RFC 4033–4035),
   constant-time token comparison.

4. **Continuous re-verification (enhancement over industry practice).**
   Most SaaS verifies once; a lapsed and re-registered domain then *inherits
   the tenant's standing*. Sesame re-checks the TXT record on a **daily
   cadence** (with a TTL-respecting floor), applies a **grace window** on
   failure with loud alerting, and revokes issuer trust if the record does not
   recover. Domain transfer/expiry becomes a detected event, not a silent
   inheritance.

5. **Issuer-under-verified-domain constraint.** A tenant's registered
   `issuer` URL MUST be a host at or under the verified name, and all JWKS
   fetches use TLS with RFC 6125/9110 host verification. Enrollment proves
   control once by DNS; every subsequent key fetch re-proves it by WebPKI.
   The daily TXT check covers the ownership-change case both anchors miss.

## Alternatives considered

| Alternative | Verdict |
|---|---|
| PGP-signed TXT string | Rejected — see decision 1: wrong tool; bootstrapping circularity; size; no added property over zone control + unguessable token. |
| Random nonce stored per enrollment | Acceptable variant; HMAC preferred for statelessness and replica symmetry. Falls back cleanly if `K_enroll` custody is ever unavailable. |
| HTTP well-known file (ACME HTTP-01 style) | Optional future secondary method; weaker fit — issuers are subdomains and DNS proves control at the zone level where issuer hosts are created. |
| Email-to-domain verification | Rejected — proves mailbox access, not zone control; weakest link in practice. |
| Verify once at enrollment only (industry default) | Rejected — leaves the domain-expiry/transfer inheritance hole; the daily re-check is a core requirement of this ADR, not an optimization. |

## Consequences

- (+) Tenant trust has an explicit, auditable birth certificate and an
  ongoing heartbeat; domain loss is detected within a day.
- (+) Stateless token verification; no per-challenge storage; horizontal
  scaling of the verifier is trivial.
- (+) Prior art alignment (ACME DNS-01) keeps tenant-side instructions
  familiar to any operator who has issued a wildcard certificate.
- (−) New custody requirement: `K_enroll` lives in the ADR-006 secrets chain,
  with epoch rotation. NOTE: a leaked `K_enroll` alone does NOT let an
  attacker enroll a domain — they must still publish the record in DNS they
  control; the secret only lets them *predict* tokens (defense in depth).
- (−) Tenants must be able to create TXT records (table stakes for any org
  running its own issuer; same requirement as ACME DNS-01 wildcard issuance).
- (−) A recurring verification service with quorum resolution, scheduling,
  grace-state management and alerting must be built and operated — specified
  in the design document.

## RFC compliance (summary — full matrix in the design doc)

RFC 1035 (TXT sizing), RFC 2104 (HMAC), RFC 4086 (secret entropy),
RFC 4033–4035 (DNSSEC validation), RFC 6125 / RFC 9110 (TLS host
verification of issuer/JWKS), RFC 8484 (DoH), RFC 8552/8553 (underscored
node names, `_sesame-challenge` registration practice), RFC 8555 §8.4
(pattern prior art), RFC 7638 (kid thumbprints, via ADR-006).

## Amendment (2026-10-10): organisation email domains

The same DNS TXT proof also establishes that an **organisation controls an email domain** (`bigbank.com` for users
`@bigbank.com`). Sesame relies on that claim wherever an email address alone would otherwise decide who someone is:

- **SAML** (ADR-014, DESIGN-saml-sp.md §6): first-time linking of an IdP identity to a Sesame user, and just-in-time
  provisioning, happen only for emails in a domain the organisation has verified. Without this, any organisation could
  claim `@bigbank.com` and have its IdP assert those users.
- **"Sign in with SSO"** discovery: an email domain routes to an organisation's SAML connection only if verified.
- **Domain-based auto-join** (organisation settings): only for verified domains.

Decisions for this subject (full design: `docs/DESIGN-org-domain-verification.md`):

1. **Same mechanism**: token, record name and value form, DoH quorum, DNSSEC, constant-time comparison, daily
   re-verification with grace and revocation, `K_enroll` custody: as decided above.
2. **Separate claims**: the token's PRF input carries `purpose = "sesame-org-domain"` and the organisation, so an
   organisation's record can never satisfy a tenant-issuer check or another organisation's, and vice versa.
3. **Exact domain by default**; `*.bigbank.com` emails need an explicit "include subdomains" claim, proven at the
   apex.
4. **Never claimable**: public suffixes (Public Suffix List) and consumer mail providers (`gmail.com`, `outlook.com`,
   …, a maintained list).
5. **One organisation per domain per tenant** while a claim is verified or in grace; tenants remain isolated, so two
   tenants may each have an organisation with the same verified domain.
6. **Stricter grace than issuers**: during grace, existing links keep working but nothing new is bound to the domain
   (no first-time linking, no provisioning, no auto-join); on revocation, discovery and auto-join stop for it.
   Existing SAML identities, which are bound to the connection and NameID rather than to DNS, keep working; the
   organisation's admins are alerted at every transition.

## Amendment (2026-10-11): DNS resolution runs in its own service, `dns-probe`

Every DNS-over-HTTPS lookup for domain verification (organisation and, later, tenant issuer) is made by a small
internal service, **`dns-probe`**, not by the components that hold the enrolment key and database credentials.

1. **What it does**: given a name and record type (TXT at `_sesame-challenge.<domain>`), it queries the configured
   DoH providers (two of Cloudflare, Google, Quad9; distinct operators) and returns what each answered: response code,
   TXT strings joined per record, CNAME chain followed, AD bit, TTL and negative-cache TTL. Nothing else.
2. **What it does not do**: it holds no secret, no database or Redis access, and never sees `K_enroll` or a token.
   Matching the token and the quorum verdict (both providers must return the record; DNSSEC must validate where the
   zone is signed) stay in the caller, in `sesame_common::domain_verify`, as pure functions over the probe's answers.
3. **Who calls it**: org-mgmt (first verification) and the re-verification job. Neither of them needs internet egress
   for this any more.
4. **Network**: ingress only from org-mgmt and the re-verification job; egress only to the configured providers'
   published DoH addresses on 443 (fixed IPs: `1.1.1.1`/`1.0.0.1`, `8.8.8.8`/`8.8.4.4`, `9.9.9.9`/`149.112.112.112`
   and their IPv6 equivalents), so the policy is a short list of address blocks, not a hostname allow-list. No cluster
   DNS needed: providers are addressed by IP with TLS names pinned in configuration.
5. **It talks only to trusted, fixed endpoints over TLS.** It deliberately does not fetch anything a user supplies
   (IdP metadata URLs stay with org-mgmt, DESIGN-saml-sp.md §7, §10): a process whose answers decide domain ownership
   must not also talk to attacker-chosen servers.
6. **Cost**: one more stateless Deployment (one small replica in pp, two in production), next to `saml-verifier`.

Rejected: resolution as a library inside org-mgmt and the job (internet egress in the two components that hold
`K_enroll` and database credentials); resolution inside `saml-verifier` (its isolation depends on having no egress at
all).
