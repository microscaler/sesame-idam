---
title: JWT Schema
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, service-topology-design.md]
---

# JWT Schema (Enriched JWTs)

## Overview

Every login issues an enriched JWT containing all identity and access claims. After the JWT is issued, it is self-contained — no further authz-core call is needed for coarse-grained checks.

## JWT Payload

```json
{
  "sub": "user-uuid",
  "email": "user@example.com",
  "email_verified": true,
  "name": "John Doe",
  "preferred_username": "johnd",
  "user_id": "user-uuid",
  "first_name": "John",
  "last_name": "Doe",
  "org_id": "org-uuid",
  "org_name": "Acme Inc",
  "user_role": "Admin",
  "user_permissions": ["invoices:write", "invoices:read", "users:manage"],
  "mfa_enabled": true,
  "is_platform_admin": false,
  "phone_number": "+141****1234",
  "phone_verified": true,
  "iat": 1705312800,
  "exp": 1705313700
}
```

## Two Auth Levels

### Coarse-grained checks (JWT claims only)

- "Is Admin?" — check `user_role` claim
- "Has invoices:write?" — check `user_permissions` claim
- Zero latency, zero cross-service call

### Fine-grained checks (require authz-core)

- "Can user delete invoice #123?" — `POST /authorize` with action + resource context
- Requires ABAC rules evaluation
- Cached in Redis with 30-second TTL

## JWT Issuance Flow

```
POST /auth/login →
  1. Verify password hash
  2. Call authz-core /principal/effective {user_id, org_id}
  3. Resolve effective roles + permissions from PG
  4. Sign JWT (RS256) with all claims
  5. Return {access_token, refresh_token, user}
```

## Code Anchors

- `microservices/idam/identity-login-service/impl/src/` — JWT signing logic
- `microservices/idam/authz-core/impl/src/` — principal/effective resolution
- `openapi/identity-login-service/openapi.yaml` — Login response schema

## Gaps / Drift

> **Open:** Verify actual JWT claims against implementation. The design doc may not match the current gen/impl output.
