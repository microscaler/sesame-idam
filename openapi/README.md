# Sesame-IDAM OpenAPI Specifications

> Four independent services, each with its own OpenAPI spec.
> Date: 2026-05-02 (updated)

## Service Map

| Service | Directory | Spec Files | Base Path | Port |
|---------|-----------|------------|-----------|------|
| **identity-auth** | `openapi/identity-auth/` | `openapi.yaml` (canonical), `identity-login-service/openapi.yaml`, `identity-session-service/openapi.yaml`, `identity-user-mgmt-service/openapi.yaml` | `/auth/*`, `/.well-known/*` | 8101 |
| **authz-core** | `openapi/authz-core/` | `openapi.yaml` | `/api/v1/am/authorize`, `/api/v1/am/principal/*` | 8102 |
| **api-keys** | `openapi/api-keys/` | `openapi.yaml` | `/api/v1/am/api-keys/*` | 8103 |
| **org-mgmt** | `openapi/org-mgmt/` | `openapi.yaml` | `/orgs/*`, `/api/v1/am/applications/*` | 8104 |

## identity-auth (combined + sub-specs)

The identity-auth service has a **canonical combined spec** (`identity-auth/openapi.yaml`) that serves as the single source of truth for BRRTRouter codegen. For convenience, the same API is split into 3 independent sub-specs that duplicate schemas from the parent:

| Sub-spec | File | Endpoints |
|----------|------|-----------|
| **identity-login-service** | `identity-login-service/openapi.yaml` | `/auth/login`, `/auth/register`, `/auth/login/oauth/github`, `/auth/token/exchange`, `/auth/password/reset/*`, `/auth/mfa/verify` |
| **identity-session-service** | `identity-session-service/openapi.yaml` | `/auth/refresh`, `/.well-known/openid-configuration`, `/.well-known/jwks.json`, `/auth/logout` |
| **identity-user-mgmt-service** | `identity-user-mgmt-service/openapi.yaml` | `/api/v1/identity/users`, `/api/v1/identity/users/me`, `/api/v1/identity/users/lookup`, email/phone verify |
| **combined** | `identity-auth/openapi.yaml` | Full combined spec for reference/codegen (all identity-auth endpoints) |

> **Note:** The combined spec (`identity-auth/openapi.yaml`) is the one fed to BRRTRouter codegen. The sub-specs are independent, self-contained copies for navigation — they do not lint independently (BRRTRouter does not yet support sub-spec linting).

## Directory Structure

```
openapi/
├── README.md                              # This file
├── identity-auth/                         # identity-auth service (canonical spec)
│   └── openapi.yaml                       # Full combined spec (all endpoints) — feeds brrtrouter-gen
├── identity-login-service/                # Sub-spec: login, register, social, token exchange
│   └── openapi.yaml                       # Self-contained copy (schemas duplicated from parent)
├── identity-session-service/              # Sub-spec: token refresh, OIDC, JWKS
│   └── openapi.yaml                       # Self-contained copy (schemas duplicated from parent)
├── identity-user-mgmt-service/            # Sub-spec: user CRUD, MFA, email/phone
│   └── openapi.yaml                       # Self-contained copy (schemas duplicated from parent)
├── authz-core/                            # authz-core service
│   └── openapi.yaml                       # authorize, principal/effective, roles, attributes
├── api-keys/                              # api-keys service
│   └── openapi.yaml                       # api-keys CRUD, validation
└── org-mgmt/                              # org-mgmt service
    └── openapi.yaml                       # orgs CRUD, memberships, SSO, roles, permissions, applications
```

## Building from Specs

From the repo root, with BRRTRouter available:

- `just gen` — Regenerate both identity-auth and authz gen crates
- `just gen-auth` — Regenerate identity-auth gen crate only
- `just gen-authorization` — Regenerate authz-core gen crate only
- `just lint-openapi` — Lint all specs via brrtrouter-gen
- `just sync-specs-from-brrtrouter` — Copy canonical specs from BRRTRouter

## Cross-Service Schema Sharing

Shared schemas (e.g., `User`, `Organization`, `UserProfile`) are duplicated in each consuming spec's `components/schemas`. This is intentional — each OpenAPI spec must be self-contained for BRRTRouter codegen to work.

| Schema | Owner Service | Consumed By |
|--------|--------------|-------------|
| `User` | identity-auth | org-mgmt (user snapshots), authz-core (principal endpoints) |
| `UserProfile` | identity-auth | authz-core (principal endpoints) |
| `Org` | org-mgmt | api-keys (org data in validation response) |
