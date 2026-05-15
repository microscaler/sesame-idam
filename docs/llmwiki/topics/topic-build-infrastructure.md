---
title: Build Infrastructure
status: verified
updated: 2026-05-15
sources: [PRD-SEASAME-AUDIT-REMEDIATION.md, hauliage patterns, actual cargo check output]
---

# Build Infrastructure

## Current State (Working)

Sesame-IDAM compiles (`cargo check --workspace` passes) with stub implementations. All Phase 2 items now completed.

### Build Status

| Item | Status |
|------|--------|
| `cargo check --workspace` | ✅ 0 errors, 6 warnings (all pre-existing dead_code) |
| `cargo test --workspace` | ✅ 4 tests (unit + doc) |
| `brrtrouter-gen lint` | ✅ All specs pass |
| `build.rs` per service | ✅ Added — entity discovery via lifeguard-migrate |
| `config/service.yaml` per service | ⏳ Phase 2 |
| `services/` layer | ⏳ Phase 2 |
| `tests/` BDD suite | ⏳ Phase 3 |
| `seeds/` | ⏳ Phase 3 |

### Build Warnings (Current)

- 6 `dead_code` warnings for `EMITTER` static in `impl/src/audit.rs` — expected, stub controllers don't call audit yet.
- `non_snake_case` warnings in generated modules are from OpenAPI endpoint names (e.g., `listAuditEvents` → `list_audit_events`). Not a blocker.

## Phase 2 Items (Completed)

### 2.1 `build.rs` Per Service ✅

Reference: `hauliage/company/impl/build.rs`

- Reads entity models from `src/models/` via `lifeguard_migrate::build_script::discover_entities()`
- Generates `OUT_DIR/entity_registry.rs` containing entity metadata and `generate_sql_for_all()` function
- Enables `cargo run -p <migrator>` for migration generation
- Requires `[build-dependencies] lifeguard-migrate = { workspace = true }` in each impl Cargo.toml

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
