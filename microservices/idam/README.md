# IDAM domain

> **Four independent services.** Driven by per-endpoint access frequency AND per-request cost.
> See `docs/service-topology-design.md` for the full analysis.

## Microservices

| Service | OpenAPI | Base Path | Port | Frequency | Cost | Responsibility |
|---------|---------|-----------|------|-----------|------|----------------|
| **identity-auth** | `openapi/identity-auth/` (4 sub-specs) | `/auth/*`, `/.well-known/*` | 8001 | HIGH | Mixed | Login, register, refresh, logout, MFA, password reset, OIDC, JWKS, user CRUD, sessions, token exchange (RFC 8693) |
| **authz-core** | `openapi/authz-core/openapi.yaml` | `/authz/authorize`, `/authz/principals/*` | 8002 | EXTREME | LOW | Per-request authorization checks, principal/effective resolution, role/permission evaluation |
| **api-keys** | `openapi/api-keys/openapi.yaml` | `/api-keys/*` | 8003 | HIGH | LOW | API key lifecycle, validation (personal + org variants), rotation, revocation |
| **org-mgmt** | `openapi/org-mgmt/openapi.yaml` | `/organizations/*`, `/applications/*` | 8004 | LOW | MEDIUM | Org/tenant CRUD, memberships, invitations, SSO/SAML/SCIM, roles, permissions, applications, webhooks |

### inter-service dependencies

```
identity-auth ──calls──→ authz-core (at login only, for JWT claim enrichment)

api-keys (independent, no calls to other services)

org-mgmt (independent, no calls to other services)
```

The **only** cross-service dependency is identity-auth calling authz-core's `/principal/effective` endpoint at login time to populate JWT claims. After the JWT is issued, it is self-contained.

### Implementation pattern

Each service follows the BRRTRouter pattern:
- `gen/` — BRRTRouter-generated crate from OpenAPI spec (controllers, handlers, registry)
- `impl/` — Binary crate depending on `gen/`, plus persistence (lifeguard) and service-specific logic

Add to `microservices/Cargo.toml` workspace members when implementing.
