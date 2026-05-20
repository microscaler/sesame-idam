---
title: Token Lifecycle
status: verified
updated: 2026-05-20
sources: [design-doc.md, Epics/03-token-lifecycle/stories/story-3.3.md]
---

# Token Lifecycle

## Overview

Sesame-IDAM manages two types of tokens per session:
- **Access tokens**: JWT (ES256-signed), short-lived, included in every API request
- **Refresh tokens**: opaque, long-lived, stored hashed in Redis with one-time-use detection

## Access Token TTL

### Role-Based Tiers (F-010 Aligned)

All access token roles use **5 minutes (300s)** TTL:

| Role | TTL | Config Var |
|------|-----|------------|
| `normal` (customer) | 5 min (300s) | `JWT_ACCESS_TTL_NORMAL` |
| `elevated` | 5 min (300s) | `JWT_ACCESS_TTL_ELEVATED` |
| `admin` (org_admin, platform_admin) | 5 min (300s) | `JWT_ACCESS_TTL_ADMIN` |
| `platform` | 5 min (300s) | `JWT_ACCESS_TTL_PLATFORM` |

### Why All Roles Use 5 Minutes

F-010 aligned all roles to 300s because:
1. **Redis load**: 3-minute admin tokens = 2.5x more refresh ops/hr at 10k admins
2. **Diminishing security return**: admin actions need step-up MFA (Epic 6), not shorter TTL
3. **Operational friction**: admin batch ops can't complete in 1-3 min windows

Step-up MFA (Epic 6) is the real security boundary for high-consequence actions.

### Configuration Resolution

Priority order (highest → lowest):
1. Environment variable (`JWT_ACCESS_TTL_*`)
2. `config.yaml` `jwt.access_token.*_ttl_secs`
3. Default: 300 seconds

### Validation

At startup, `validate_minimum_ttl()` rejects any TTL < 60 seconds. Prevents accidental DoS from zero-TTL misconfigurations (HACK-301).

### Metrics

`token_ttl_seconds{role: "..."}` histogram is recorded at token issue time via `TtlConfig::record_ttl_metric()`.

## Refresh Token TTL

### Role-Based Tiers

| Role | TTL | Config Var |
|------|-----|------------|
| `normal` (customer) | 30 days | `JWT_REFRESH_TTL_DAYS` |
| `admin` (org_admin, platform_admin, elevated) | 7 days | `JWT_ADMIN_REFRESH_TTL_DAYS` |

### Why Longer Than Access Tokens

Refresh tokens are stored hashed in Redis with:
- **One-time-use detection** (invalidated on use)
- **Token family binding** (tear detection)
- **Rotation on every use** (replay protection)

These protections make long-lived refresh tokens safe.

### Validation

`validate_refresh_exceeds_access()` ensures refresh TTL > access TTL for every role at startup (HACK-306).

### Refresh Token Rotation

On refresh:
1. Old refresh token invalidated in Redis
2. New refresh token issued with fresh expiry
3. New access token issued with fresh 300s TTL
4. **Not**: old access tokens are NOT invalidated (fundamental JWT limitation — requires version check + denylist, Story 5.x)

## Token Issuance Flow

```
Login successful
  → Resolve user role (from authz service, NOT client input)
  → ttl_for_role(role) → Duration (always 300s for access)
  → refresh_ttl_for_role(role) → Duration (7 or 30 days)
  → Issue JWT with exp = iat + access_ttl
  → Issue refresh token with exp = iat + refresh_ttl
  → Store refresh token hash in Redis
  → record_ttl_metric(role) → histogram
```

## Security Gotchas

### HACK-301: Zero TTL DoS
If `JWT_ACCESS_TTL_NORMAL=0`, all tokens expire immediately. Mitigated by `validate_minimum_ttl()` at startup.

### HACK-303: Same TTL for All Roles
Admin tokens have the same 5-minute window as customer tokens. Documented trade-off: step-up MFA provides the real security boundary.

### HACK-304: Token Size Budget
Non-issue — same digit count for exp regardless of TTL length.

### HACK-305: Clock Skew Tolerance
60-second tolerance means tokens effectively live up to 6 minutes. Documented as acceptable operational trade-off.

### HACK-306: Refresh Without Access Rotation
Refresh token rotation does NOT invalidate access tokens issued before rotation. Attacker with valid refresh token can keep obtaining new access tokens. Mitigated by:
- Short access token TTL (5 min)
- Version check + denylist (Story 5.x)

### HACK-302: Refresh Token Never Decreases
If refresh TTL is increased, the new value applies going forward. No "never decreases" enforcement. Documented for operational awareness.

## Implementation Files

- `impl/src/jwt/ttl.rs` — `TtlConfig`, `validate_minimum_ttl()`, `validate_refresh_exceeds_access()`, `record_ttl_metric()`
- `impl/src/jwt/mod.rs` — Re-exports TTL types
- `impl/src/main.rs` — Loads config, calls validators at startup
- `impl/src/controllers/auth_register.rs` — Uses TTL config for register flow
- `impl/src/controllers/social_callback.rs` — Uses TTL config for social OAuth flow
- `impl/config/config.yaml` — Default TTL values

## Testing

- **Unit tests**: `impl/src/jwt/ttl.rs` (test module) — role resolution, env override, validation
- **BDD integration tests**: `impl/tests/bdd/jwt_ttl.rs` — role-based TTL, env override, metrics, edge cases
