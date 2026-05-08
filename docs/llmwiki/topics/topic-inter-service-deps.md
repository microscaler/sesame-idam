---
title: Inter-Service Dependencies
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, service-topology-design.md]
---

# Inter-Service Dependencies

## The Only Cross-Service Dependency

```
identity-login-service → authz-core (POST /principal/effective)
```

This call happens **once at login time** to enrich JWT claims. After the JWT is issued, it is self-contained.

## Fully Independent Services

The following services have NO cross-service dependencies:
- **identity-session-service** — Handles refresh, OIDC, JWKS independently
- **identity-user-mgmt-service** — Handles user CRUD, MFA independently
- **api-keys** — Handles M2M key validation independently
- **org-mgmt** — Handles org lifecycle independently

## Why This Matters

1. **Failure isolation.** A crash in identity-session-service doesn't affect login or authz.
2. **Independent deployment.** Services can ship on different cycles.
3. **Independent scaling.** Scale only the services under load.

## Code Anchors

- `microservices/idam/identity-login-service/impl/src/` — Only service with outbound HTTP calls to other Sesame services
- `openapi/identity-login-service/openapi.yaml` — Auth flow endpoints

## Gaps / Drift

> **Open:** Verify no hidden cross-service dependencies exist in the actual implementation.
