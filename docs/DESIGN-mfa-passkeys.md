# DESIGN: two-step sign-in and passkeys (ADR-008, phase A)

Status: IMPLEMENTED (October 2026). Decision record: [ADR-008](ADR-008-authentication-assurance-passkey-first-mfa.md).
Follow-ups: [DESIGN-org-sso.md](DESIGN-org-sso.md), [DESIGN-account-sharing-controls.md](DESIGN-account-sharing-controls.md).

This is what Sesame does today for authenticator apps (TOTP), passkeys (WebAuthn), recovery codes and organisation
sign-in policy, and why. §7 lists what is left for Charles to decide.

## 1. What was there before

The "groundwork" was stubs. `setup_user_mfa_totp` returned the literal secret `TOTP_SECRET_PLACEHOLDER`,
`verify_user_mfa` answered `mfa_verified: true` without checking anything, and `disable_user_mfa` did nothing. None of
the three was wired into the running service (each answered 501), so nothing was exposed, but nothing worked either.
`auth_login` hard-coded `mfa_required: false`, and no token carried `amr`, `acr` or `auth_time`. The two stubs are
deleted; `disable_user_mfa` is now the support-mediated reset (§5).

## 2. The gate

Every first factor (password, emailed code, magic link, SMS code, social provider) ends in one function,
`services::mfa::after_first_factor`, and every later step in `services::mfa::next_step`. Both call the same pure
decision (`services::mfa::decide`, unit-tested):

| The account | Its organisation policy | After a first factor |
|---|---|---|
| has no factor | none | 200, tokens (`amr: ["pwd"]`, `acr: aal1`) |
| has TOTP and/or a passkey | any | 202, `mfa_token`, `methods` it may use |
| has no factor | `mfa` or `passkey` | 202, `enrollment_required` |
| has TOTP only | `passkey` | 202 (TOTP), then 202 `enrollment_required: passkey` |
| used a recovery code | `passkey` | 202 `enrollment_required: passkey` (ADR-008 §6) |

A passkey sign-in (username-less or as the second step) is phishing-resistant MFA on its own and satisfies every
policy. **No token exists until the gate says so**: a stolen password never yields one for an account with a second
factor, and no first factor (an emailed code included) bypasses the second.

The 202 body (`MfaChallengeResponse`):

```json
{ "mfa_required": true, "mfa_token": "…", "expires_in": 600,
  "methods": ["passkey", "totp", "recovery_code"], "enrollment_required": null }
```

The `mfa_token` is an opaque 256-bit value; Redis holds only its SHA-256 (`mfa:ctx:*`) with what the sign-in has
proved so far, for 10 minutes. Completing it deletes it; of two concurrent completions only the one that deletes it
issues tokens.

### Endpoints (identity-login-service, `/idam/v1`)

| Endpoint | Auth | Purpose |
|---|---|---|
| `POST /auth/login` (and the email-code, magic-link, SMS and social verifies) | first factor | 200 tokens or 202 |
| `POST /auth/mfa/verify` | `mfa_token` | TOTP or recovery code |
| `POST /auth/passkeys/challenge` | `client_id` or `mfa_token` | WebAuthn request options (username-less or second factor) |
| `POST /auth/passkeys/verify` | the assertion | finish a passkey sign-in |
| `POST /auth/mfa/manage` | Bearer, recent sign-in | open a 15-minute security session |
| `POST /auth/mfa/factors`, `…/remove`, `…/rename` | security session | list, remove, rename |
| `POST /auth/mfa/recovery-codes` | security session | new set (old set stops working) |
| `POST /auth/mfa/totp/enroll`, `…/confirm` | security session or enrolment | add an authenticator app |
| `POST /auth/passkeys/register/options`, `…/register` | security session or enrolment | add a passkey |

org-mgmt: `GET/PUT /organizations/{org_id}/auth-policy` (members read; owners and admins set; audited).
identity-user-mgmt-service: `POST /admin/users/{user_id}/mfa/disable` (support-mediated reset, §5).

### Why factor management is not a bearer API

Changing how an account signs in is what a stolen session wants most. So the bearer token is exchanged once, at
`/auth/mfa/manage`, for a 15-minute security session, and only when the sign-in is recent (`auth_time` within 10
minutes, `login_required` otherwise) and, for an account that already has a factor, used one (`mfa_required`
otherwise). An older session, or one from before the account had a factor, can never remove a factor or add an
attacker's. The hosted security page always signs the user in again first.

## 3. Truthful claims (ADR-008 §3)

`sesame_common::authn` computes `amr`, `acr` and `auth_time` from the methods used, and every token path carries them:

* access tokens at sign-in, set-active-organization (unchanged from the presented token) and registration;
* the authorization code (`AuthorizationCode.authn`) to the relying party's access and ID tokens;
* refresh: the refresh JWT carries the three claims and each rotation copies them into its successor and the new
  access token, so a refreshed token reports the original ceremony and its original `auth_time`, never a fresher or
  stronger one. The `rfam:*` session record and the rotation's two round trips are unchanged.

| Ceremony | `amr` | `acr` |
|---|---|---|
| password | `pwd` | `urn:sesame:acr:aal1` |
| emailed code or magic link | `email` | `aal1` |
| password + emailed code | `email pwd` | `aal1` (never `mfa`) |
| password + TOTP | `mfa otp pwd` | `urn:sesame:acr:aal2` |
| password + recovery code | `mfa pwd rc` | `aal2` |
| passkey (synced, verified) | `mfa swk user` | `urn:sesame:acr:aal2-phishing-resistant` |
| passkey (device-bound, verified) | `hwk mfa user` | `aal2-phishing-resistant` |
| SMS code | `sms` | `aal1` |
| social provider | `fed` | `aal1` |

`email`, `rc` and `fed` are Sesame values (RFC 8176 has none for them), deliberately distinct so a resource can never
mistake an emailed code for TOTP. An emailed code counts in the password's factor category: whoever holds the mailbox
can reset the password, so it is never what makes a ceremony `mfa`.

## 4. The pieces

### TOTP

RFC 6238 (HMAC-SHA1, 30 s, 6 digits), current step ± 1. A code is accepted once: the step is recorded with an
`UPDATE … WHERE totp_last_step < $step`, so the replay check and the record are one statement. One authenticator app
per account. The 160-bit secret is sealed with AES-256-GCM under a key derived (HKDF-SHA256, label
`totp-secret/v1`) from the deployment KEK — `MFA_SECRET_KEK`, or the existing custody KEK `SMS_CREDENTIAL_KEK`
(ADR-009) when no dedicated one is set — with the tenant, user and factor id as associated data. A database copy is
useless without the KEK, and a sealed secret moved to another row does not open. The QR code is drawn in the browser
from the `otpauth://` URI; the secret is shown once, during enrolment.

### Passkeys

webauthn-rs 0.5.5 (OpenSSL, vendored and linked statically, so no image change). User verification required;
discoverable credential preferred (`residentKey: preferred`, so a passkey can sign in without a username; every
platform authenticator makes one anyway); no attestation; existing passkeys excluded at registration. The signature
counter and backup flags are updated after each use, and webauthn-rs refuses a counter that went backwards. The user
handle is the account id (a random UUID, no personal data). Credentials are stored in Postgres
(`user_mfa_factors.passkey`, public material only), unique per tenant by credential id. Ceremony state lives in
Redis for 5 minutes and is taken once (`GETDEL`).

Username-less sign-in: `/auth/passkeys/challenge` with the client id, then the browser offers the user's passkeys
either as a button ("Sign in with a passkey") or as autofill on the email field (conditional mediation, where the
browser supports it). The response names its credential and user handle; Sesame checks that the credential is an
active passkey of that user in the client's tenant before verifying the signature.

### Relying party ID (ADR-008 §5): `pp.pricewhisperer.ai`

A passkey is bound to an RP ID, a registrable domain the browser compares with the origin of every ceremony. For
PriceWhisperer pre-production the hosts are:

* `sesame-idam.pp.pricewhisperer.ai` — Sesame's hosted sign-in and security page (where the ceremonies run);
* `trader.pp.pricewhisperer.ai` — the trader app (its BFF is a confidential OIDC client; it never runs a ceremony).

Both are under `pp.pricewhisperer.ai`, so the RP ID is **`pp.pricewhisperer.ai`**:

* the ceremonies on the Sesame host are valid (the origin is a subdomain of the RP ID);
* pp and production passkeys never mix: a browser offers a credential only for its exact RP ID, and production's will
  be `pricewhisperer.ai`, so a pp sign-in (RP ID `pp.pricewhisperer.ai`) never sees production passkeys and the
  other way round;
* the trader app (or a future `auth.pp.pricewhisperer.ai`) could run ceremonies itself later without re-enrolling
  anyone;
* **no WebAuthn Related Origin Requests are needed**: ROR is only for origins on a different registrable domain from
  the RP ID (for example a hosted login at `auth.sesameidentity.com` serving `pricewhisperer.ai` passkeys). It would
  also exclude Firefox, which does not support it.

Choosing `sesame-idam.pp.pricewhisperer.ai` itself would have worked today but tied every passkey to that exact
host. Choosing `pricewhisperer.ai` in pp would have mixed pp and prod passkeys in users' password managers.

Configuration: `SESAME_WEBAUTHN_RP_ID`, `SESAME_WEBAUTHN_ORIGINS` (comma-separated; every origin must be the RP ID or
under it, checked at start-up) and `SESAME_WEBAUTHN_RP_NAME` on identity-login-service. Unset, the RP ID is the host of
`SESAME_AUTH_PUBLIC_URL`. One RP per deployment: each Sesame deployment serves one product domain today. A
multi-product Sesame (`auth.sesameidentity.com`) needs per-tenant RP IDs from ADR-007 verified domains and, for
tenants whose domain is not the hosted login's, ROR (a `/.well-known/webauthn` file on the tenant's domain listing the
Sesame origin) — a follow-up, not needed for PriceWhisperer.

### Recovery codes

Ten codes, 16 Crockford base32 characters each (80 random bits, `xxxx-xxxx-xxxx-xxxx`), generated when an account
gets its first factor and on request; stored as salted SHA-256 (80 bits make guessing pointless without a slow hash);
each works once; regenerating invalidates the old set; removing the last factor removes them. The hosted page makes
the user confirm they saved them.

### Abuse controls

Every second-factor endpoint shares one lockout identity per account (`mfa:{user_id}` in the existing
`abuse_guard`: 5 failures, then 60 s doubling to an hour), so guessing TOTP codes and recovery codes is bounded per
account whatever the route. An `mfa_token` burns after five wrong attempts. Pending enrolments never satisfy a
challenge.

## 5. Recovery (ADR-008 §6)

Recovery never silently downgrades assurance:

* **Lost authenticator, has recovery codes**: password + recovery code signs in at `aal2` (never phishing-resistant).
  Under a passkey policy the user must enrol a new passkey before any token is issued.
* **Lost password**: the reset link resets the password only; the second factor still applies at the next sign-in.
* **Lost everything**: a tenant administrator resets the user's factors (`POST /admin/users/{id}/mfa/disable`): their
  own tenant only, never their own account, only users whose administrator roles they could grant. The user signs in
  with their password and, if their organisation requires it, enrols again at that sign-in.
* **Every event is audited and emailed** to the account holder: factor added or removed, recovery codes generated,
  a recovery code used to sign in, an administrator reset. A new audit event type, `authenticator_changed`, is a
  security event (WARN).

## 6. Organisation policy (minimal)

`org_auth_policies.require_method` is `none` (default), `mfa` or `passkey`, set by an organisation's owners and
admins. The strictest policy of the user's active memberships applies. Retail users (no organisation, or an
organisation without a policy) are not affected. A member without the required method is made to enrol at their next
sign-in; the enrolment sign-in's tokens say what it proved (a password: `aal1`), and every later sign-in uses the new
factor. A member cannot remove the last method their policy needs (`409 policy_requires_factor`).

## 7. Decisions for Charles

1. **Enrolment race.** A member of an organisation that turns on `mfa` who has not yet enrolled can be enrolled by
   whoever has their password (the attacker adds their own authenticator). The user is emailed at once. Closing it
   needs proof beyond the password before a forced enrolment, most cheaply an emailed code. Recommended for
   organisations that set `passkey`; optional for `mfa`.
2. **A dedicated KEK.** TOTP secrets are sealed under `SMS_CREDENTIAL_KEK` (domain-separated by HKDF). A dedicated
   `MFA_SECRET_KEK` in Secret Manager (`scripts/sesame-secrets.sh`, gcp-infrastructure `sesame-secrets`) would let the
   two be rotated apart. Rotation of either is not yet automated (it would re-seal every secret).
3. **Step-up (RFC 9470).** Resources can already read `acr`/`amr`/`auth_time`. Accepting `acr_values` and `max_age` at
   `/oauth/authorize` (and answering with the stronger ceremony) is the next piece; nothing in PriceWhisperer needs it
   yet.
4. **Retail default.** The tenant default is optional for everyone. A tenant-wide "require a second factor for
   accounts with a paid plan" switch is a small addition to the policy lookup if wanted.
5. **SMS as a second factor.** Not offered (NIST "restricted", ADR-008 §1). The `sms` method exists for sign-in only
   where a tenant opts in.

## 8. Verification

* Unit: `sesame_common::authn` (claim honesty), `services::mfa` (gate decisions, TOTP RFC 6238 vectors, sealing,
  recovery-code format, RP ID validation, context serialisation), `org-mgmt services::auth_policy`.
* Frontend: `src/lib/webauthn.test.mjs` (encoding both ways).
* pp: test users only — password + TOTP, recovery code, passkey ceremonies driven by a virtual authenticator, policy
  enforcement and admin reset (see the release notes of the version that shipped this).
