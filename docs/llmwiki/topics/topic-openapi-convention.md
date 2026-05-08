---
title: OpenAPI Convention
status: partially-verified
updated: 2026-01-22
sources: [openapi/README.md, design-doc.md]
---

# OpenAPI Convention

## Spec Layout

Each service has its own OpenAPI spec under `openapi/{service}/openapi.yaml`. **No canonical/merged spec.** Each spec is self-contained.

```
openapi/
├── identity-login-service/openapi.yaml  # 20 endpoints: login, register, social, OTP, magic link
├── identity-session-service/openapi.yaml # 13 endpoints: refresh, OIDC, JWKS, step-up, impersonation, MCP
├── identity-user-mgmt-service/openapi.yaml # 25 endpoints: user CRUD, MFA, email/phone, migrations
├── authz-core/openapi.yaml              # 4 endpoints: authorize, principal/effective, roles, attributes
├── api-keys/openapi.yaml                # 10 endpoints: key CRUD, validation, archiving
└── org-mgmt/openapi.yaml                # 34 endpoints: orgs, membership, SSO/SCIM, RBAC, webhooks
```

## Schema Duplication

Shared schemas (User, UserProfile, Org) are duplicated in each spec's `components/schemas`. Each spec must be self-contained for BRRTRouter codegen.

## Rules

1. Every UI-dynamic field must be in the spec.
2. BRRTRouter silently drops unrecognized fields.
3. After spec changes, run `just gen` and verify compilation.

## Code Anchors

- `openapi/README.md` — Full spec documentation
- `openapi/*/openapi.yaml` — Source of truth for each service

## Gaps / Drift

> **Open:** Verify spec contents against actual implementation endpoints.
