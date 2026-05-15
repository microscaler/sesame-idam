# Epic 5: Token Versioning & Revocation

## Summary

Implement per-subject and per-tenant token versioning so that privilege changes can invalidate future requests quickly without relying solely on short TTLs or jti denylisting. Add targeted jti denylisting for urgent revocations and push invalidation events for near-real-time response to important authz changes.

## Why This Epic Is Needed

The JWT document identifies "stale permissions" as the primary trade-off of self-contained JWTs: "If a token is self-contained and valid for ten minutes, then any authorisation fact embedded in it can be stale for up to ten minutes unless you add version checks." Token versioning is the bridge between short-lived tokens and immediate revocation. Without it, privilege changes require waiting for token expiry or maintaining a full jti denylist for every user.

## Current State

- `jti` claim is generated and stored for token identification
- `redis.rs` module has a blacklist of revoked token IDs
- Token versioning (`ver` claim) is proposed but not implemented
- No per-subject version cache
- No per-tenant version bump mechanism
- No push invalidation events

## Stories

- [ ] Story 5.1: Add `ver` (token version) claim to access tokens
  - Monotonically increasing integer per subject
  - Bumped whenever user's permissions change
  - Stored in Redis: `authz_ver:{sub}` with short TTL (15-60s)
  - On token validation: compare `claims.ver` to cached version

- [ ] Story 5.2: Implement per-tenant token versioning
  - Per-subject version for individual user revocation
  - Per-tenant version for platform-wide authz changes
  - Bump tenant version when org roles/permissions change
  - Stored in Redis: `authz_ver:{sub}` and `authz_ver:tenant:{tenant_id}`

- [ ] Story 5.3: Implement targeted jti denylisting
  - For exceptional, urgent revocation cases only
  - Store in Redis with TTL matching token `exp`
  - Cache at gateway level for short window (seconds, not minutes)
  - DO NOT check central blacklist on every request -- defeats the purpose

- [ ] Story 5.4: Implement push invalidation events
  - When authz change occurs (role revoked, user disabled, org deleted), emit a version bump event
  - Downstream services drop cached version on receiving the bump
  - Near-real-time response similar to Microsoft Entra's Continuous Access Evaluation
  - Event delivery: Redis pub/sub or similar lightweight mechanism

- [ ] Story 5.5: Implement version mismatch handling
  - When `claims.ver < current_ver`: deny with "stale authz snapshot"
  - Return 401 with retry-after header
  - Client must re-authenticate to get fresh token with new version

## OpenAPI Changes Needed

- `LoginResponse`: Add `token_version` field
- Add endpoint for clients to check current token version: `GET /api/v1/identity/token-version` (optional, for push-invalidation clients)

## Design Doc Changes Needed

- `design-doc.md`: Add token versioning section under JWT Enrichment
- `design-doc.md`: Document the version bump flow when authz changes
- Wiki: Create `topics/topic-token-versioning.md` (new)
- Wiki: Update `topics/topic-jwt-schema.md` to include version claims

## Gaps in the JWT Document

- Does not specify the Redis data structure for version storage. Should it be a simple string or a sorted set with metadata?
- Does not address the version mismatch scenario: should the client be allowed to use the stale token until expiry if the version bump is non-urgent?
- Does not specify how to handle version collisions in a multi-instance deployment (concurrent version bumps).
- Push invalidation via Redis pub/sub works but doesn't survive restarts. What's the persistence layer?

## Dependencies

- Depends on Epic 2 (Claims Schema) for the `ver` claim
- Intersects with Epic 3 (Token Lifecycle) for family-based revocation
- Intersects with Epic 4 (Hybrid Authz) for version check in the validation pipeline
