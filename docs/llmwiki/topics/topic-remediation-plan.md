---
title: Remediation Plan
status: unverified
updated: 2026-05-14
sources: [PRD-SEASAME-AUDIT-REMEDIATION.md]
---

# Sesame-IDAM Structural Remediation Plan

## Overview

Sesame-IDAM compiles and tests pass, but `brrtrouter client build` fails across all services due to package naming mismatches. Additionally, several structural conventions from Hauliage are missing for production readiness.

## Endpoints

Total: **133 endpoints** across 6 services (up from original 119):

| Service | Port | Endpoints | Access Pattern |
|---------|------|-----------|----------------|
| identity-login-service | 8101 | 20 | HIGH — login, register, OAuth, OTP |
| identity-session-service | 8105 | 13 | HIGH — refresh, OIDC, JWKS |
| identity-user-mgmt-service | 8106 | 25 | MEDIUM — user CRUD, MFA |
| authz-core | 8102 | 4 | EXTREME — principal/effective |
| api-keys | 8103 | 10 | HIGH — key validation |
| org-mgmt | 8104 | 34 | LOW — org lifecycle |

## What's Already Working

- [x] 6 OpenAPI specs with `X-Tenant-ID` header on all 131/133 operational endpoints
- [x] 47 entity model files across 6 services (Lifeguard ORM derive macros)
- [x] `sesame_idam_database` crate with `PooledLifeExecutor` pattern
- [x] `sesame-audit` crate with HMAC event signing
- [x] Workspace compiles: `cargo check --workspace` — 0 errors
- [x] Tests pass: 4 unit tests, 1 doc test
- [x] Stub controllers for all 133 endpoints (placeholder responses)

## Build Warnings (Current)

- 26 `non_snake_case` warnings in generated modules — expected from OpenAPI endpoint names
- 5 `dead_code` warnings for `EMITTER` static — expected, stub controllers don't call audit yet

---

## Phase 0: Tiltfile Rewrite (Parallel with Phase 1)

**Status: ✅ Completed**

All Phase 0 subtasks completed. Tiltfile uses hardcoded service discovery (no blob parsing), correct `openapi/idam/` paths, and delegates to sesame-idam CLI shim for gen/build.

| Step | Status | Notes |
|------|--------|-------|
| 0a. `bff-suite-config.yaml` | ✅ | 6 services + ports |
| 0b. `docker/microservices/Dockerfile.template` | ✅ | hauliage pattern |
| 0c. `docker/base/Dockerfile` + `dev-entrypoint.sh` | ✅ | Alpine 3.23 + hot-reload |
| 0d. Helm values cleanup | ✅ | Removed stale files, fixed typos |
| 0e. Tiltfile rewrite | ✅ | 305 lines, hardcoded discovery |
| 0f. Validation | ⏳ | Pending `tilt up` |

## Phase 1: Fix Package Naming (CRITICAL)

**Status: ✅ Completed**

All gen crate package names were manually corrected to match what impl crates expect.

### Changes Made

| Service | Before (gen name) | After (gen name) |
|---------|-------------------|------------------|
| api-keys | `api_key_service` | `api_keys_service_api` |
| authz-core | `authorization_core_service__authz_core` | `authz_core_service_api` |
| identity-login | `login_service` | `identity_login_service_service_api` |
| identity-session | `session_service` | `identity_session_service_service_api` |
| identity-user-mgmt | `user_management_service` | `identity_user_mgmt_service_service_api` |
| org-mgmt | `organization_management_service` | `org_mgmt_service_api` |

### Result

`cargo check --workspace` now passes with 0 errors. Only warnings are expected:
- 26 `non_snake_case` warnings in generated modules (from OpenAPI endpoint names)
- 5 `dead_code` warnings for `EMITTER` static (stub controllers don't call audit yet)

## Phase 2: Add Build Infrastructure (MODERATE)

**Status:** Not started

### 2.1 Add `build.rs` to each impl crate

- Reads entity models from `src/models/`
- Generates migrations via lifeguard-migrate
- Creates `migrations/` output directory

### 2.2 Add `config/service.yaml` to each impl crate

- CORS configuration
- Security provider configuration
- HTTP server configuration (address, port, timeouts)
- Database pool configuration

### 2.3 Add `services/` layer to each impl crate

- Controllers call services (not database directly)
- Services implement business logic
- Services receive `PooledLifeExecutor` for DB access

## Phase 3: Add Supporting Files (MINOR)

**Status:** Not started

### 3.1 Add `org_resolution.rs` to each impl service

- Tenant/org ID resolution from request context

### 3.2 Add `tests/` directory with BDD test skeleton

- Minimal smoke test for at least one endpoint per service

### 3.3 Add `seeds/` directory with development seed data

- Seed data for local development/testing

## Phase 4: Workspace Cleanup

**Status:** Not started

### 4.1 Rename database crate from `database` to `sesame_idam_database`

- Update `Cargo.toml` name field
- Update all dependency references across 6 impl crates
- Update workspace member path if needed

### 4.2 Add `may_postgres` `[patch]` to workspace `Cargo.toml`

## Phase 5: Tiltfile Validation & Data Wiring

**Status:** Not started

5a. Run `tilt up` and verify all 6 services start
5b. Verify port forwards work (postgres 5432, redis 6379)
5c. Verify live_update sync (edit source → save → service restarts)
5d. Create database env manifests (secrets/configmaps for postgres)
5e. Wire Redis deployment via K8s manifest

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Breaking existing stub controllers | Low | Low | Phase 1 is purely mechanical string replacement |
| Breaking database crate references | Low | Medium | All references are in Cargo.toml — grep finds them all |
| Misnaming gen dependency paths | Low | Medium | Use find/grep to audit all path = "../gen" references |
| Build.rs conflicts with existing models | Low | Low | Follow hauliage pattern exactly |
| Config.yaml format incompatibility | Low | Low | Follow hauliage format exactly |
| **Tiltfile rewrite breaks Tilt startup** | Medium | High | Validate Tiltfile with `tilt lint` or dry-run |
| **Missing Dockerfile template breaks image builds** | High | High | Create template from hauliage's pattern, adapt paths |
| **Helm charts missing breaks K8s deployments** | High | High | Create minimal Helm chart or use k8s_yaml with templates |

## Acceptance Criteria

- [ ] `cargo check --workspace` passes with 0 errors
- [ ] `cargo test --workspace` passes (existing 4 + 1 doc test)
- [ ] `brrtrouter client build` succeeds for all 6 services
- [ ] No `non_snake_case` warnings introduced (existing ones are from gen code)
- [ ] No dead_code warnings for new code
- [ ] All gen→impl dependency paths resolve correctly
- [ ] Package naming follows `sesame_idam_<service>_gen` / `sesame_idam_<service>` convention
- [ ] `sesame_idam_database` crate properly renamed and referenced

## Open Questions

1. **Should `sesame_idam_database` be moved to workspace root like hauliage's `hauliage_database`, or kept nested?**
   - Hauliage: `hauliage_database/` at workspace root
   - Sesame: `database/` at workspace root (already correct position)
   - Only naming change needed: `database` → `sesame_idam_database`

2. **Does `sesame-audit` need to be moved into the `idam/` subdirectory for consistency?**
   - Currently at `microservices/sesame-audit/` (at workspace root)
   - Hauliage's equivalent (`email_reminder_worker`) is at `microservices/` root
   - Current position is fine

3. **Should we consolidate the 6 services under `microservices/idam/` into `microservices/` directly?**
   - Hauliage puts services directly under `microservices/` (no `hauliage/` prefix dir)
   - Sesame uses `microservices/idam/{service}/` pattern
   - This is a cosmetic difference — does not affect build or naming

## Out of Scope

- Implementing actual business logic in stub controllers (separate PRD)
- OpenAPI spec changes (specs are correct, only package naming is wrong)
- Hauliage changes (this only affects sesame-idam)
- Authentication/authorization implementation
- Database migration content (only build.rs for generation, not the SQL)

## Code Anchors

- `PRD-SEASAME-AUDIT-REMEDIATION.md` — Full remediation document
- `brrtrouter-workspace-architecture` skill — Workspace patterns and pitfalls
