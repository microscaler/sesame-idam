# FR-P0-005 — Denylist Read-Side Enforcement

**Status:** implementation in progress; cross-service acceptance evidence pending

## Story

As an operator onboarding Hauliage test users, I need every protected BRRTRouter consumer to
reject an access token whose `jti` Sesame has denylisted, so logout and targeted revocation take
effect beyond the issuing service.

## Scope

- Run Sesame's dynamic token-status check inside `JwksBearerProvider` after signature and
  standard-claim validation and before authorization succeeds.
- Use the same checker in all six Sesame services through their security-provider initialization.
- Query Redis for `denylist:{jti}` and reject a present key.
- Fail closed on invalid required status claims and Redis connection, read, write, or query errors.
- Emit fixed-label counters without token, `jti`, subject, tenant, or other claim values.

Token-version comparison shares the same pipelined Redis lookup for efficiency, but its functional
acceptance remains tracked by FR-P0-006 and FR-P0-007.

## Functional requirements

| ID | Requirement |
|---|---|
| FR-P0-005-A | A protected request with an active, non-denylisted token MUST continue after one authoritative dynamic-status lookup. |
| FR-P0-005-B | A protected request with a `denylist:{jti}` key MUST receive the same uniform 401 response as another invalid bearer token. |
| FR-P0-005-C | Dynamic status MUST be rechecked when cryptographic claims come from BRRTRouter's JWT claims cache. |
| FR-P0-005-D | Claims extraction MUST follow a successful validation and MUST NOT trigger a second Redis lookup for the same authorization attempt. |
| FR-P0-005-E | Missing or malformed `jti`, `sub`, `tenant_id`, or `ver` status claims MUST fail closed. |

## Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-P0-005-A | Active results MUST NOT be negatively cached; a logout must be observable on the next protected request. |
| NFR-P0-005-B | Revoked and stale results MAY be cached because those decisions are monotonic; the cache MUST be size-bounded and eviction MUST fall back to Redis. |
| NFR-P0-005-C | Redis connect/read/write work MUST have bounded timeouts and errors MUST NOT silently downgrade to allow. |
| NFR-P0-005-D | Connection reuse MUST avoid a single process-wide lock; status lookups use bounded sharded connections. |
| NFR-P0-005-E | Metrics MUST use a fixed result label set: `active`, `denylist`, `version`, `dependency`, and `invalid`. |

## Acceptance criteria

| ID | Evidence |
|---|---|
| AC-P0-005-A | Focused BRRTRouter tests prove the checker runs on both JWT cache miss and hit paths and runs once before claims extraction. |
| AC-P0-005-B | Sesame unit tests prove active is never cached, known rejection is never reaccepted, missing claims fail closed, and Redis errors return `Unavailable`. |
| AC-P0-005-C | A Redis-backed test writes `denylist:{jti}` after an active decision and the very next check returns `Revoked` without a sleep. |
| AC-P0-005-D | An end-to-end suite logs out once and reuses the same access token against representative protected routes in every consumer, each returning 401. |
| AC-P0-005-E | A dependency-outage test proves rejection completes within the ADR bound and `/metrics` increments `result="dependency"`. |

## Design and code anchors

- `microservices/idam/common/src/token_status.rs` — Redis lookup, fail-closed mapping,
  positive-only rejection cache, and counters.
- `microservices/idam/*/impl/src/security.rs` — all Sesame JWKS providers attach the checker.
- `BRRTRouter/src/security/jwks_bearer/validation.rs` — dynamic check at the authentication
  boundary on cryptographic cache hits and misses.
- [ADR-003](../../../../ADR-003-token-status-dependency-outage.md) — dependency failure policy.

## Remaining evidence

- Add the real HTTP logout-to-consumer matrix (AC-P0-005-D).
- Add a deployed Redis outage probe and `/metrics` scrape assertion (AC-P0-005-E); the
  connection-refused unit path already proves bounded fail-closed behavior.
- Record accepted Sesame and BRRTRouter commits in the P0 exit evidence.
