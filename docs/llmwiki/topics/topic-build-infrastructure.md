---
title: Build Infrastructure
status: verified
updated: 2026-05-14
sources: [PRD-SEASAME-AUDIT-REMEDIATION.md, hauliage patterns]
---

# Build Infrastructure

## Current State (Working)

Sesame-IDAM compiles (`cargo check --workspace` passes) with stub implementations. Several hauliage conventions are still missing for production readiness.

### Build Status

| Item | Status |
|------|--------|
| `cargo check --workspace` | ✅ 0 errors, 31 warnings |
| `cargo test --workspace` | ✅ 5 tests (4 unit + 1 doc) |
| `brrtrouter-gen lint` | ✅ All specs pass (fixed path refs) |
| `build.rs` per service | ⏳ Phase 2 |
| `config/service.yaml` per service | ⏳ Phase 2 |
| `services/` layer | ⏳ Phase 2 |
| `tests/` BDD suite | ⏳ Phase 3 |
| `seeds/` | ⏳ Phase 3 |

### Build Warnings (Current)

- 26 `non_snake_case` warnings in generated modules (authz-core, identity-user-mgmt) — expected from OpenAPI endpoint names (e.g., `listAuditEvents` → `list_audit_events`). Not a blocker.
- 5 `dead_code` warnings for `EMITTER` static in `impl/src/audit.rs` — expected, stub controllers don't call audit yet.

## Phase 2 Items (Planned)

### 2.1 `build.rs` Per Service

Reference: `hauliage/company/impl/build.rs`

- Reads entity models from `src/models/`
- Generates migrations via lifeguard-migrate
- Creates `migrations/` output directory

### 2.2 `config/service.yaml` Per Service

Reference: `hauliage/company/impl/config/service.yaml`

- CORS configuration
- Security provider configuration
- HTTP server configuration (address, port, timeouts)
- Database pool configuration

### 2.3 `services/` Layer Per Service

Reference: `hauliage/company/impl/src/services/`

- Controllers call services (not database directly)
- Services implement business logic
- Services receive `PooledLifeExecutor` for DB access

## Code Anchors

- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 3.2` — Detailed gap analysis
- `hauliage/company/impl/` — Reference implementation
