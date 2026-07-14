# LLM Wiki — Session Log

## [2026-07-14] feat | Hauliage enablement — permissions, org-admin, revocation evidence

- **`principal_effective`:** `app_role_permissions` table + hauliage seed; permissions resolved
  from role assignments and flow into JWT `sx.permissions` via `fetch_effective_authz`.
- **Org-admin:** real `fetch_users_in_org`, `change_user_role_in_org`, `remove_user_from_org`,
  `revoke_pending_invite` + `org_lifecycle` service methods + BDD.
- **Revocation:** `token_lifecycle` BDD asserts `SesameTokenStatusChecker` returns `Revoked` after
  logout denylist write.
- **Tests:** org BDD `seed_user` uses `with_pre_auth_tenant` under forced users RLS.

## [2026-07-14] fix | Hauliage demo logins — pre-auth RLS tenant context + seed order

- **Root cause:** Forced RLS on `sesame_idam.users` filtered all rows when login/register did not set
  `sesame.tenant_id`; `sesame_current_tenant_id()` was also not granted to `sesame_idam`.
- **`with_pre_auth_tenant`:** `microservices/database/src/pre_auth_tenant.rs` — uses Lifeguard
  `with_session_transaction` with nil subject/org placeholders (per-job autocommit would clear GUCs).
- **Migration:** `migrations/rls/20260714180002_pre_auth_tenant_and_grants.sql`.
- **Controllers:** `auth_login`, `auth_register`, `signup_validate`, `social_callback` use pre-auth context.
- **Seeds:** `seed_order.txt` — platform tenant registry before demo users (tenant gate).
- **Tests:** `auth_flow.rs` — shipper + transport demo login BDD.
- **Hauliage wiki:** reseed + RLS troubleshooting in `sesame-idam-brrtrouter-integration.md`.
- **Justfile:** `just tilt-trigger <resource>` wraps `tilt trigger … --port 10351`.

## [2026-07-14] feat | Sesame-IDAM RLS bridge — database context + users policy

- **`sesame_idam_database`:** `rls_context.rs` — `session_context_from_validated_claims` (ported from
  consumer pattern; tenant-agnostic unit tests).
- **Migrations:** `migrations/rls/20260714180000_sesame_rls_contract_v1.sql` (install + GRANT to
  `sesame_idam`), `..._users_tenant_rls.sql` (forced tenant policy on `sesame_idam.users`).
- **Reference:** `sql/rls/v1/reference-idam-users.sql`.
- **Test:** `database/tests/rls_users_zero_bleed.rs` — unqualified SELECT tenant isolation.
- **Verify (ms02):** `cargo test -p sesame_idam_database` — 6/6 pass.
- **Next:** wire protected controllers to `with_session_transaction`; org-scoped policies on more tables.

## [2026-07-14] feat | Epic 10 P1 — platform tenant admin API, CLI, BDD

- **OpenAPI + codegen (10.1):** `PlatformAdmin` tag, 5 routes, `PlatformServiceAuth` on
  `identity-login-service` spec; `just gen-identity-login`.
- **Platform auth (10.7):** `platform_auth.rs` constant-time key check; `security.rs` registers
  `SESAME_PLATFORM_ADMIN_KEY` (fallback `dev-platform-admin`).
- **Controllers (10.2–10.5):** `platform_tenant_{create,get,status_patch,oauth_upsert,oauth_rotate}`.
  OAuth upsert returns DB metadata (not `resolve` — avoids secret env requirement on PUT).
- **CLI (10.6):** `sesame-idam tenant {create,get,status set,oauth set,oauth rotate}` in
  `tooling/src/sesame_idam_tooling/cli/tenant.py`; unit tests in `tooling/tests/test_tenant_cli.py`.
- **BDD (10.8):** `platform_tenant_admin.rs` — mint→register→login, unknown slug, suspend, rotate.
- **Verify:** `cargo nextest run -p sesame_idam_identity_login_service` on ms02.

## [2026-07-14] docs | PRD-P1 platform tenant admin + stories 10.1–10.8

- **PRD:** [PRD-P1-platform-tenant-admin.md](../PRD-P1-platform-tenant-admin.md) — platform REST API,
  `X-Platform-Admin-Key` auth, CLI mirror, OAuth metadata/rotate, acceptance gate.
- **Stories:** `Epics/10-platform-tenancy/stories/story-10.{1..8}.md` with acceptance criteria.
- **Build order:** 10.1 → 10.7 → 10.2 → 10.3 → 10.4 → 10.5 → 10.6 → 10.8.
- **Updated:** Epic 10 README, docs-catalog, index.md, topic-platform-tenants.md, design doc PRD map.

## [2026-07-14] docs | SaaS-of-SaaS multi-tenancy design document

- **Deliverable:** [design-saas-of-saas-multi-tenancy.md](../design-saas-of-saas-multi-tenancy.md) — canonical
  design for tenant registry, platform vs self-service provisioning, store/Stripe/KYC pipeline,
  platform OAuth vs org Application, secrets phases, API surface, data model, phased delivery (P1–P4),
  and epic-ready story backlog (10.1–10.21).
- **Epic stub:** [Epics/10-platform-tenancy/README.md](../Epics/10-platform-tenancy/README.md).
- **PRD map:** P1 platform-admin API, P2 self-service worker, P3 online store + KYC, P4 scale.
- **Updated:** docs-catalog, index.md, topic-platform-tenants.md.

## [2026-07-14] feat | Platform tenant registry — SaaS-of-SaaS (ADR-004)

- **Decisions locked:** both platform-admin and self-service tenant minting; hard reject unknown
  slugs (`404 tenant_unknown`); OAuth secrets in K8s/env with DB metadata; platform tenant OAuth
  separate from org `Application`; hauliage + pricewhisperer seeded now to avoid PW migration.
- **Models:** `tenants`, `tenant_oauth_providers` in identity-login-service.
- **Services:** `TenantService`, `TenantOAuthService`, `tenant_gate`.
- **Wired:** `auth_login`, `auth_register`, `signup_validate`, `social_login`, `social_callback`.
- **Seed:** `impl/seeds/20260714000000_platform_tenants.sql`.
- **Docs:** [ADR-004](../ADR-004-platform-tenant-provisioning.md), [topic-platform-tenants](./topics/topic-platform-tenants.md).
- **TODO:** migrator run + apply on ms02; platform admin API + OAuth rotate REST; hauliage BFF social proxy.

## [2026-07-14] harden | P0 dynamic token-status enforcement

- Centralized denylist/version enforcement in BRRTRouter's `JwksBearerProvider` for every Sesame
  protected consumer; removed the obsolete authz-only fail-open middleware placeholder.
- Removed active/negative caching so logout is visible on the next protected request; retained
  only bounded monotonic revoked/stale caching.
- Added 16 Redis connection shards, 250 ms connect/read/write bounds, uniform fail-closed behavior,
  and fixed-label `sesame_token_status_checks_total` scrape output without token contents.
- Added [ADR-003](../ADR-003-token-status-dependency-outage.md), the detailed
  [FR-P0-005 story](../roadmap/launch-1.0/p0-harden-core/fr-p0-005-denylist-read-side/README.md),
  and [token-status wiki page](./topics/topic-token-status-enforcement.md).
- Verification: focused BRRTRouter 7/7; Sesame token-status 9/9 against the shared Redis endpoint;
  workspace lint; workspace check; and Nextest 891/891 with retries disabled. Cross-service
  logout-to-401 and a deployed `/metrics` outage scrape remain before marking FR-P0-005 accepted.

## [2026-07-14] feat | OAuth MVP — Google/Microsoft social login (identity-login-service)

- **BRRTRouter:** `HttpRedirect` typed response (302 + `Location` header) in `src/typed/core.rs`.
- **identity-login-service:** Real `social_login` (authorize redirect + Redis CSRF state) and
  `social_callback` (code exchange via `sesame_common::http`, user link/create, `token_issuer`).
  Providers: **google**, **microsoft**. **Per-tenant env** (no global fallback):
  `SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_ID/SECRET`, same for Microsoft,
  `SESAME_OAUTH__{TENANT}__ALLOWED_REDIRECT_URIS` (e.g. `SESAME_OAUTH__HAULIAGE__*`).
- **Services:** `services/oauth/{config,state,providers}`, `social_credential_service`,
  `UserService::create_oauth_user`.
- **Routes:** Registered in `main.rs` (overwrite gen stubs).
- **Verify (ms02):** `cargo check -p sesame_idam_identity_login_service` + 3 OAuth unit tests green.
- **Still TODO:** Hauliage BFF proxy + frontend callback page; provider app registration; BDD with
  live Google/Microsoft; full P2 linking/MFA gates.


- Evaluated the strategic and dated roadmaps and clarified their different purposes: the
  six-week D3/D4 Hauliage cut is **“just enough IDAM” to onboard initial test users**, not a
  Sesame product release; Launch 1.0 GA remains P0+P1+P2+P4.
- Added [`docs/roadmap/launch-1.0/`](../roadmap/launch-1.0/README.md) with a current delivery
  assessment, release relationship, traceability rules, global Definition of Done, and ten
  cross-cutting non-functional requirements.
- Broke P0–P5 into linked subdirectories. Each phase now defines scope/dependencies, stable
  functional and non-functional requirement IDs, observable acceptance criteria, and exit
  evidence.
- Added a separate [Hauliage test-user enablement contract](../roadmap/launch-1.0/hauliage-test-user-enablement/README.md)
  for the shipper/transporter journeys, live BFF/Kubernetes proof, tenant isolation, reset
  procedure, and an explicit go/no-go decision for initial test users.
- Cross-linked every phase from [`ROADMAP-launch-1.0.md`](../ROADMAP-launch-1.0.md), clarified
  the dated Hauliage roadmap banner, and catalogued the new specifications.

## [2026-07-13] docs | Launch 1.0 product roadmap (deliver on the README)

- **New:** [`docs/ROADMAP-launch-1.0.md`](../ROADMAP-launch-1.0.md) — full-product roadmap to
  make Sesame competitive with PropelAuth/Clerk and the adoption wedge for BRRTRouter.
- **Grounded in** a competitive gap analysis: Sesame ≈12% of a full IdP today (~16/136
  endpoints real). Strong verticals = asymmetric JWT/JWKS + org lifecycle + tenant isolation.
  Missing = user-mgmt (0/26), MFA/social/passwordless, SSO/SCIM, API-key lifecycle, webhooks,
  **RLS bridge (0 lines — the README killer feature)**, TS SDK, hosted UI.
- **Thesis:** lead with the moat (security + **RLS bridge**) + DX (SDK/hosted UI) for envy;
  interleave table-stakes. **Launch 1.0 = P0 harden + P1 RLS bridge + P2 auth surface + P4
  developer contract** (~3–4mo). P3 enterprise SSO/SCIM + P5 trust/SOC2 = 1.1/1.2 fast-follow.
- **Immediate:** P0 revocation enforcement in BRRTRouter `JwksBearerProvider` (also a BRRTRouter
  selling feature); de-risk P1 with a `SesameExecutor` + zero-bleed spike before scoping.

## [2026-07-13] feat | D3/D4 frozen-contract build — fetch_org, signup_validate, org metadata, logout revocation

Branch `feat/d4-hauliage-consumer-surface`. The ADR-002 §3.1 Hauliage consumer contract is now functionally complete.

- **`fetch_org`** (`GET /organizations/{id}`, `479b30d`) — real controller + `org_lifecycle::get_organization` with membership + cross-tenant isolation (Forbidden, no existence leak). BDD member/non-member/cross-tenant (live DB).
- **`signup_validate`** (`GET /auth/signup/validate`, `a8123ca`) — tenant-scoped availability pre-check (email_required/invalid/tenant_required/email_taken); register stays the authoritative gate. Was unregistered stub.
- **Org `metadata`** (`97fb99a`) — JSONB column via `Org` LifeModel + migrator (`20260713065042_organizations.sql`, additive ALTER); `create_organization` persists opaque persona metadata (ADR-002 §3.3), `fetch_org` echoes it. **Migrated `create_organization`/`get_organization` from raw sea_query SQL → Lifeguard ORM** per policy; remaining invite/accept/list fns flagged legacy.
- **Logout revocation write-path** (`88b094e`) — logout denylists the access `jti` (`denylist:{jti}`, TTL=remaining life) via validated `jwt_claims`, aligned with `sesame_common::denylist` key scheme. Read-side enforcement (`DenylistMiddleware`) still stubbed (placeholder Redis closure) — bounded by deferred hybrid fallback.
- **Verify:** login 49/49 + org-mgmt 14/14 serial-green. Pre-existing parallel flake = `authz_enrichment` shared-env race (serialized by nextest `db-serial`). Dev DB: added `metadata` column as `postgres`, granted `sesame_idam` DML.

See [`../audit/delivery-roadmap-2026-07-13.md`](../audit/delivery-roadmap-2026-07-13.md) "Progress — 2026-07-13".

## [2026-07-13] docs | Delivery roadmap re-scoped to 6-week D3/D4 Hauliage launch

- **New:** [`docs/audit/delivery-roadmap-2026-07-13.md`](../audit/delivery-roadmap-2026-07-13.md).
- **Scope decision:** deliver narrow **D3/D4 Hauliage-consumer** target first — launch ≈6 weeks (2026-08-24). Full aspirational surface (RLS bridge, TS SDK, hosted UI, 119-endpoint parity, hybrid online-fallback authz) deferred to post-launch (Appendix A).
- **Gap map (launch-scope only):** real = create org / memberships / accept+send invite / register / active-org / core auth. **Stub or missing** = `fetch_org`, `fetch_users_in_org`, `change_user_role_in_org`, `remove_user_from_org`, `revoke_pending_invite`, `invitations/preview` (no controller), role→permission mapping in `principal_effective`.
- **6-week plan:** W1 commit Wave A + A8 verify + freeze BFF contract; W2 org read surface; W3 invite/membership lifecycle; W4 role→perm + persona claims; W5 revocation minimum (jti denylist + `ver` rejection, launch blocker); W6 E2E + tenant-isolation suite.
- **Open decisions:** ADR-002 S3 webhook/worker provisioning in/out; api-keys rotation in/out; RLS bridge confirmed OUT (Hauliage self-enforces tenancy).

## [2026-05-17] Entity Wiki Pages — Comprehensive Audit and Fix

### Summary

Complete audit of all 17 entity wiki pages against the actual Lifeguard impl models in the `impl/` crates. Cross-referenced each page against every column in every impl model for matching services. Created 7 missing entity pages, fixed 1 existing page, verified 9 existing pages.

### Changes Made

**7 new entity pages created:**

| Entity | Service | Columns | Key Details |
|--------|---------|---------|-------------|
| entity-email-verification.md | identity-user-mgmt-service | 6 | FK cascade to users, token limited to 64 chars |
| entity-social-account.md | identity-user-mgmt-service | 8 | FK cascade to users, provider/user_id strings |
| entity-employee.md | identity-user-mgmt-service | 8 | Self-referencing manager_id (ON DELETE SET NULL) |
| entity-scim-user.md | org-mgmt | 7 | Minimal SCIM model, no FK to users table |
| entity-org-domain.md | org-mgmt | 6 | Domain verification status |
| entity-org-invite.md | org-mgmt | 8 | Timestamp-based acceptance (not boolean/status) |
| entity-org-membership.md | org-mgmt | 7 | FK cascade on org_id and user_id, role is free-form string |

**1 existing page corrected:**

| Entity | Issue Fixed |
|--------|-------------|
| entity-api-key.md | Added references to api_key_usage and archived_api_key impl models (endpoint, method, reason, archived_at columns) |

**10 existing pages verified as complete** — all impl columns present:
- entity-user.md, entity-session.md, entity-organization.md, entity-role.md, entity-permission.md, entity-application.md, entity-mfa-device.md, entity-audit-log.md, entity-webhook.md, entity-scim-user.md

**Index updated:**
- `docs/llmwiki/index.md` — All 17 entity pages listed with status `verified` (changed entity-webhook from `partially-verified` to `verified`)

### OpenAPI vs Impl Discrepancies (Documented in ERD)

The ERD documents 41 impl models across 6 services. 17 impl models have **no corresponding OpenAPI schema** — they are database-only entities queried via service APIs without dedicated REST endpoints. The ERD also documents 14 categories of schema mismatches where OpenAPI specs describe properties that don't exist in impl, or vice versa.

### Open Issues

| Entity | Issue |
|--------|-------|
| Role/Permission | OpenAPI spec says `application_id`, impl uses `org_id` — specs are stale |
| AuditEvent (all) | OpenAPI spec has 16 properties (event_action, hmac_signature, target_id, etc.) — doesn't match either impl version (8-col authz-core or 10-col user-mgmt) |
| Org | OpenAPI spec has 21 properties including slug, logo_url, domain_auto_join, SAML fields — impl has only 6 columns |
| Application | OpenAPI spec has `slug`, impl has OIDC fields (client_id, client_secret, redirect_uris) |
| ScimUser | OpenAPI spec uses SCIM protocol format (emails array, name object, roles) — impl is a simple 7-col table |
| WebhookSubscription | OpenAPI spec has 12 properties with delivery tracking — impl has 8 columns with `active` boolean, not `enabled` |

## [2026-05-17] Epics Location and Implementation Status

### Summary

Added epics documentation discoverability and implementation status tracking. Fresh agents were not finding `docs/Epics/` because it was never referenced in AGENTS.md or the wiki index.

### Changes Made

**AGENTS.md** — Added `docs/Epics/INDEX.md` to the docs catalog table with description. Added epics directory layout explanation below the table: `docs/Epics/{N}-{name}/stories/story-N.M.md` pattern, INDEX.md as canonical master index.

**INDEX.md** — Added `Status` column to the epic table. Added "Implementation Status" section with:
- Story-level status for all 9 epics (44 stories total)
- Epic 1 Story 1.1 marked as **Implementing** — detailed file inventory: `key_manager.rs` (807 lines, Ed25519 gen/sign/verify, KeyManager with rotation/revocation/health, 11 unit tests), `controllers/jwks.rs`, `controllers/admin_jwks_revoke.rs`, `jwks_client.rs`, `main.rs`
- All other 40 stories marked as **Design** — verified by searching impl/ for story keywords (`jwt_only`, `jwt_with_fallback`, `route_policy`, `RouteAuthCategory`, `RoutePolicyStore`, claims schema types, version cache, delegation `act` claim, caching, observability spans) — none found
- Updated overall status from "Design phase -- no code changes" to "Story 1.1 in implementation"

### Verification

Searched all impl/ crates for implementation keywords. Only Epic 1 (asymmetric JWT) has code. Confirmed via: `search_files` across all impl dirs for key terms returned matches only in `identity-session-service/impl/` for key_manager, jwks, Ed25519, KeyManager. Zero matches for route classification or claims schema code.

### Open Issues

- Story 1.1 is "implementing" but not yet verified as compiling. No check was run that the key_manager changes integrate cleanly with the rest of identity-session-service build.
- The INDEX.md status section will need updates whenever new stories move from design to implementing.

### Files Changed

| File | Action |
|------|--------|
| `docs/llmwiki/entities/entity-email-verification.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-social-account.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-employee.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-scim-user.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-org-domain.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-org-invite.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-org-membership.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-api-key.md` | Patched — added missing entity references |
| `docs/llmwiki/index.md` | Patched — added 7 new entities, fixed webhook status |
|| `docs/llmwiki/topics/topic-entity-relationship-diagram.md` | Updated — comprehensive ERD + all 41 impl models + OpenAPI gaps |
|| `docs/llmwiki/topics/topic-data-model.md` | Updated — full table list + 17 impl models without OpenAPI + 14 schema mismatches |

## [2026-05-18] Epic 9 — Observability OTEL Spans

### Summary

Wired OTEL tracing spans across all 6 sesame-idam microservices following the hauliage BRRTRouter pattern. No custom Prometheus counters — all JWT/authz diagnostics flow through `tracing::span!()` into the existing `brrtrouter::otel::init_logging_with_config()` stack. Fixed a compilation error in `authz_span_middleware.rs` (`res.status.as_u16()` → `res.status`). Created comprehensive span catalog wiki page.

### Changes Made

**Core span files:**

| File | Span | Purpose |
|------|------|---------|
| `key_manager.rs` | `key.generate` | Key generation at bootstrap and rotation |
| `key_manager.rs` | `key.rotate.prepare` | Prepare next key for rotation |
| `key_manager.rs` | `key.rotate.activate` | Promote next key to current |
| `key_manager.rs` | `key.revoke` | Key revocation with reason tracking |
| `key_manager.rs` | `key.health` | Health check with key count |
| `jwks_client.rs` | `jwks.cache.refresh` | JWKS cache validation with hit/miss/cache_status |
| `controllers/jwks.rs` | `jwks.document` | JWKS document served with key count |
| `controllers/admin_jwks_revoke.rs` | `key.revoke.admin` | Admin revoke endpoint |
| `controllers/auth_refresh.rs` | `token.refreshed` | Token refresh with user_id, tenant_id |
| `controllers/admin_issue_token.rs` | `token.issued` | Admin token issuance |
| `auth_token.rs` (login) | `token.issue` | Token issuance with grant_type |

**Middleware:**

| File | Span | Purpose |
|------|------|---------|
| `authz_span_middleware.rs` | `authz.request` | Wraps all authz-core requests (route, method, result) |
| `main.rs` (all 6 services) | N/A | `set_extra_prometheus` for Lifeguard DB metrics in /metrics |

**Controller spans (other services):**

| Service | Controller | Span |
|---------|-----------|------|
| identity-user-mgmt | create_user.rs | `user.created` |
| identity-user-mgmt | delete_user.rs | `user.deleted` |
| identity-user-mgmt | disable_user.rs | `user.disabled` |
| api-keys | create_api_key.rs | `api_key.created` |
| api-keys | delete_api_key.rs | `api_key.deleted` |
| org-mgmt | delete_org.rs | `org.deleted` |
| org-mgmt | create_application.rs | `application.created` |

**Wiki updates:**

| File | Action |
|------|--------|
| `topics/topic-observability.md` | **Created** — Full OTEL span catalog with 15 span entries, attributes, security constraints, not-yet-implemented section |
| `index.md` | Patched — Added observability topic link |

**Bug fixes:**

| File | Fix |
|------|-----|
| `authz_span_middleware.rs` | `res.status.as_u16()` → `res.status` (u16, not StatusCode) |

### Compilation

`cargo check --workspace`: **PASS** (0 errors)

### Gaps (not yet implemented)

- **Story 9.1 full sub-spans**: `jwt.typ_check`, `jwt.signature_verify`, etc. happen inside BRRTRouter's `JwksBearerProvider::validate_token()` — would require changes to BRRTRouter itself
- **Story 9.3 authz fallback spans**: Blocked until Story 4 (hybrid authz) implementation
- **Story 9.4 shadow decision spans**: Blocked until migration mode
- **Story 9.5 token revocation span**: No token revocation endpoint exists yet
- **Story 9.6 structured JWT logging**: Partial — token lifecycle controllers have spans; per-request JWT fields (issuer, subject, session_id, jti) not yet wired
- **Story 9.7 alerting**: No Loki/Grafana alert rules created yet (spans/logs are ready for them)
- **Controller coverage**: Only representative controllers instrumented; many CRUD read/list controllers still lack spans

## [2026-07-06 pm3] Wiki State Sync — Implementation Status Snapshot

### Current implementation state (for future sessions)

**Real, DB/Redis-backed, tested (live Kind postgres + redis):**

| Endpoint | Service | Notes |
|----------|---------|-------|
| `POST /auth/register` | identity-login-service | argon2id, tenant-scoped, real Ed25519 tokens |
| `POST /auth/login` | identity-login-service | credential verify + authz-core role enrichment (best-effort) |
| `POST /auth/refresh` | identity-session-service | Redis rotation (works on login-issued tokens) |
| `GET /.well-known/jwks.json` | identity-session-service | shared signing key via Secret, rotation/revocation |
| `POST /authz/principals/effective` | authz-core | roles + attributes from PG; permissions pending |
| `GET/PATCH /identity/me` | identity-session-service | raw handlers (JWT principal), user_profiles upsert |

**Key architectural facts learned:**

- Typed BRRTRouter handlers drop `jwt_claims` — principal-dependent endpoints must use raw handlers (`identity-session-service/impl/src/raw_handler.rs`).
- Shared Ed25519 signing key distributed via `sesame-idam-jwt-signing` K8s Secret (`just jwt-signing-secret`); login signs, session publishes JWKS.
- Tenant ids are plain strings (slugs like `hauliage`); `format: uuid` removed from X-Tenant-ID in all specs.
- Builds/tests run on ms02 (`ssh ms02`, `export PATH=$HOME/.cargo/bin:$PATH`); local Mac builds over NFS are ~100x slower.
- Live-DB BDD pattern: `db_available()` skip-gate + `DB_POOL_MAX=2` (nextest is process-per-test; bigger pools exhaust postgres max_connections).

**Everything else** (OTP, social, magic links, MFA, SCIM, SSO, org-mgmt CRUD, api-keys, user-mgmt admin CRUD, `authorize` decisioning) is still gen-stub mocks.

## [2026-07-06 pm2] /identity/me DB-Backed + Helm Env Wiring + Tenant Format Fix

### Summary

Three hauliage unblockers:

1. **`GET`/`PATCH /identity/me` (H4.1, identity-session-service)** — DB-backed current-user profile. Key discovery: BRRTRouter's typed dispatch (`TypedHandlerRequest<T>`) drops `HandlerRequest::jwt_claims`, so principal-dependent endpoints cannot be typed handlers. New `raw_handler` module: `spawn_raw_handler()` (dedicated coroutine + panic→500, mirroring typed spawn) and `authenticated_principal()` (sub/tenant_id from validated claims, cross-checked against `X-Tenant-ID` header). `ProfileService` reads `users` (entity duplicated into session models) + `user_profiles`, PATCH upserts first/last name + avatar with partial-update semantics. 6 live-DB BDD tests incl. cross-tenant denial.
2. **Helm env wiring (H6.2)** — deployment.yaml now injects `DB_*` (values + optional `sesame-idam-db-credentials` Secret with dev fallback), `REDIS_URL`, `AUTHZ_CORE_URL` (login only, via values `app.config.authzCoreUrl`), and the shared JWT signing key from the `sesame-idam-jwt-signing` Secret (both env vars optional so services boot without it). New `sesame_keygen` bin in sesame-common + `just jwt-signing-secret` recipe generates/applies the Secret — applied to the shared Kind cluster (kid `key-2026-07-06-0807`). Values files fixed: app database is `sesame_idam`/`sesame_idam` role (was pointing at the `postgres` superuser DB, which does not contain our schema).
3. **X-Tenant-ID format (spec)** — dropped `format: uuid` from all X-Tenant-ID params in the 6 specs; tenant ids are slugs (`hauliage`) or uuids. Verified BRRTRouter's `decode_param_value` never validated the uuid format, so this is documentation truth-up, not a behavior change; gen crates pick it up at next regen. All specs pass `just lint-openapi`.

Also capped `DB_POOL_MAX=2` in all live-DB test fixtures — parallel nextest processes each open their own Lifeguard pool and were exhausting Postgres max_connections (flaky pool-init panics).

### Gates

`just nt` 867/867 PASS, `just lint-rust` PASS, `just lint-openapi` PASS, helm template renders the new env block.

### Open Issues

- `PATCH /identity/me` ignores `name`/`preferred_username` (no storage column; spec fields noted in controller docs).
- user-mgmt admin CRUD endpoints (get/update/disable user) still stubs — rest of H4.1.
- gen crates not regenerated after the spec format change (behaviorally identical; next `just gen` picks it up).

## [2026-07-06 pm] Login-Time Role Enrichment via authz-core (H3.1/H3.3/H3.5)

### Summary

Wired the single sanctioned cross-service dependency: identity-login-service now calls authz-core `POST /authz/principals/effective` at login (may_http, 500ms timeout, `AUTHZ_CORE_URL` env, default `http://authz-core:8102`) and embeds the returned roles in both the `TokenResponse.roles` field and the signed token's namespaced `sx.roles` claims. Enrichment is best-effort: if authz-core is unreachable, login succeeds with empty roles (logged warning) — resolves Epics INDEX open question #1 as **login-time enrichment** for v1.

authz-core `principal_effective` is now DB-backed: tenant-scoped queries over `role_assignments` (roles) and `principal_attributes` (attributes) via a new `PrincipalService`; permissions remain empty until the org-mgmt role→permission mapping / entitlements work. Non-UUID principals return empty without touching the DB. Register & Overwrite wired in authz-core main.rs + Lifeguard pool warmup.

New seed `authz-core/impl/seeds/20260706000001_hauliage_demo_roles.sql`: OWNER/DISPATCHER/DRIVER for the three hauliage demo users (applied to the Kind postgres). NOTE: seed_order.txt must be regenerated (`cargo run -p sesame_idam_migrator`) after adding seeds, or setup-db.sh skips them.

### Tests

- authz-core: live-DB BDD (`principal_effective_db.rs`) — seeded role resolves, tenant isolation (no cross-tenant leak), unknown principal → empty, non-uuid guard.
- login: `authz_enrichment.rs` — mock authz-core (may_minihttp) proves roles land in response + sx claims; unreachable authz-core proves graceful degradation.
- Gates: `just nt` 861/861 PASS, `just lint-rust` PASS.

### Open Issues

- Roles are enriched but `permissions`/entitlements are still empty (needs org-mgmt mapping, Epic 2/7).
- For roles to appear in real deployments, authz-core must be reachable from login-service (Tilt/Helm wiring, H6.3).

## [2026-07-06] Real Login/Register + WIP Refactor Landed (Hauliage P0/P1)

### Summary

Implemented the first real, DB-backed authentication flow end-to-end and stabilised the previously-broken WIP tree. `POST /auth/register` and `POST /auth/login` on identity-login-service now verify argon2id credentials against `sesame_idam.users` (tenant-scoped), issue genuine Ed25519-signed `at+jwt` access tokens (kills `placeholder_signature` for the login path), seed refresh-token state in Redis compatible with session-service rotation, and return spec-conformant `TokenResponse` (the contract hauliage's E2E fixtures encode). Migrations + hauliage demo seed applied to the shared Kind postgres; 7 live-DB BDD tests pass including `hauliage_demo_user_logs_in`.

### Key changes

- **sesame-common**: fixed 4 compile errors in the WIP refactor (jwks_cache Uri/http_legacy, JwksCacheInner visibility, ParseError Clone, arc-swap dep). New `jwt::signer::Ed25519Signer` — shared signing key via `SESAME_JWT_SIGNING_KEY_PKCS8_B64`/`SESAME_JWT_SIGNING_KID` env (K8s Secret), dev fallback generates ephemeral key. **Real bug fixed:** `VersionStore` used `INCRBY key 0` — versions never advanced; now increments by 1. `get_key` no longer silently substitutes a different JWKS key on kid miss (use `get_any_valid_key` explicitly). `jwt_claims_cover_decision` returns true for empty requirements; `sanitize_key_input` preserves unicode, strips control chars; `set_ttl_config` merges over defaults; DPoP proof at exactly 60s now rejected.
- **identity-session-service**: `KeyManager::new()` bootstraps from the shared signing key env so JWKS publishes the same key login signs with. Rotation prefers `token.family_id` from Redis over the caller-supplied hint. Bin now reuses lib modules (no duplicate compilation).
- **identity-login-service**: restructured so all modules live in the lib (bin + tests share). New: `models/user.rs` (duplicated from user-mgmt per shared-schema convention, with `composite_unique = "tenant_id, email"`), `services/{password,user_service,token_issuer}`, `redis.rs`, real `auth_login`/`auth_register` controllers using the hauliage lifeguard pattern (stateless service + `sesame_idam_database::db()` at controller edge), Register & Overwrite in main.rs.
- **Migrations/seeds**: migrator paths fixed (`../../../migrations` wrote OUTSIDE the repo → `../../migrations`); users migration regenerated with `UNIQUE(tenant_id, email)`; `scripts/setup-db.sh` seed path fixed; new seed `identity-user-mgmt-service/impl/seeds/20260706000000_hauliage_demo_users.sql` (owner/dispatcher/driver @hauliage.dev, password `SecureP@ss123!`).
- **Deps**: `may_minihttp` switched to the microscaler fork (AGENTS rule; crates.io was silently used), `argon2` added.
- **Lint**: `just lint-rust` was broken (referenced deleted `sesame-audit` crate → recipe always errored; historical "Lint PASS" claims were vacuous). Recipe fixed, ~900 pedantic findings fixed/auto-fixed across impl crates; strict gate now **passes**. `sesame-common` has ~380 remaining pedantic findings — split into `just lint-common` (Phase 1 warn) per the jsf-linting phase plan.

### Gates

- `cargo check --workspace`: PASS. `just nt`: **852/852 PASS** (count reduced from 890 by deduplicating session-service bin/lib double-compiled tests; new signer/password/auth-flow tests added). `just lint-rust`: PASS. DB migrations + seeds applied to Kind postgres (`sesame_idam` db).
- NOTE: builds/tests must run on ms02 (`ssh ms02`, PATH needs `~/.cargo/bin`); local Mac builds over NFS take >20min.

### Open Issues

- Login issues tokens with empty `roles` — authz-core `/principal/effective` call (H3.5) not wired yet.
- OpenAPI `X-Tenant-ID` is `format: uuid` but the hauliage tenant is the string `hauliage` — spec vs tenancy-model conflict to resolve before HTTP-level integration.
- `sesame-common` pedantic backlog (~380) tracked via `just lint-common`.
- `db_integration_suite` target for `just nt-db-suite` still missing (H1.6).

## [2026-07-06] Hauliage Readiness Plan

### Summary

Cross-repo audit of sesame-idam vs hauliage to produce a launch-readiness backlog. Findings: sesame-idam is a compile-clean scaffold — 127 handlers, zero DB-backed; login/register/authorize/validate are echo stubs; real code = session-service JWKS/KeyManager, Redis refresh rotation, common crate libraries. Hauliage side: stub identity service, env-var org scoping (`HAULIAGE_ORGANIZATION_ID`), mocked AuthGuard, but BRRTRouter `JwksBearerProvider` plumbing ready; its E2E fixtures already encode the sesame-idam `TokenResponse` contract.

### Deliverable

`docs/plan/hauliage-readiness-plan.md` — 7 epics (H1 persistence, H2 real auth, H3 claims/authz, H4 users/orgs, H5 api-keys, H6 deployment, H7 E2E) + hauliage-side Track B, with P0–P3 sequencing and 4 blocking decisions (signing locus, authz-core call pattern, org boundary vs hauliage `company`, cluster topology).

### Open Issues

- `db_integration_suite` binary referenced by `just nt-db-suite` does not exist in the repo.
- Helm `deployment.yaml` injects no DB/Redis env vars.
- Large uncommitted refactor (config→common, jwt/jwks_cache/fallback_cache splits) must land before backlog work stacks on it.

## [2026-06-10] HTTP Client Policy — may_http Only

### Summary

Audit found `reqwest::Client` in `jwks_cache.rs` (real usage, not comments), and `tokio::spawn` in `token_versioning/subscriber.rs` and `version_store.rs`. `reqwest` depends on `tokio` runtime — using it in may-coroutine services would spawn a separate runtime threadpool, breaking the single-runtime constraint.

### Decision

**Rule: All outbound HTTP in Sesame-IDAM must use `may_http` only.** Banned: `reqwest`, `hyper` (direct), `surf`, `ureq`, `isahc`, any `tokio::spawn` for background tasks. Allowed: `may_http::client::Client` for all outbound HTTP, `may::task::spawn` for background/coroutine tasks.

### Files Requiring Migration

| File | Issue | Status |
|------|-------|--------|
| `jwks_cache.rs` | `reqwest::Client` field + `client.get(endpoint).send()` | To migrate |
| `token_versioning/subscriber.rs` | `tokio::spawn` at lines 214, 892 | To migrate |
| `token_versioning/version_store.rs` | `tokio::spawn` at line 512 | To migrate |

### Wiki Changes

- Created `topics/topic-http-client-policy.md` — HTTP client policy page
- Updated `index.md` — Added topic-http-client-policy link
- Updated `log.md` — This entry

## [2026-07-06 pm4] rustls Confirmed — openssl-sys Removed

### Summary

Confirmed `rustls` is the effective TLS stack and `openssl-sys` is absent from the workspace dependency graph (including `x86_64-unknown-linux-musl`).

### Root cause of musl failure

The direct `reqwest` dependency on `identity-session-service` used default features, which pull `native-tls` → `openssl-sys`. That direct dep was removed in the prior session.

### Additional cleanup

- Removed unused workspace `reqwest = { version = "0.12", features = ["json"] }` from `microservices/Cargo.toml` — no crate referenced it, and default features would reintroduce `native-tls` if someone added it back carelessly.
- Documented rustls-only policy in `topics/topic-http-client-policy.md`.

### Verification

```bash
cargo tree --target x86_64-unknown-linux-musl -p sesame_idam_identity_session_service -i openssl-sys
# → "did not match any packages"
```

Transitive `reqwest` from BRRTRouter uses `default-features = false, features = ["rustls", ...]`. Service code uses `may_http` only.

### Open Issues

- H6.1 in-cluster login smoke still returns 401 (security provider registration) — separate from TLS stack.

## [2026-07-06 pm5] may_http as Zero-reqwest Target

### Summary

User direction: move fully off `reqwest`; use coroutine-compliant HTTP client for inter-controller calls. Reviewed `wrk-rs/src/main.rs` and `may_http` source.

### Findings

- **Client is `may_http`, not `may_minihttp`.** `may_minihttp` is server-only; `may_http` (`rust-may/may_http`) has `HttpClient` in `src/client/client_impl.rs` — blocking I/O on `may::net::TcpStream`, cooperative inside coroutines.
- **`wrk-rs`** is the canonical usage: `HttpClient::connect((host, port))`, `client.get(uri)`, read body in coroutine scope.
- **sesame-idam service code** already on `may_http` (`authz_client.rs`, `jwks_cache/cache.rs`).
- **Remaining reqwest** is BRRTRouter-only: `jwks_bearer`, `remote_api_key`, `spiffe/validation` use `reqwest::blocking`; OTEL HTTP exporter is transitive.
- **may_http gaps:** no TLS, no DNS (empty `dns.rs`), no connection pool. Blocks external HTTPS JWKS until rustls layer added (fork candidate).

### Wiki

Updated `topics/topic-http-client-policy.md` — migration goal, wrk-rs pattern, gap table, BRRTRouter targets.

## [2026-07-06 pm6] BRRTRouter Refactor Scope for Zero-reqwest

### Summary

User confirmed BRRTRouter likely needs refactoring too (not just sesame-idam). Mapped all reqwest call sites and transitive deps in BRRTRouter sibling repo.

### Production reqwest usage (3 files)

- `security/jwks_bearer/mod.rs` — blocking client + `std::thread` background JWKS refresh
- `security/spiffe/validation.rs` — duplicated JWKS fetch logic
- `security/remote_api_key.rs` — blocking GET on request validation path

### Architectural issues beyond reqwest swap

- JWKS refresh uses OS threads + `thread::sleep`, not may coroutines
- JWKS fetch logic duplicated between JWKS bearer and SPIFFE providers
- External HTTPS JWKS blocked until may_http gets rustls TLS

### Transitive reqwest

- `opentelemetry-http` (OTLP HTTP exporter) — mitigate via grpc-tonic-only
- `jsonschema` — remote schema fetch; check feature flags
- `goose` — dev-deps only

### Decision

`brrtrouter::http` wrapper over `may_http` should live in **BRRTRouter** (benefits hauliage + sesame-idam). Documented 6-phase sequencing in `topic-http-client-policy.md`.

## [2026-07-06 pm7] BRRTRouter Phase 1 — `brrtrouter::http` + Security Provider Migration

### Summary

Implemented Phase 1 in BRRTRouter sibling repo: new `brrtrouter::http` module and migrated all three production `reqwest::blocking` call sites.

### Changes (BRRTRouter repo)

- `src/http/mod.rs`, `src/http/fetch.rs` — `fetch_get`, `fetch_get_text_with_retry`
- HTTP: `may_http::HttpClient`; HTTPS: rustls + `may::net::TcpStream` + httparse
- `security/remote_api_key.rs` — may_http fetch with `X-API-Key` header
- `security/jwks_bearer/mod.rs`, `security/spiffe/validation.rs` — shared retry fetch
- `Cargo.toml` — added `may_http`, `http_legacy`, `httparse`, `rustls`, `rustls-platform-verifier`

### Remaining

- JWKS background refresh still on `std::thread` (Phase 3)
- Direct `reqwest` dep retained for OTEL/jsonschema transitive
- Test helpers still use reqwest blocking
- Full `cargo check` blocked locally by pre-existing `prost-types` env issue (unrelated to this change)

## [2026-07-06 pm8] BRRTRouter HTTP Fetch Test Coverage

### Summary

Added unit + integration tests for `brrtrouter::http` and provider wiring after may_http migration.

### Tests added (BRRTRouter repo)

**Unit (`src/http/fetch.rs`):** parse 404, partial headers, bounded read, malformed URL, error Display

**Integration (`tests/http_fetch_tests.rs`):**
- `fetch_get_http_returns_json_body`
- `fetch_get_http_rejects_oversize_body`
- `fetch_get_http_sends_extra_headers`
- `fetch_get_text_with_retry_succeeds_after_transient_failure`
- `fetch_get_text_with_retry_returns_none_when_all_attempts_fail`
- `remote_api_key_provider_validates_via_http_fetch`
- `fetch_get_jwks_shaped_document_via_retry_helper`
- `jwks_bearer_provider_loads_keys_via_http_fetch` — end-to-end JWKS refresh + JWT validate
- `fetch_get_connect_error_on_dead_port`

### Wiki

Updated `topic-http-client-policy.md` Phase 1 status.

## [2026-07-06 pm9] Sesame Adopts `brrtrouter::http` End-to-End

### Summary

Migrated all Sesame-IDAM outbound HTTP to `brrtrouter::http` via `sesame_common::http` re-export. Removed direct `may_http` usage from service/common code.

### Changes

**BRRTRouter (sibling):** Added `fetch_post` to `brrtrouter::http`; POST integration test in `http_fetch_tests.rs`.

**sesame-idam:**
- `sesame-common/src/http.rs` — re-exports `fetch_get`, `fetch_post`, `fetch_get_text_with_retry`, options/errors; unit tests for GET+POST
- `jwks_cache/cache.rs` — uses `crate::http::fetch_get` (HTTPS via rustls in BRRTRouter layer)
- `authz_client.rs` — uses `sesame_common::fetch_post` for authz-core enrichment
- Removed `may_http` from `sesame-common` and `identity-login-service` impl `Cargo.toml`
- Updated `topic-http-client-policy.md` — canonical path is `sesame_common::http` → `brrtrouter::http`

### Test coverage

- `sesame-common/src/http.rs` — live mock-server GET + POST tests
- Existing `authz_enrichment` BDD + `authz_client` parse_roles unit tests unchanged
- BRRTRouter `http_fetch_tests.rs` — added `fetch_post_http_sends_json_body`

## [2026-07-06 pm10] Core Session Flows — Refresh, Discovery, Userinfo, Logout

### Summary

Implemented hauliage-blocking session/auth flows on top of existing login/register work. Token refresh now issues real Ed25519 JWTs (same key as login); OIDC discovery returns a populated document; userinfo and logout are DB/Redis-backed.

### Changes

**identity-session-service:**
- `services/token_issuer.rs` — shared `Ed25519Signer`; signs access + refresh JWTs on rotation
- `services/token_rotation.rs` — removed placeholder RS256 tokens; signed refresh JWT (was raw JSON); family set registration
- `services/discovery.rs` + `controllers/openid_configuration.rs` — env-driven OIDC discovery (`SESAME_JWT_ISSUER`, `SESAME_IDAM_PUBLIC_URL`)
- `controllers/auth_refresh.rs` — correct `user_id` + `scope` in success response; wired in `main.rs`
- `controllers/oauth_userinfo.rs` — raw handler, DB-backed profile (same path as `/identity/me`)
- `main.rs` — registers `auth_refresh`, `openid_configuration`, `oauth_userinfo`; eager `SIGNER` init

**identity-login-service:**
- `controllers/auth_logout.rs` — revokes refresh token family in Redis; wired in `main.rs`
- `redis.rs` — `revoke_refresh_token()` for logout
- `models/refresh_token.rs` — `from_json()` for logout lookup

### Open

- In-cluster login 401 ("Security provider not found") still needs deploy smoke (H6.1)
- Refresh error paths still return typed 200 with empty tokens (OpenAPI/typed-handler limitation)
- Full E2E: register → login → refresh → userinfo → logout (H7.1)

## [2026-07-06 pm11] H7.1 Token Lifecycle BDD + ms02 Test Run

### Summary

Added cross-service BDD tests for the hauliage P1 gate (H7.1) and verified them on **ms02** against live Kind postgres/redis.

### Tests added

**identity-login-service** `tests/bdd/token_lifecycle.rs`:
- `full_token_lifecycle_register_userinfo_refresh_logout` — register → userinfo → login → refresh → reuse rejected → logout → refresh rejected
- `register_token_response_matches_hauliage_contract` — H2.5 TokenResponse shape (EdDSA, Bearer, expires_in, refresh fields)

**identity-session-service** `tests/bdd/oidc_discovery.rs`:
- `openid_configuration_returns_populated_document`

### ms02 verification (canonical)

```bash
ssh ms02 'source ~/.cargo/env && cd ~/Workspace/microscaler/seasame-idam/microservices && \
  cargo test -p sesame_idam_identity_login_service --test main_bdd token_lifecycle -- --nocapture'
```

All 3 tests **passed** on ms02 (~0.6s with postgres/redis on localhost:5432/6379). Mac without port-forwards skips gracefully.

## [2026-07-06 pm12] Cross-repo BRRTRouter refactor backlog (workaround cleanup)

### Summary

Documented BRRTRouter refactor tasks needed to remove sesame-idam workarounds across three repos. Next staged work: **BR-1** (`security: []` semantics) → **SI-1/SI-2** (restore global security on IDAM specs) → **HI-1** (H7.2 hauliage JWKS smoke).

### Wiki pages

| Repo | Page |
|------|------|
| sesame-idam | [`topics/topic-brrtrouter-refactor-backlog.md`](./topics/topic-brrtrouter-refactor-backlog.md) |
| BRRTRouter | [`../../BRRTRouter/docs/llmwiki/topics/sesame-idam-workarounds-cleanup.md`](../../BRRTRouter/docs/llmwiki/topics/sesame-idam-workarounds-cleanup.md) |
| hauliage | [`../../hauliage/docs/llmwiki/topics/sesame-idam-brrtrouter-integration.md`](../../hauliage/docs/llmwiki/topics/sesame-idam-brrtrouter-integration.md) |

Also updated `topic-brrtrouter-codegen.md` (security + raw-handler gaps) and closed Open item in `topic-http-client-policy.md`.

## [2026-07-06 pm13] BR-1 security:[] fix + restore global OpenAPI security

### Summary

Implemented **BR-1** in BRRTRouter: explicit operation `security: []` is now public; omitted security inherits global. Restored global `BearerAuth` + `ApiKeyHeader` on login/session specs (**SI-1**). Added regression tests (**SI-2**).

### BRRTRouter

- `src/spec/security_presence.rs` — `extract_operation_security_presence`, `resolve_operation_security`
- `build_routes_with_security_presence` wired from `load_spec` / `load_spec_full`
- `tests/spec_security_tests.rs` — fixture-based inheritance tests

### Sesame-IDAM

- Global `security` re-added to `identity-login-service` and `identity-session-service` OpenAPI specs
- `identity-login-service/impl/tests/openapi_security.rs` — loads real specs, asserts public vs protected routes

### Verification (ms02)

```bash
cargo test --test spec_security_tests          # BRRTRouter — 3 passed
cargo test -p sesame_idam_identity_login_service --test openapi_security  # 4 passed
cargo test -p hauliage_fleet --test sesame_jwks_smoke  # skips until fleet redeploy + demo seed
```

## [2026-07-06 pm14] Hauliage HI-2/HI-1/HI-3 — fleet JWKS integration

Cross-repo: hauliage fleet wired to sesame-idam cluster JWKS (`_sesame-idam-kubernetes.yaml` + fleet helm/config). `GET /vehicles` requires BearerAuth. Smoke test `hauliage/microservices/fleet/impl/tests/sesame_jwks_smoke.rs` exercises login → fleet (curl, skips when services/seeds unavailable).

## [2026-07-06 pm22] E2E green + JWKS response fix

- Session `JwkOnly` now serializes `alg: EdDSA` (fixes JWKS 500 from response schema validation).
- Tiltfile: `BRRTRouter/src` in microservice build deps; `build-tooling` as build dep.
- ms02: `sesame_jwks_smoke` 3/3 pass; manual login → fleet 200 verified.

## [2026-07-06 pm22] BRRTRouter HTTP fetch path fix

`fetch_get_http` / `fetch_post_http` now send path+query only to `may_http` (not full URL). Unblocks in-cluster JWKS fetch from hauliage fleet pods.

## [2026-07-07] Hauliage HI-5 + JWKS company/BFF

Cross-repo: hauliage identity/company/fleet global BearerAuth; company + BFF JWKS config; BFF login (HI-4) committed (`16fae98`). BRRTRouter `73744df`, sesame `ced7b92`.

**Wave 2 staged:** HI-7 pin → redeploy company/bff → smoke extend → consignments JWKS → OpenAPI client. See hauliage [`sesame-idam-brrtrouter-integration.md`](../../hauliage/docs/llmwiki/topics/sesame-idam-brrtrouter-integration.md).

## [2026-07-07] Wave 2 complete (hauliage)

HI-7 verified, company smoke 2/2, consignments JWKS, BFF login 200 after 30s fetch timeout. Wave 3: OpenAPI client, fleet ownership, BR-2/BR-3.

## [2026-07-08] ingest | account-first onboarding checkpoint (Sesame side)

Consumer org lifecycle + JWT `org_id` implemented; E2E with Hauliage BFF paused for BRRTRouter stub lifecycle work.

- **New wiki:** [`topics/topic-account-first-onboarding-checkpoint.md`](./topics/topic-account-first-onboarding-checkpoint.md) — identity-login `set_active_organization`, org-mgmt consumer API, ports, protected controllers.
- **Cross-repo:** Hauliage [`account-first-onboarding-checkpoint.md`](../../hauliage/docs/llmwiki/topics/account-first-onboarding-checkpoint.md); BRRTRouter [`PRD_IMPL_CONTROLLER_LIFECYCLE.md`](../../BRRTRouter/docs/PRD_IMPL_CONTROLLER_LIFECYCLE.md).
- **`index.md`:** registered checkpoint page (pending manual index row if table format requires edit).

## [2026-07-10] refactor | SI-3 — typed handlers for /identity/me + userinfo (BR-2)

Removed `raw_handler.rs` workaround now that `TypedHandlerRequest` carries `jwt_claims`.

- **Controllers:** `users_me_get`, `users_me_patch`, `oauth_userinfo` use `auth_context::authenticated_principal` + typed `handle`.
- **Dispatch:** `impl/src/main.rs` registers all three via `spawn_typed_with_stack_size_and_name`.
- **Tests:** `users_me_db.rs` + `token_lifecycle.rs` call typed handlers via `TypedHandlerRequest::from_handler`; BDD fixtures add `jwt_claims: None` where structs are built manually (BR-2 field).
- **Build:** workspace `may_minihttp` path pin + lockfile regen (local fork `client` feature).

Next: **SI-4** (auth_refresh OAuth 401) after **BR-3**.

## [2026-07-10] fix | SI-4 — OAuth-compliant auth_refresh errors (BR-3)

`identity-session-service/impl/src/controllers/auth_refresh.rs` returns `HttpJson<serde_json::Value>`:

- **200** — rotated token pair (`TokenResponse` JSON)
- **401** — `invalid_grant` for invalid/reused refresh tokens
- **500** — Redis unavailable during rotation

**Tests:** `identity-login-service/impl/tests/bdd/token_lifecycle.rs` asserts `status == 401` on reuse and post-logout refresh.

OpenAPI already had `401` + `ErrorResponse` schema on `/session/refresh` (BR-3 trigger for new stubs).

## [2026-07-10] feat | Wave A — typed active-organization + account-first BDD

- **Staging:** [`docs/audit/first-delivery-wave-a.md`](../audit/first-delivery-wave-a.md) — D1–D3 delivery backlog (A1–A9).
- **A2:** `identity-login-service` — `auth_context.rs`; `set_active_organization` typed handler + `jwt_claims` (no base64 decode); `main.rs` uses typed dispatch.
- **A5:** `tests/bdd/account_first_onboarding.rs` — org_id in JWT + 403 for non-member.
- **BRRTRouter:** `ImplControllerStubParams.uses_http_json` field fix (unblocks workspace compile during BR-3 work).

Enriched [`docs/audit/epic-delivery-audit-2026-07-10.md`](../audit/epic-delivery-audit-2026-07-10.md) with delivery-tier model (D0–D6), stub vs impl matrix, cross-repo/platform gaps (k8s `:8080`, BR-3/SI-4, account-first E2E), and Waves A–D priority roadmap. Noted SI-3 merged (§7.1 done); `set_active_organization` remains raw-handler outlier.

## [2026-07-12] verify | Wave A redeploy + backlog reconciliation

- **Tilt (ms02, port 10351):** triggered `build`/`docker`/`deploy` for `identity-login-service` and `identity-session-service`.
- **Compile:** `cargo check` for login + session services — clean.
- **BDD:** `account_first` (2) and `users_me` (6) pass with **SKIP** (Postgres/Redis unreachable).
- **Blocker resolved:** shared-k8s cluster healthy; use `KUBECONFIG=../shared-k8s-cluster/kubeconfig/shared-k8s.yaml` (default kubectl context unset).
- **Port-forward:** postgres + redis from namespace `data` (not `sesame-idam` for redis).
- **Live BDD (all pass):** `account_first` 2/2, `token_lifecycle` 2/2, `users_me` 6/6, `api_keys` 14/14.
- **Fix:** `api_key_flow.rs` — add `jwt_claims: None` to `TypedHandlerRequest` (BR-2 field).
- **Backlog update:** [`first-delivery-wave-a.md`](../audit/first-delivery-wave-a.md) — A7 seeds ✅, A9 validate+BDD ✅, A8 helm/database-env 🟡, A6 Playwright spec 🟡 (`REAL_LOGIN=1`).
- **A6 green:** Hauliage Playwright `real_account_first_onboarding.spec.ts` — register → onboarding → `POST /api/v1/organizations/me` → shipper dashboard (4.1s). Required frontend pod restart (stuck nginx `/api/v1/` proxy).
- **Uncommitted:** Wave A sesame-idam code + audit docs on ms02 working tree.

## [2026-07-13] chore | consume merged may_minihttp native client

- Replaced the local `may_minihttp` path dependency and crates.io patch with the Microscaler `integration/microscaler-fork` Git branch.
- Removed the retired `may_http` dependency from the active workspace; retained the HTTP 0.2 alias required by the native client API.
- Reconciled [`topic-http-client-policy.md`](./topics/topic-http-client-policy.md) and `AGENTS.md` with the BRRTRouter → `may_minihttp::client` stack.
- `Cargo.lock` resolves merge commit `6041097`; login and session service checks pass on ms02.

## [2026-07-13] fix | delivered-code audit and quality gates

- Corrected refresh observability: removed the pre-validation `JwtIssued/allowed` audit event (which falsely marked failed refreshes as successful and duplicated successful events) and declared the recorded `result` span field.
- Removed invitation capability tokens from org-mgmt info logs; logs retain only `invite_id`, `org_id`, and email delivery context.
- Restored `just lint-rust`: fixed pedantic findings, scoped unavoidable generated-code allowances, and updated stale `TypedHandlerRequest` test fixtures for `jwt_claims`.
- Updated all shared `AccessClaims` fixtures for the delivered optional `org_id` claim; focused `sesame-common` run passes 367/367.
- Serialized handler-level live-Postgres BDD modules through the existing Nextest mutex group after API-key scenarios exposed connection-contention retries.
- Verification on ms02: `cargo check --workspace` passes; `just lint-rust` passes; workspace Nextest passes 907/907 with retries disabled.

## [2026-07-14] feat | first-class RLS without SesameExecutor

- Accepted [`ADR-005-first-class-rls-contract.md`](../ADR-005-first-class-rls-contract.md): RLS is an optional capability of Lifeguard's existing executors, not a new wrapper hierarchy.
- Expanded Lifeguard `SessionContext` to the mandatory tenant, subject, active organization,
  session, roles, and permissions contract, with optional user/org classifications.
- Added `LifeguardPool::with_session_transaction` over the existing pinned primary executor.
- Published `sql/rls/v1/install.sql` plus executable Hauliage reference policies with locked
  `search_path`, `SECURITY INVOKER`, transaction-local GUCs, and explicit grants.
- Lifeguard verification on ms02: 485 unit tests pass; RLS integration suite 19/19 passes,
  including commit, rollback, application error, panic, missing helper, and pool-slot reuse.
- Consumer validation corrected the tenant field to opaque text (`hauliage`), matching Sesame's
  delivered access tokens; subject and active organization remain UUIDs.

## [2026-07-14] verify | live Hauliage RLS path and session stability

- Applied the existing Sesame migrations/seeds to the shared stack after login exposed the missing
  platform-tenant registry.
- Verified real login/JWKS → Hauliage BFF → Company → forced PostgreSQL RLS for both seeded
  personas without printing tokens: shipper resolved only AME Corp; transporter resolved only
  Transport Services.
- Closed H1.5/H7.4 and corrected active roadmap references from the rejected `SesameExecutor`
  wrapper to Lifeguard's base-executor contextual transaction capability.
- Diagnosed repeated session-service liveness kills as a May coroutine stack overflow on the
  three-middleware JWKS path. Raised only identity-session-service from 32 KiB to 64 KiB and
  verified the rendered Helm value plus a 50-request/10-concurrent JWKS burst (50 HTTP 200s).
