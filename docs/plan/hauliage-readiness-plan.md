# Hauliage Readiness Plan — Tasks & Stories

Date: 2026-07-06
Status: draft
Sources: repo audit (this session), hauliage repo audit, `docs/llmwiki/topics/topic-remediation-plan.md`, `docs/Epics/INDEX.md`

## Goal

Get sesame-idam to a state where the hauliage project can use it as its identity
provider: real login issuing real Ed25519-signed JWTs with the claims hauliage
needs, JWKS validation across hauliage's ~16 services, refresh, basic user/org
management, and a deployable stack reachable from the hauliage Kind cluster.

## Where we are

**Sesame-idam (verified by audit):**

- Workspace compiles; 127 handlers exist across 6 services, but **zero handlers
  query Postgres**. ~42% are explicit brrtrouter stubs; most of the rest echo
  request fields plus audit logging.
- `auth_login` does not verify credentials. `auth_register` returns
  `access_{uuid}` fake tokens. `auth_token.rs` (~1,900 lines, 51 unit tests)
  signs with a literal `placeholder_signature`.
- authz-core `principal_effective` returns empty roles/permissions;
  `authorize` always returns `allowed: true`. api-keys `validate_api_key`
  echoes the caller-supplied `valid` field.
- **Real and working:** identity-session-service JWKS endpoint + `KeyManager`
  (Ed25519, rotation, revocation, 100 tests); Redis-backed refresh-token
  rotation; `JwksBearerProvider` validation wired into all consumer services;
  audit crate; common crate (jwt, denylist, token_versioning, fallback_cache,
  entitlement_cache — ~300+ unit tests, several modules ahead of service
  integration).
- 39 lifeguard entity models exist but are not wired to any handler. No
  committed migrations or seeds. `services/` layer is empty doc-comments.
  `db_integration_suite` referenced by `just nt-db-suite` does not exist.
- Remediation plan phases 2.2/2.3 (service.yaml, services layer), 3 (seeds,
  BDD skeleton), 4 (workspace cleanup), 5 (Tilt validation + DB/Redis wiring)
  remain open. Helm deployment template injects no DB/Redis env vars.
- Large uncommitted refactor in flight (config consolidation into common,
  jwt/jwks_cache/fallback_cache module splits).

**Hauliage (verified by audit):**

- Own `identity` service is an empty stub (`login_user.rs` returns all-`None`).
  Org/user context comes from `HAULIAGE_ORGANIZATION_ID` / `HAULIAGE_USER_ID`
  env vars. Frontend `AuthGuard` is hardcoded open; sign-in forms are mocked.
- Every service already registers BRRTRouter security providers and can take a
  `JwksBearerProvider` via `config.yaml` — the validation plumbing exists.
- Frontend E2E fixtures already mock the **sesame-idam identity-login-service
  `TokenResponse`** shape (`access_token`, `token_type`, `expires_in`,
  `refresh_token`, `refresh_token_expires_in`, `user_id`, `mfa_required`,
  `scope`). This is the de-facto integration contract.
- Roles required: `OWNER | DISPATCHER | FLEET_MANAGER | DRIVER | VIEWER`.
  Two org personas (shipper vs haulier). No tenancy today — hauliage would be
  tenant `hauliage` with applications (hauliage-web, hauliage-mobile, ...).
- Deploys on Kind (`hauliage` namespace, shared `data` namespace for
  postgres/redis). No sesame-idam ports/URLs configured anywhere yet.

## Epic H1 — Persistence foundation (blocks everything)

| Story | Description | Notes |
|-------|-------------|-------|
| H1.1 ✅ | Land the in-flight refactor (config→common, jwt/jwks_cache/fallback_cache splits); `just nt` + `just lint-rust` green | **Done 2026-07-06** — 4 compile errors fixed, 852/852 tests pass, lint gate repaired (was referencing deleted `sesame-audit`) and passes; `sesame-common` pedantic backlog split to `just lint-common` (Phase 1 warn) |
| H1.2 🔶 | Wire `services/` layer: controllers call services, never DB directly | **Started** — hauliage-pattern services layer live in identity-login-service (`password`, `user_service`, `token_issuer`); remaining services still stubs |
| H1.3 ✅ | Generate, review migrations from lifeguard entities; `apply_order.txt`; apply to cluster | **Done 2026-07-06** — migrator path bug fixed (wrote outside repo), migrations + `apply_order.txt` in `./migrations/`, applied to Kind postgres; users table has `UNIQUE(tenant_id, email)` |
| H1.4 ✅ | Seed data: hauliage tenant demo users, roles, orgs | **Done 2026-07-09** — demo users + role assignments + org/membership seeds applied (`20260706000002_hauliage_demo_orgs.sql`: Transport Services `b2000001-…`, AME Corp `b2000002-…`, memberships for all five demo users). Login JWTs now carry `org_id` per persona |
| H1.5 🔶 | [RLS bridge delivery](./hauliage-readiness-tasks/README.md#track-b--transaction-local-rls-h15): validated claims → transaction-local context via `SesameExecutor`, versioned helper SQL, reference policies, and zero-bleed proof | **In progress 2026-07-14.** Tenancy is a launch guarantee; session-scoped GUCs are explicitly rejected. |
| H1.6 | Create the `db_integration_suite` test target so `just nt-db-suite` actually runs against Kind postgres | Profile exists in nextest.toml; binary missing. (Interim: login auth-flow BDD tests hit live postgres and skip gracefully when absent.) |

## Epic H2 — Real authentication (identity-login-service)

| Story | Description | Notes |
|-------|-------------|-------|
| H2.1 ✅ | Password hashing (argon2id) + credential verification in `auth_login`; user lookup by tenant+email; audit on failure | **Done 2026-07-06** — single indistinguishable 401 for unknown user / wrong password / disabled account (no enumeration). Lockout counters still TODO |
| H2.2 ✅ | Token signing architecture decided + implemented: **shared signing key** — `Ed25519Signer` in `sesame_common::jwt::signer`, key material via `SESAME_JWT_SIGNING_KEY_PKCS8_B64`/`SESAME_JWT_SIGNING_KID` env (K8s Secret); session-service `KeyManager` bootstraps from the same env so JWKS stays the single source of truth | **Done 2026-07-06** for the login path. `auth_token.rs` (token exchange) still uses placeholder signing — migrate when that endpoint is implemented for real |
| H2.3 ✅ | `auth_register`: real user INSERT (tenant-scoped, argon2id hash), email uniqueness per tenant (pre-check + DB constraint), real token issuance, 201/400 | **Done 2026-07-06** |
| H2.4 ✅ | Login/register store refresh tokens in Redis (`refresh:{jti}` + `family:{sid}`) compatible with session-service rotation; rotation now prefers the token's own `family_id` | **Done 2026-07-06** |
| H2.5 🔶 | `TokenResponse` conformance test against the hauliage E2E fixture shape | **Partially** — BDD asserts `token_type`/`expires_in`/`refresh_token`/`user_id` and JWT claims; a literal fixture-diff test still worth adding |
| H2.6 | Password reset flow (`auth_reset_password`) backed by DB + email-verification entity | Can be P2 if hauliage launch tolerates admin resets |

## Epic H3 — Claims & authorization (authz-core + Epic 2 subset)

| Story | Description | Notes |
|-------|-------------|-------|
| H3.1 ✅ | authz-core call pattern decided: **login-time enrichment only** for hauliage v1 (JWT-first; hybrid per Epic 4 later) | Implemented as best-effort: login degrades to empty roles when authz-core is unavailable |
| H3.2 ✅ | Claims schema v1: `sub`, `tenant_id`, `roles[]`, `sid`, `ver`, `typ=at+jwt` under the namespaced claims URI | **Done 2026-07-09** — login-issued tokens carry all of these including `org_id` (resolved from org-membership seeds); verified live on ms02 for both demo personas |
| H3.3 🔶 | `principal_effective` real implementation | **Done for roles + attributes 2026-07-06** — queries `role_assignments`/`principal_attributes` tenant-scoped (live-DB BDD incl. tenant isolation). Permissions (role→permission mapping in org-mgmt tables) + Redis caching still pending |
| H3.4 | `authorize` real decision evaluation (role→permission mapping); remove always-allow | EXTREME-traffic service; needs the Epic 7 caches eventually, simple DB+Redis for v1 |
| H3.5 ✅ | Wire login → authz-core `/principal/effective` via `may_http` | **Done 2026-07-06**, fixed 2026-07-09 — `services/authz_client.rs` (500ms timeout, AUTHZ_CORE_URL env), roles land in TokenResponse + sx claims; BDD with mock authz-core + graceful-degradation test. Demo role seed: OWNER/DISPATCHER/DRIVER for the hauliage users. 2026-07-09 fixes: client was missing the `/idam/v1` base path (404), and `principal_effective` emitted `"org_id": null` for tenant-scoped assignments which failed response schema validation — endpoint also marked `security: []` (S2S, called before any user token exists) |

## Epic H4 — Users & orgs (minimum viable surface)

| Story | Description | Notes |
|-------|-------------|-------|
| H4.1 🔶 | Current-user profile — DB-backed | **`GET`/`PATCH /identity/me` done 2026-07-06** (lives in identity-session-service per the spec, not user-mgmt): raw handlers reading the validated JWT principal (typed dispatch drops `jwt_claims`), tenant cross-check header↔claims, `users` + `user_profiles` upsert, 6 live-DB BDD tests. Remaining: user-mgmt admin CRUD (get/update/disable user) |
| H4.2 | org-mgmt core CRUD: org create/fetch/update, memberships, role assignment | 35/42 handlers stubbed; scope to what hauliage needs |
| H4.3 | **Boundary decision doc: org-mgmt vs hauliage `company` service.** Hauliage company owns KYC/escrow/GICS; IDAM owns identity-org + roles. Define the ID mapping (IDAM org_id ↔ hauliage organization UUID) and which system is source of truth for membership | Cross-repo decision; blocks H4.2 scoping |
| H4.4 | org invites flow (create/accept) if hauliage team-invite feature is in launch scope | Verify with hauliage roadmap; else P2 |

## Epic H5 — API keys (M2M)

| Story | Description | Notes |
|-------|-------------|-------|
| H5.1 | api-keys: create/list/delete backed by DB with hashed key storage | Currently echo stubs |
| H5.2 | `validate_api_key` real hash lookup + tenant scoping; contract test against BRRTRouter `RemoteApiKeyProvider` so hauliage workers (email_reminder_worker, iot_worker) can use it | Passthrough today |

Priority: P2 — hauliage workers currently use static `test123` keys; needed
before production but not for first integrated login.

## Epic H6 — Deployment & reachability

| Story | Description | Notes |
|-------|-------------|-------|
| H6.1 | Complete remediation Phase 5: `tilt trigger` validation per service, DB secrets/configmaps, Redis wiring, live_update verification | Wiki phase open since May |
| H6.2 ✅ | Helm `deployment.yaml`: inject `DB_*`/`REDIS_URL` env | **Done 2026-07-06** — DB env from values + `sesame-idam-db-credentials` Secret (optional, dev fallback), `REDIS_URL`, `AUTHZ_CORE_URL` (login), shared JWT signing key from `sesame-idam-jwt-signing` Secret (created via `just jwt-signing-secret`, applied to the cluster). Values files fixed: app DB is `sesame_idam`, not the postgres superuser DB |
| H6.3 | Decide + implement cluster topology: sesame-idam namespace on the shared Kind cluster so hauliage pods reach `identity-session-service.sesame-idam.svc.cluster.local:8105` (JWKS) and login at 8101 | Hauliage has zero idam URLs configured today |
| H6.4 | `config/service.yaml` per impl crate (remediation Phase 2.2) | CORS/security/http/db-pool config |
| H6.5 | OIDC discovery (`openid_configuration`) returns real issuer/JWKS URLs | Currently all-`None`; cheap and unblocks standard clients |

## Epic H7 — End-to-end validation

| Story | Description | Notes |
|-------|-------------|-------|
| H7.1 | Full-flow E2E (BDD, live HTTP + Kind postgres/redis): register → login → validate JWT via JWKS → `/users/me` → authz check → refresh → logout/revoke | Nothing like this exists; current BDD is schema/in-process only |
| H7.2 | Cross-repo smoke: hauliage service configured with sesame-idam JWKS URL accepts a sesame-issued token; hauliage E2E fixture replaced by real login against idam | The real acceptance test for "ready for hauliage" |
| H7.3 | Token revocation path: denylist + version-bump verified end-to-end (Epic 5 subset needed for logout) | denylist middleware exists in authz-core; login-side wiring missing |
| H7.4 | RLS zero-bleed E2E: validated Hauliage token → `SesameExecutor` → unqualified query, including pooled reuse, rollback, missing context, and cross-tenant attempts | Completion evidence for H1.5; see the [delivery task list](./hauliage-readiness-tasks/README.md) |

## Track B — Hauliage-side tasks (hauliage repo, tracked here for visibility)

1. Configure `JwksBearerProvider` (jwks_url → sesame-idam session-service) in
   all service `config.yaml`s; remove `test123` static keys and mock
   `BearerJwtProvider`.
2. Retire the stub `identity` microservice; proxy `/api/v1/identity/*` through
   BFF (or Vite proxy in dev) to sesame-idam.
3. Replace `HAULIAGE_ORGANIZATION_ID`/`HAULIAGE_USER_ID` env hacks with claims
   extracted from the validated JWT (org_resolution.rs rewrite).
4. Frontend: wire sign-in forms to `AuthContext.signIn()`, implement real
   `AuthGuard`, add refresh-token handling, propagate `X-Tenant-ID`.
5. Adopt `X-Tenant-ID: hauliage` injection at BFF/gateway.

## Sequencing & priorities

```
P0 (foundation):        H1.1 → H1.2/H1.3 → H1.4/H1.5/H1.6, H6.1/H6.2
P1 (first login):       H2.1/H2.2 → H2.3/H2.4/H2.5, H3.1 → H3.2/H3.3/H3.5, H4.1, H6.3/H6.4/H6.5
P1 gate:                H7.1 green
P2 (integration):       H3.4, H4.2/H4.3, H5.*, H7.2/H7.3, Track B
P3 (post-launch):       H2.6, H4.4, hybrid authz (Epic 4), DPoP (Epic 8), MFA/SSO/SCIM
```

Explicitly deferred for hauliage v1: social OAuth, OTP/passwordless, MFA
enforcement, SCIM, enterprise SSO, webhooks, impersonation/delegation
(Epic 6), hybrid fallback caching (Epic 7) — the library code in `common` for
several of these already exists and can be integrated later.

## Key decisions needed before implementation

1. **Signing locus** (H2.2): where does login get its Ed25519 signing key —
   shared `common` module vs internal mint call to session-service.
2. **authz-core call pattern** (H3.1): login-only enrichment vs per-request.
3. **Org boundary** (H4.3): IDAM org-mgmt vs hauliage `company` service split
   and ID mapping.
4. **Cluster topology** (H6.3): sesame-idam namespace on shared Kind cluster
   vs separate cluster.
