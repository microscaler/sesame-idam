# LLM Wiki — Session Log

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

