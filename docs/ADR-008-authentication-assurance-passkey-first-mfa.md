# ADR-008: Authentication assurance — passkey-first MFA, honest factor classification, step-up

Status: PROPOSED (2026-07-22)
Related: ADR-004 (platform tenancy), ADR-006 (shared signing keys),
ADR-007 (tenant domain verification), BRRTRouter
`docs/DESIGN-route-auth-assurance.md` (resource-side counterpart).

## Context

ADR-007 anchors *which issuer* is trusted; this ADR anchors *how much the
issuer's assertion about a human is worth*. Two drivers:

1. **Phishing defense for tenant end-users** — explicitly out of scope of the
   domain-verification mechanism (ADR-007 §10) and unsolvable by any
   server-side key machinery: a user who authenticates to a look-alike origin
   has defeated every JWKS control voluntarily.
2. **Market/compliance pressure for "2FA/3FA with SMS or Email"** — which this
   ADR accommodates *honestly* rather than literally, because the popular
   framing miscounts factors and overrates two weak channels.

### Factor honesty (the analysis this ADR stands on)

Factors are categories — something you **know** / **have** / **are** — and
security comes from *independence and quality*, not count:

- **SMS OTP**: NIST SP 800-63B classifies PSTN/SMS as a RESTRICTED
  authenticator (SIM-swap, SS7 interception are commodity attacks).
- **Email OTP**: usually not an independent factor at all — email typically
  owns account **recovery**, so an attacker with the mailbox both resets the
  password and receives the OTP. Circular.
- password + SMS + email is therefore not "3FA"; it is one knowledge factor
  plus two correlated possession proxies.
- **A passkey (WebAuthn/FIDO2) is genuinely multi-factor in one gesture**
  (possession of the authenticator + the biometric/PIN that unlocks it) and —
  decisively — is **phishing-resistant by construction**: the credential is
  origin-bound at registration and the browser will not exercise it on a
  look-alike domain. The user is removed from the URL-checking loop.

## Decision

### 1. Authentication ladder (normative)

| Rank | Method | Classification | Role |
|---|---|---|---|
| 1 | Passkey (WebAuthn, platform or roaming) | Phishing-resistant MFA (`amr: ["swk","user"]` or `["hwk","user"]`, + `"mfa"`) | Preferred primary for all tenants |
| 2 | TOTP (RFC 6238) + password | MFA, phishable | Supported fallback |
| 3 | Email OTP | Step-up signal ONLY (`amr: ["otp"]` never sole basis for `"mfa"`) | Sensitive-operation friction; never a primary factor |
| 4 | SMS OTP | RESTRICTED (NIST) | Only where a market mandates it; tenant must opt in; flagged in `amr: ["sms"]` |

### 2. Tenant-level policy, enforced at issuance

Each tenant configures, per audience/application:
`required_methods` (allow-list from the ladder), `min_assurance`
(`aal1|aal2|aal2-phishing-resistant`), and step-up rules
(operation → required `acr` + `max_age`). **Enforcement happens at token
issuance**: if the ceremony did not satisfy policy, no token is minted. The
resource side never compensates for a weak ceremony after the fact.

### 3. The token carries the proof (the hinge to BRRTRouter)

Access tokens (RFC 9068 `at+jwt`) MUST include:

- `amr` — RFC 8176 method references actually used (`swk`, `hwk`, `otp`,
  `sms`, `pwd`, `mfa`, …), truthfully;
- `acr` — the assurance class the ceremony satisfied
  (`urn:sesame:acr:aal2-phishing-resistant` etc.);
- `auth_time` — OIDC, when the ceremony happened.

Sesame asserts what happened; resources verify against requirements
(BRRTRouter design note). Sesame MUST never inflate `amr`/`acr` (e.g. email
OTP alone MUST NOT yield `mfa`).

### 4. Step-up, standardized (RFC 9470)

When a resource replies `401` with
`WWW-Authenticate: Bearer error="insufficient_user_authentication",
acr_values="…", max_age=…`, the application redirects to Sesame with those
parameters; Sesame runs the additional ceremony and reissues. No bespoke
protocol.

```mermaid
sequenceDiagram
    autonumber
    participant U as User
    participant App as Tenant app / BFF
    participant RS as Resource (BRRTRouter)
    participant S as Sesame

    U->>App: sensitive action (e.g. change payout account)
    App->>RS: request + access token (amr: pwd,otp; acr: aal1)
    RS-->>App: 401 insufficient_user_authentication\nacr_values=aal2-phishing-resistant, max_age=600
    App->>S: authorize?acr_values=…&max_age=600
    S->>U: passkey ceremony (origin-bound)
    U-->>S: assertion
    S-->>App: new token (amr:+swk,user,mfa; acr: aal2-pr; fresh auth_time)
    App->>RS: retry with new token
    RS-->>App: 200
```

### 5. WebAuthn in multi-tenancy: RP ID = verified tenant domain

Passkey RP ID MUST be the tenant's ADR-007-verified domain (or the issuer
host under it). Credentials are therefore tenant-origin-bound — a loadlinker
passkey is meaningless anywhere else — and the login UX MUST be served on the
tenant's verified origin (or use WebAuthn Related Origin Requests where
supported). This is the deliberate interlock: **ADR-007 verifies the domain;
ADR-008 roots user credentials in it.**

### 6. Recovery is a first-class attack surface

An account is exactly as strong as its weakest recovery path. Normative
minimums: recovery MUST NOT silently downgrade assurance (recovering a
passkey account via email OTP alone re-enters a probationary `acr` until a
new passkey is enrolled); recovery events are audited and notified
out-of-band; tenant admins can require support-mediated recovery for
high-assurance cohorts.

## What BRRTRouter implements (none of it authentication)

See BRRTRouter `docs/DESIGN-route-auth-assurance.md`. Summary: expose
`amr`/`acr`/`auth_time` (already delivered via typed-request claims);
per-route `required_acr`/`required_amr`/`max_auth_age` in the existing route
policy store; RFC 9470 challenge emission. BRRTRouter never performs
ceremonies — it verifies claims about them. That boundary keeps the library
IdP-agnostic and small.

## Consequences

- (+) Phishing against tenant users is defeated structurally (origin-bound
  credentials), not procedurally (user vigilance).
- (+) "2FA/3FA with SMS/email" market asks are satisfiable per tenant policy
  — with truthful `amr` so relying parties are never deceived about assurance.
- (+) Step-up is uniform across all services via one RFC.
- (−) Passkey ceremonies bind login UX to tenant origins (§5) — a real
  constraint on shared login pages; Related Origin Requests partially
  relaxes it.
- (−) OTP channels (email/SMS senders, rate limiting, abuse controls) are new
  operated infrastructure with real abuse surface.
- (−) Recovery ceremonies (§6) require product design, not just engineering.

## Standards

WebAuthn Level 2/3 (W3C), FIDO2/CTAP2, RFC 6238 (TOTP), RFC 8176 (amr),
RFC 9068 (at+jwt), RFC 9470 (step-up challenge), OIDC Core (`acr`,
`auth_time`), NIST SP 800-63B (AAL levels; SMS restriction).
