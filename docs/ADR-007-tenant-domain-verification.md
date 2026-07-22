# ADR-007: Tenant domain-control verification (DNS TXT challenge with continuous re-verification)

Status: PROPOSED (2026-07-22)
Related: ADR-004 (platform-tenant provisioning), ADR-006 (shared signing keys),
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
