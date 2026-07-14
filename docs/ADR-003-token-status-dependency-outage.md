# ADR-003 — Fail Closed When Token-Status Redis Is Unavailable

**Status:** Accepted for P0 and Hauliage test-user enablement
**Date:** 2026-07-14

## Context

Sesame access tokens are cryptographically self-contained, but explicit `jti` revocation and
principal/tenant version invalidation are dynamic security decisions stored in Redis. Allowing a
request when Redis cannot answer would silently disable logout and authorization-change
invalidation. Blocking without a bound would instead let a Redis fault exhaust BRRTRouter workers.

## Decision

Protected bearer-token requests fail closed when the token-status lookup cannot connect, read,
write, decode, or complete its Redis pipeline.

- Connection, read, and write operations use a 250 ms timeout.
- Sixteen lazily connected shards bound connection-lock contention; a failed connection is dropped
  and the next request may reconnect.
- Active decisions are never cached. Every successfully authorized protected request therefore
  consults Redis once, including when the JWT's cryptographic claims are cached.
- Revoked and stale decisions are monotonic and may be retained in a 10,000-entry per-provider
  cache. Arbitrary eviction is safe because the next request returns to Redis; it never turns a
  cached rejection into an allow decision.
- BRRTRouter returns its uniform invalid-bearer 401 response. It does not reveal whether signature,
  denylist, version, required-claim, or Redis dependency validation failed.
- `sesame_token_status_checks_total{result=...}` distinguishes `active`, `denylist`, `version`,
  `dependency`, and `invalid` without claim or token labels.

## Consequences

- Redis is on the availability path for every protected Sesame request. During an outage, public
  endpoints remain available but protected endpoints reject access rather than accepting
  potentially revoked or stale authority.
- A Redis round trip is added to protected requests. Pipelining combines denylist and two version
  reads; sharding prevents one connection mutex from serializing the whole service.
- Operators must alert on `result="dependency"`, restore Redis, and verify recovery before
  admitting test users. Clients should treat the uniform 401 as non-retryable authentication
  failure; internal alerts distinguish the dependency cause.

## Rejected alternatives

- **Fail open:** rejected because it silently removes explicit revocation and version enforcement.
- **Negative-cache active tokens:** rejected because the next request after logout could still be
  accepted for the cache TTL.
- **Check only authz-core:** rejected because other protected consumers would omit enforcement.
- **Per-service middleware:** rejected because registration could drift and middleware ordering
  relative to JWKS validation is easier to misconfigure than the provider boundary.

## Verification

- Focused unit tests cover active/no-cache, monotonic rejection cache, missing claims, and Redis
  errors.
- BRRTRouter tests cover dynamic checks on cryptographic cache hits/misses and one check before
  claims extraction.
- P0 exit still requires a live Redis outage timing/metric test and cross-service logout-to-401 BDD.
