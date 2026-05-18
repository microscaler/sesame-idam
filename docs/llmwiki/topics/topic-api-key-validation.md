---
title: API Key Validation
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, service-topology-design.md]
---

# API Key Validation

## Validation Flow

```
M2M Client → POST /api-keys/validate/personal {api_key: "sk_..."} →
  api-keys:
    1. Query PG: hash lookup (key_hash = SHA-256(api_key))
    2. Return {valid: true, user: {...}, org: {...}}
```

## Key Points

1. **Simple hash comparison.** SHA-256 stored hash comparison. Extremely fast CPU.
2. **Two validation endpoints:**
   - `/api-keys/validate/personal` — User-scoped key
   - `/api-keys/validate/org` — Org-scoped key
3. **Redis not needed for cache.** Hash lookup itself is so fast (microseconds) that caching adds overhead.

## API Key Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api-keys` | POST | Create API key |
| `/api-keys/{id}` | GET/PATCH/DELETE | Manage key |
| `/api-keys/validate` | POST | Validate any API key |
| `/api-keys/validate/personal` | POST | Validate personal key |
| `/api-keys/validate/org` | POST | Validate org-scoped key |
| `/api-keys/archived` | GET | Fetch expired/revoked keys |
| `/api-keys/usage` | GET | Usage statistics |
| `/api-keys/import` | POST | Import from third-party |

## Code Anchors

- `microservices/idam/api-keys/impl/src/` — Validation handler logic
- `openapi/api-keys/openapi.yaml` — API key endpoints

## Gaps / Drift

> **Open:** Verify actual validation logic and endpoint implementations against source code.
