# Epic 7: Caching Strategy

## Summary

Implement the caching policies recommended in the JWT document: JWKS cache (5 min), subject/tenant version cache (15-60s), online fallback authz result cache (5-30s), denylist cache (until token exp), and entitlement snapshot cache (30-300s). Each cache has a different TTL, invalidation strategy, and failure mode.

## Why This Epic Is Needed

The JWT document identifies caching as critical to the hybrid model's economics. "The common path must stay local" -- and the only way to keep it local is aggressive, well-timed caching. The current Redis infrastructure exists (session cache, permission cache) but doesn't cover the full cache set required for the hybrid authz model.

## Current State

- Redis is already used for: session cache, permission cache (30s TTL, >99% target hit ratio), JWKS cache, API key validation cache
- Generated runtime has `cache_ttl_secs` knobs for remote API-key verification and JWKS validation
- Current caching: authz-core uses Redis 30s TTL for permission resolution
- No entitlement snapshot cache
- No per-route TTL for fallback results
- No denylist cache at service level (jti is stored centrally in Redis)

## Stories

- [ ] Story 7.1: Implement JWKS cache with configurable TTL
  - Default: 5 minutes
  - On cache miss: fetch from `/.well-known/jwks.json`
  - On fetch failure: return cached (stale) or error depending on policy
  - Track `jwks_cache_hit_ratio` and `jwks_refresh_failures_total` metrics

- [ ] Story 7.2: Implement subject/tenant version cache
  - TTL: 15-60 seconds (configurable per route class)
  - Key: `authz_ver:{sub}` or `authz_ver:tenant:{tenant_id}`
  - On cache miss: query authz-core for current version
  - Bypass for high-risk routes (always check central)

- [ ] Story 7.3: Implement online fallback authz result cache
  - TTL: 5-30 seconds (configurable per route class)
  - Key: hash of (sub + org_id + action + resource_id)
  - On cache hit: return cached decision
  - On cache miss: call authz-core, store result
  - Track `authz_fallback_total{route}` and `authz_fallback_ratio`

- [ ] Story 7.4: Implement denylist cache at gateway/service level
  - TTL: Until token `exp` (dynamic per token)
  - Key: `denylist:{jti}`
  - Store on jti revocation, expire on token expiry
  - CRITICAL: Short cache window (seconds, not minutes) to avoid delaying revocation

- [ ] Story 7.5: Implement entitlement snapshot cache
  - TTL: 30-300 seconds (configurable per entitlement complexity)
  - Key: `entitlements:{entitlements_ref}`
  - On cache miss: fetch full ACL from database/authz-core
  - On cache hit: decode and evaluate locally
  - Reduces token size by allowing compact `entitlements_ref` instead of full ACL

## OpenAPI Changes Needed

- No API changes needed (all caching is internal)
- Document cache TTLs in endpoint descriptions for consumers who need to understand staleness windows

## Design Doc Changes Needed

- `design-doc.md`: Add comprehensive caching strategy section
- `design-doc.md`: Document cache hit ratio targets per cache type
- `design-doc.md`: Add cache failure mode handling (stale cache, cache miss storm)
- Wiki: Create `topics/topic-caching-strategy.md` (new)
- Wiki: Update `topics/topic-authorization-flow.md` with cache details per type

## Gaps in the JWT Document

- The document recommends different TTLs but doesn't explain how to reconcile them when a route needs multiple caches with different TTLs. Should the longest TTL win? Should each cache be independent?
- No guidance on cache warming strategies (pre-populate caches on service startup).
- No guidance on cache invalidation under high write load (when entitlements change frequently, short TTLs cause thundering herd on authz-core).
- Does not address cache serialization format (message pack? JSON? protobuf?) for throughput.

## Dependencies

- Intersects with Epic 2 (Claims Schema) for entitlement snapshot caching
- Intersects with Epic 4 (Hybrid Authz) for fallback result caching
- Intersects with Epic 5 (Versioning) for version cache
