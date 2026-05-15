---
title: Token Versioning
status: partially-verified
updated: 2026-05-16
sources: [design-doc.md, Epics/05-token-versioning/versioning.md]
---

# Token Versioning

## Overview

Token versioning provides instant privilege invalidation without relying solely on short token TTLs. A monotonically increasing `ver` claim is embedded in every JWT, and per-subject/tenant versions are stored in Redis. When authz changes occur, the version is bumped — existing tokens with stale versions are rejected on the next request.

## Design

### Claims

```json
{
  "ver": 42,
  "sid": "ses_01JV8W..."
}
```

- `ver` = monotonically increasing version (u64) per subject
- `sid` = session ID identifying which session this token belongs to

### Version Storage

| Key | Type | TTL | Purpose |
|-----|------|-----|---------|
| `authz_ver:{sub}` | String | 15-60s | Current version for subject |
| `authz_ver:tenant:{tenant_id}` | String | 15-60s | Current version for tenant |

### Token Issue Flow

1. Read current version from `authz_ver:{sub}` (default 0)
2. Increment: `new_ver = current_ver + 1`
3. Store: `SET authz_ver:{sub} new_ver EX 30`
4. Include `ver` and `sid` in the JWT

### Token Validation Flow

1. Read cached version: `GET authz_ver:{sub}` and `GET authz_ver:tenant:{tenant_id}`
2. If `claims.ver < cached_ver`: reject with "stale authz snapshot" (401)
3. If `claims.ver >= cached_ver`: allow

### Version Bump on Authz Change

When a role/permission change occurs:
1. `INCR authz_ver:{tenant_id}` → atomic bump
2. `INCR authz_ver:{sub}` for the affected user
3. Metric `token_version_total{event: "bumped"}` emitted
4. Existing tokens with older `ver` are rejected on next request

### TTL Strategy

Subject version: 15 seconds. Tenant version: 60 seconds. This is shorter than token TTL (300s), meaning after a version bump, stale tokens are rejected for the cache TTL duration. After TTL expiry, the cache is empty and the version check is skipped (fail open).

### Version Mismatch Handling

When `claims.ver < cached_ver`:
- HTTP 401 Unauthorized with `WWW-Authenticate: Bearer error="stale_auth_token", retry_after=N`
- Response body: `{"error": "stale_auth_token", "message": "...", "retry_after": N, "reason": "stale_authz_snapshot"}`
- Gap 1-10: `retry_after = 300` (allow refresh)
- Gap >100: `retry_after = 0` (immediate re-auth required)
- jwt-only and jwt-with-fallback routes skip version check entirely

## Risks / Trade-offs

- **Redis dependency**: If Redis is down, version check is skipped (fail open). Signature validation, exp, iss, aud still apply.
- **Gap between bump and rejection**: Existing tokens with older `ver` are rejected on the NEXT request, not immediately. The window is up to token TTL.
- **Version overflow**: u64 at 1 bump/second would take ~584,000 years. No overflow handling needed.

## Wiki References

- **Related stories**: Story 5.1 (ver claim), Story 5.2 (version cache), Story 5.3 (jti denylist), Story 5.4 (push invalidation), Story 5.5 (version mismatch)
- **Intersects with**: Story 4.4 (route classification — which routes check version), Story 6.1 (token exchange bumps version), Story 6.3 (step-up MFA bumps version)
