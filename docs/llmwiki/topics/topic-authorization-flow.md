---
title: Authorization Flow
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, service-topology-design.md]
---

# Authorization Flow

## Per-Request Authorization

Called on **every** API request from every consumer app. Must respond in <10ms.

```
Consumer App → POST /api/v1/am/authorize {org_id, action: "invoice:write"} →
  authz-core:
    1. Redis cache lookup (sub + org_id + action)
    2a. Cache HIT → return cached result
    2b. Cache MISS →
         a. Query PG: resolve role/permission rules
         b. Write result to Redis (30s TTL)
         c. Return {allowed: true/false, reason: "..."}
```

## Cache Strategy

- **TTL:** 30 seconds for permission resolution results
- **Target hit ratio:** >99%
- **Sharding:** Can shard by `org_id` (permissions are org-scoped)

## JWT Claims vs /authorize

| Check Type | Mechanism | Latency | Example |
|-----------|-----------|---------|---------|
| Coarse-grained | JWT claims directly | Zero (in-memory) | "Is Admin?" "Has invoices:write?" |
| Fine-grained | `POST /authorize` | <10ms (cached) | "Can user delete invoice #123?" |

## Principal/Effective Flow

Called once at login time from identity-login-service:

```
identity-login-service → POST /api/v1/am/principal/effective {user_id, org_id} →
  authz-core:
    1. Resolve user's roles in this org
    2. Walk role inheritance chain (parent_role_id)
    3. Collect all permissions
    4. Return effective claims for JWT signing
```

## Code Anchors

- `microservices/idam/authz-core/impl/src/` — Authorization handler logic
- `openapi/authz-core/openapi.yaml` — authorize + principal/effective endpoints

## Gaps / Drift

> **Open:** Verify cache implementation, TTL values, and hit ratio targets against source code.
