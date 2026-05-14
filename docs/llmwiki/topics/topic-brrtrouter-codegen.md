---
title: BRRTRouter Codegen
status: verified
updated: 2026-05-14
sources: [AGENTS.md, justfile, openapi/README.md, PRD-SEASAME-AUDIT-REMEDIATION.md]
---

# BRRTRouter Codegen

## Overview

Sesame-IDAM uses BRRTRouter for code generation from OpenAPI specs. Each microservice has:

- **gen/** — Generated types + handler traits (NEVER edit directly)
- **impl/** — Binary crate with actual implementation

Package naming is now correct (see `topic-package-naming-convention.md`). All 12 gen crates match their impl dependency names.

## Codegen Commands

```bash
just gen                # Regenerate all 6 services
just gen-identity-login # Single service
just lint-openapi       # Lint all OpenAPI specs
```

## Codegen Recipes (justfile)

Each recipe delegates to brrtrouter-gen:

```bash
# Codegen
cd BRRTRouter && cargo run --bin brrtrouter-gen -- generate \
  --spec $(pwd)/openapi/idam/<service>/openapi.yaml \
  --output $(pwd)/microservices/idam/<service>/gen \
  --package-name <service>_service_api \
  --force

# Lint
cd BRRTRouter && cargo run --bin brrtrouter-gen -- lint \
  --spec $(pwd)/openapi/idam/<service>/openapi.yaml \
  --fail-on-error

# Serve (echo handlers)
cd BRRTRouter && cargo run --bin brrtrouter-gen -- serve \
  --spec $(pwd)/openapi/idam/<service>/openapi.yaml \
  --addr <addr>
```

> **NOTE:** The `--package-name` values use `<service>_service_api` convention, which matches the corrected gen crate package names (Phase 1 completed).

## OpenAPI Spec Layout

```
openapi/idam/
├── identity-login-service/openapi.yaml  # 20 endpoints: login, register, social, OTP, magic link
├── identity-session-service/openapi.yaml # 13 endpoints: refresh, OIDC, JWKS, step-up, impersonation, MCP
├── identity-user-mgmt-service/openapi.yaml # 25 endpoints: user CRUD, MFA, email/phone, migrations
├── authz-core/openapi.yaml              # 4 endpoints: authorize, principal/effective, roles, attributes
├── api-keys/openapi.yaml                # 10 endpoints: key CRUD, validation, archiving
└── org-mgmt/openapi.yaml                # 34 endpoints: orgs, membership, SSO/SCIM, RBAC, webhooks
```

**Spec path:** `openapi/idam/<service>/openapi.yaml` (nested under `idam/`), not `openapi/<service>/openapi.yaml`.

**No canonical/merged spec.** Each OpenAPI spec is self-contained.

**Total: 133 endpoints across 6 specs** (updated from 119 per PRD).

## Schema Duplication Convention

Shared schemas (User, UserProfile, Org) are **duplicated** in each consuming spec's `components/schemas`. This is intentional — each OpenAPI spec must be self-contained for BRRTRouter codegen to work.

| Schema | Owner Service | Consumed By |
|--------|--------------|-------------|
| User | identity-login-service | authz-core (principal endpoints) |
| UserProfile | identity-login-service | authz-core (principal endpoints) |
| Org | org-mgmt | api-keys (org data in validation) |

## Rules

1. **Never edit generated code** under `gen/`. Fix the OpenAPI spec instead.
2. **Every UI-dynamic field** must be in the OpenAPI spec. BRRTRouter silently drops unrecognized fields.
3. **After spec changes**, run `just gen` and verify the generated code compiles.

## Current Codegen State

| Item | Status |
|------|--------|
| `brrtrouter-gen lint` | ✅ All 6 specs pass |
| `cargo check --workspace` | ✅ 0 errors |
| Package naming | ✅ Phase 1 completed |
| OpenAPI spec layout | ✅ `openapi/idam/` nesting |

## Code Anchors

- `microservices/Cargo.toml` — All 12 crates registered (gen+impl for 6 services)
- `openapi/idam/*/openapi.yaml` — Source of truth for request/response shapes
- BRRTRouter sibling repo: `../BRRTRouter`
