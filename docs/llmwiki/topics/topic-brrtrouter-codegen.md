---
title: BRRTRouter Codegen
status: partially-verified
updated: 2026-01-22
sources: [AGENTS.md, justfile, openapi/README.md]
---

# BRRTRouter Codegen

## Overview

Sesame-IDAM uses BRRTRouter for code generation from OpenAPI specs. Each microservice has:

- **gen/** — Generated types + handler traits (NEVER edit directly)
- **impl/** — Binary crate with actual implementation

## Codegen Commands

```bash
just gen                # Regenerate all 6 services
just gen-identity-login # Single service
just lint-openapi       # Lint all OpenAPI specs
```

## OpenAPI Spec Layout

```
openapi/
├── identity-login-service/openapi.yaml  # 20 endpoints: login, register, social, OTP, magic link
├── identity-session-service/openapi.yaml # 13 endpoints: refresh, OIDC, JWKS, step-up, impersonation, MCP
├── identity-user-mgmt-service/openapi.yaml # 25 endpoints: user CRUD, MFA, email/phone, migrations
├── authz-core/openapi.yaml              # 4 endpoints: authorize, principal/effective, roles, attributes
├── api-keys/openapi.yaml                # 10 endpoints: key CRUD, validation, archiving
└── org-mgmt/openapi.yaml                # 34 endpoints: orgs, membership, SSO/SCIM, RBAC, webhooks
```

**No canonical/merged spec.** Each OpenAPI spec is self-contained.

## Schema Duplication Convention

Shared schemas (User, UserProfile, Org) are **duplicated** in each consuming spec's `components/schemas`. This is intentional — each OpenAPI spec must be self-contained for BRRTRouter codegen to work.

|| Schema | Owner Service | Consumed By |
|--------|--------------|-------------|
|| User | identity-login-service | authz-core (principal endpoints) |
|| UserProfile | identity-login-service | authz-core (principal endpoints) |
|| Org | org-mgmt | api-keys (org data in validation) |

## Rules

1. **Never edit generated code** under `gen/`. Fix the OpenAPI spec instead.
2. **Every UI-dynamic field** must be in the OpenAPI spec. BRRTRouter silently drops unrecognized fields.
3. **After spec changes**, run `just gen` and verify the generated code compiles.

## Code Anchors

- `microservices/Cargo.toml` — All 12 crates registered (gen+impl for 6 services)
- `openapi/*/openapi.yaml` — Source of truth for request/response shapes
- BRRTRouter sibling repo: `../BRRTRouter`

## Gaps / Drift

> **Open:** Verify actual codegen output matches the OpenAPI specs in the current state.
