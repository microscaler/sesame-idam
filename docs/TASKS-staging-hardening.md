# TASKS: Staging hardening → safe public exposure for pentest

Source of truth for the pre-exposure work identified in
`LAUNCH-READINESS-staging-pentest.md`. Ordered into gates. Each task has an
owner-layer, acceptance criteria, and a status box. Work top-down; a gate is
not "done" until every MUST in it is checked.

Status legend: `[ ]` todo · `[~]` in progress · `[x]` done · `[-]` deferred (with reason)

Layers: **GW** = Envoy / Gateway API (HTTPRoute) · **APP** = service code ·
**INFRA** = cluster/GCP/Helm · **OBS** = observability · **SEC** = secrets ·
**PROC** = process/rehearsal

---

## GATE A — Perimeter controls (exposure blockers; nothing goes public until all MUST are done)

### A1 [ ] Volumetric rate limiting — GW (MUST)
Envoy / Gateway API `HTTPRoute` + rate-limit policy (or Cloud Armor) on all
externally-reachable auth paths.
- Per-route limits: `/auth/login`, `/auth/*otp*` (send AND verify separately),
  `/auth/refresh`, `/auth/register`, `/oauth/*`, `/.well-known/jwks.json`.
- Key by client IP AND by target path; stricter on OTP-send than on reads.
- Acceptance: scripted burst (e.g. 100 req/s to `/auth/login`) returns 429
  after threshold; JWKS scrape at high rate throttles; legit single-user flow
  never trips.
- Note: this is VOLUMETRIC only. Per-identity lockout is A2 (stateful, not GW).

### A2 [x] Account lockout / progressive backoff — APP + shared store (MUST)
> DONE 2026-07-24: `services/abuse_guard.rs` (Redis; SHA-256-hashed identity
> keys, sliding decay, base×2^n backoff capped at LOCKOUT_MAX_SECS, audit on
> lock). Wired into `auth_login` for ALL failure paths including unknown
> identifiers (no enumeration via lockout behaviour); locked response is
> byte-identical to wrong-password. BDD: `tests/bdd/account_lockout.rs`
> (9/9 green incl. decay + no-oracle). OTP-verify wiring joins when the OTP
> verify flows are implemented (they are gen stubs today) — the guard API
> (`record_login_failure`) is shared and ready.
Per-identity failed-attempt tracking (login + OTP verify). Gateway cannot do
this (stateless, no identity view).
- Counter keyed on (tenant, username/identifier) in Redis (or DB); progressive
  delay + temporary lock after N failures; auto-decay.
- Emit an audit event on lock; surface a generic error (no user-enumeration
  signal — same response for locked vs wrong-password).
- Acceptance: 6 bad passwords → locked with backoff; correct password during
  lock still denied; lock decays; no timing/message oracle for account
  existence.

### A3 [x] OTP abuse & toll-fraud controls — APP (MUST)
> DONE 2026-07-24 (guard layer): `abuse_guard::gate_otp_send` — per-recipient
> window + daily caps, rapid-resend dedupe, tenant SMS opt-in (ADR-008
> interim: `SMS_OPTED_IN_TENANTS` env until a tenants column lands), global
> daily SMS spend ceiling (fail-CLOSED for SMS on Redis outage). Enforced in
> newly wired controllers: login_email_otp, login_phone_otp, magic_link_send,
> sms_magic_link_send (email OTP + magic link share one channel budget; both
> SMS endpoints share the meter). Responses stay generic on suppression —
> denials go to the audit log. BDD: `tests/bdd/otp_caps.rs`.
> UPDATE 2026-07-24: the EMAIL side is now REAL — SMTP provider
> (`services/email.rs`, default target the Mailpit test endpoint in the
> `data` namespace), email OTP send/verify + magic link send/verify
> (hashed-in-Redis, TTL'd, attempt-capped, single-use), and the caps are
> proven at the delivery boundary by `tests/bdd/email_round_trip.rs`
> (capped sends never reach the mailbox; unknown accounts get identical
> responses and zero mail). SMS provider remains unbuilt — SMS caps still
> precede the capability.
Per-recipient send caps and provider cost ceilings for email/SMS OTP and
magic links.
- Max sends per recipient per window; global daily SMS spend ceiling; dedupe
  rapid re-sends; SMS restricted to opted-in tenants (ties to ADR-008).
- Acceptance: repeated OTP-send to one number/email caps out; SMS spend cannot
  exceed configured ceiling; email flood to one mailbox is bounded.

### A4 [ ] TLS everywhere + HSTS, no plaintext auth path externally — GW/INFRA (MUST)
- TLS terminate at ingress; HSTS with sane max-age; redirect 80→443; verify no
  auth endpoint is reachable over plaintext from outside.
- In-cluster JWKS fetch remains a known plaintext hop (BR-1c) — acceptable
  inside the mesh for staging; do NOT expose it externally unauthenticated
  beyond the read-only JWKS doc.
- Acceptance: SSL Labs-style scan clean; `curl http://` to any auth path
  refuses/redirects.

### A5 [x] CORS locked to staging origin only — APP config (MUST)
> DONE 2026-07-24: found the deployed impl binaries installed NO CORS
> middleware at all (only the unused gen mains carried the wiring). Shared
> installer added (`sesame_common::cors::build_cors_middleware`), wired into
> all 6 impl mains. Origin policy is pure config: `cors.origins` per service
> (dev defaults = explicit localhost origins, wildcard REMOVED) with the
> per-environment `CORS_ALLOWED_ORIGINS` env override (comma-separated exact
> origins) for staging/prod. Wildcard+credentials still panics at startup.
BRRTRouter CORS is built; just configure. Allow-list the exact staging
frontend origin(s); no wildcard; credentials mode correct.
- Acceptance: cross-origin request from a non-allowed origin is rejected;
  the staging SPA works.

### A6 [x] `iss` + `aud` set on every consumer — APP config (MUST)
> DONE 2026-07-24: expectations are now CONFIG, not code — the hard-coded
> `ALLOWED_ISSUERS`/`EXPECTED_AUDIENCE` constants in sesame_common became
> env-overridable (`JWT_ALLOWED_ISSUERS` / `JWT_EXPECTED_AUDIENCES`, defaults
> preserved + service audiences added); both validation paths (BRRTRouter
> JWKS provider AND the common-path validator) honour them. Every consumer's
> config.yaml sets its OWN service-specific aud (identity-login, authz-core,
> org-mgmt, identity-user-mgmt, api-keys); identity-session-service is the
> issuer/JWKS provider and deliberately has none (documented in its config).
> Issued tokens enumerate their consumers via `JWT_ISSUE_AUDIENCES`
> (env-tunable; issuer via `SESAME_JWT_ISSUER`). Acceptance proven in
> `common/tests/a6_env_expectations.rs`: token minted for service X rejected
> by service Y (aud mismatch); wrong-iss rejected everywhere, including the
> compiled-in default once the environment pins its own list.
Audit all 6 services' `config.yaml`: `iss` matches the issuer, `aud` is the
service-specific audience, neither left default/empty.
- Acceptance: a token minted for service X is rejected by service Y (aud
  mismatch); a token with wrong iss is rejected everywhere.

---

## GATE B — Blast-radius containment (makes "assume breach" safe)

### B1 [ ] Single-replica identity-session-service — INFRA (MUST)
Helm `replicas: 1` for the IdP (ADR-006 in-memory-key constraint). Document
that scaling is blocked until ADR-006 lands.
- Acceptance: exactly one IdP pod; JWKS served consistently.

### B2 [ ] Isolated GCP project / VPC / registry — INFRA (MUST)
Dedicated project, VPC, and registry path for staging. No shared secrets, no
network path to any real data.
- Acceptance: staging cannot resolve/route to prod; separate service accounts;
  separate registry namespace.

### B3 [ ] Secret hygiene for staging — SEC (MUST)
Secrets via external-secrets or sealed-secrets; none baked into images; none in
git plaintext. `K_enroll`/signing keys (where present) from the secret chain.
- Acceptance: `grep` of images and git shows no secret material; pods read
  mounted secrets.

### B4 [ ] NetworkPolicy (pod-to-pod + egress) — INFRA (SHOULD)
Default-deny; explicit allow for required flows; restrict outbound to needed
providers (OTP email/SMS) only.
- Acceptance: a compromised app pod cannot reach the DB of an unrelated
  service or arbitrary internet egress.

### B5 [ ] Disposable identities + reseedable store — APP/INFRA (MUST)
Only synthetic sample users; no real PII; one command to wipe + reseed the
user store.
- Acceptance: `just reseed` (or equiv) returns to a known clean state.

---

## GATE C — Observation & recovery (the point of the exercise)

### C1 [ ] Audit + auth logs shipped off-cluster, structured — OBS (MUST)
Sesame's `sesame_audit` EMITTER + auth-failure logs → Loki/equivalent,
retained beyond pod lifetime, queryable.
- Acceptance: kill a pod, logs from before its death are still queryable; auth
  failures are searchable by identity/path/outcome.

### C2 [ ] Threat-signal alerting — OBS (MUST)
Alerts on: auth-failure spike, lockout-rate spike, JWKS scrape rate, 5xx
burst, OTP-send spike, unexpected egress.
- Acceptance: a simulated credential-stuffing run fires an alert within
  minutes.

### C3 [ ] Rehearsed rotate-and-redeploy reset — PROC (MUST)
Runbook + rehearsal: admin-revoke signing keys (new kid), wipe users,
redeploy, verify logins work with fresh keys. Time it.
- Acceptance: full reset executed from the runbook by someone who didn't write
  it, under (target) 15 minutes, logins green after.

### C4 [ ] Forensics readiness — OBS (SHOULD)
Ensure a compromise is reconstructable: request IDs correlate across services;
token jti/kid logged on validation; who-did-what-when is answerable.
- Acceptance: given a suspicious token, trace its issuance and every use.

---

## GATE D — Graduation to REAL users (NOT required for staging pentest; tracked here so it isn't forgotten)

- [ ] D1 WebAuthn / passkeys — phishing-resistant MFA (ADR-008). **Top
  pre-launch item.**
- [ ] D2 Tenant domain verification ceremony (ADR-007).
- [ ] D3 Shared signing keys / HA (ADR-006) — unblocks multi-replica IdP.
- [ ] D4 Token versioning + denylist / fast revocation (Epic 5).
- [ ] D5 Route-level auth assurance + RFC 9470 step-up (BRRTRouter
  `DESIGN-route-auth-assurance.md`).
- [ ] D6 Clean external pentest report + remediation.
- [ ] D7 Real data protection review (PII handling, retention, DSR).

---

## Working order

Gate A and Gate B in parallel (different layers) → Gate C → rehearse C3 →
seed B5 → expose in a watched window → iterate (observe → harden → redeploy)
until quiet → then Gate D for real-user launch.

## Cross-references

- `LAUNCH-READINESS-staging-pentest.md` — the assessment behind these tasks.
- `ADR-006/007/008` — the Gate D design work.
- BRRTRouter `DESIGN-route-auth-assurance.md` — D5 resource side.
