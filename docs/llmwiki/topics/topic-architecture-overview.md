---
title: Architecture Overview
status: verified
updated: 2026-05-14
sources: [PRD-SEASAME-AUDIT-REMEDIATION.md, design-doc.md, service-topology-design.md, sesame-idam-complete.md]
---

# Architecture Overview

## Six Independent Rust Microservices

Sesame-IDAM is NOT a monolith. It is **six independent services** split by access frequency and per-request cost. Total: **133 endpoints, 26 tags**.

> **Note:** Endpoint count was 119, updated to 133 per PRD-SEASAME-AUDIT-REMEDIATION.md. The existing wiki pages (entities, topics, references) were based on the old count and should be re-verified against the current OpenAPI specs.

| Service | Port | Frequency | Cost | Endpoints | Responsibility |
|---------|------|-----------|------|-----------|----------------|
| **identity-login-service** | 8101 | HIGH | Medium-High | 20 | Login, register, social OAuth, OTP, passwordless, dual OTP, signup validation |
| **identity-session-service** | 8105 | HIGH | Low | 13 | Token refresh, OIDC discovery, JWKS, step-up MFA, impersonation, direct token, MCP |
| **identity-user-mgmt-service** | 8106 | MEDIUM | Medium | 25 | User CRUD, MFA, email/phone, social, migrations, password clearing |
| **authz-core** | 8102 | EXTREME | Low-Medium | 4 | Per-request authorization checks |
| **api-keys** | 8103 | HIGH | Low | 10 | M2M key management/validation |
| **org-mgmt** | 8104 | LOW | High | 34 | Org lifecycle, SSO/SCIM, webhooks, application RBAC |

## Why Six Services

1. **Different access patterns demand different scales.** Login handles bursts, refresh handles steady state, authorize handles every API call.
2. **Different per-request costs.** Password hashing is expensive, JWT verification is cheap, org SSO setup takes seconds.
3. **Failure domains are isolated.** A login outage doesn't affect session refresh or authorization.
4. **Independent deployment cycles.** OTP flows can ship without touching user management.

## Storage

- **PostgreSQL** — All persistent data (namespace `data` in shared Kind cluster)
- **Redis** — Session cache, permission cache, key validation cache (namespace `sesame-idam`)

## Workspace Crates

- **12 workspace members** — 6 gen crates + 6 impl crates
- **Shared crates:** `sesame_idam_database` (PooledLifeExecutor), `sesame_audit` (HMAC signing)
- **Codegen pattern:** gen/ (generated types + handlers) + impl/ (binary + controllers + models)
- **Naming convention:** All gen→impl package names match. See `topic-package-naming-convention.md` for details.

## The Only Cross-Service Dependency

```
identity-login-service → authz-core (POST /principal/effective at login time only)
```

After the JWT is issued, it is self-contained. All other services are fully independent.

## Code Anchors

- `microservices/idam/` — All 6 service directories (gen+impl each)
- `openapi/` — 6 OpenAPI spec directories
- `Tiltfile` — Tilt deployment config (ports 10351 for dev)
- `helm/sesame-idam-microservice/` — Helm charts for k8s deployment
- `k8s/microservices/` — Kubernetes manifests

## Gaps / Drift

> **Note:** This page is based on the design docs which may have diverged from the actual implementation. Verify against source code for accuracy.
