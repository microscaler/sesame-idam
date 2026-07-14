# P0 — Harden the Core

**Target:** Hauliage test-user enablement and Launch 1.0 GA

**Outcome:** a token accepted by any Sesame/BRRTRouter consumer is signature-valid,
standards-conformant, current, and not revoked.

## Scope and dependencies

This phase completes the read side of capabilities already partly delivered: Ed25519/JWKS,
refresh rotation, access-token denylist writes, and the `ver` claim. It depends on the merged
BRRTRouter `JwksBearerProvider` integration and Redis availability. Hybrid online authorization,
DPoP, and enterprise policy evaluation are out of scope.

## Functional requirements

| ID | Requirement |
|---|---|
| FR-P0-001 | Every protected consumer MUST validate signature, `kid`, issuer, audience, expiry, not-before, and configured clock skew using the published JWKS. |
| FR-P0-002 | Access tokens MUST use `typ=at+jwt`; missing or different `typ` values MUST be rejected on protected routes. |
| FR-P0-003 | Consumers MUST reject algorithms outside the configured asymmetric allow-list and MUST never infer the algorithm from an untrusted token alone. |
| FR-P0-004 | Logout and other explicit revocation operations MUST denylist the access-token `jti` for no less than its remaining lifetime. |
| [FR-P0-005](./fr-p0-005-denylist-read-side/README.md) | Every protected consumer MUST consult the denylist and reject a listed `jti`. Cache behavior MUST NOT allow a known revocation to be re-accepted. |
| FR-P0-006 | Consumers MUST compare the token `ver` with the authoritative principal/session version and reject stale versions. |
| FR-P0-007 | Security-sensitive membership, credential, session, and account-state changes MUST bump the applicable version or revoke the affected session family. |
| FR-P0-008 | Unknown `kid` values MUST trigger a bounded JWKS refresh before rejection; refreshes MUST be coalesced or rate-limited. |

## Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-P0-001 | Validation overhead MUST have a benchmark and agreed p95/p99 regression budget for cache-hit and cache-miss paths. |
| NFR-P0-002 | Redis/JWKS outage behavior MUST be specified by ADR, observable, bounded, and covered by tests; no silent security downgrade is permitted. |
| NFR-P0-003 | Denylist and version caches MUST be bounded by TTL and size, and invalidation behavior MUST be deterministic. |
| NFR-P0-004 | Token rejection responses MUST be uniform enough to avoid exposing account, key, or revocation state. |
| NFR-P0-005 | Metrics MUST distinguish signature, claims, algorithm, expiry, denylist, version, and dependency failures without recording token contents. |

## Acceptance criteria

| ID | Observable evidence |
|---|---|
| AC-P0-001 | Given a valid token, after logout the same token receives 401 from every protected consumer route tested end to end. |
| AC-P0-002 | Given a version bump, all earlier tokens for the affected scope receive 401 while a newly issued token succeeds. |
| AC-P0-003 | Tokens with `alg=none`, a symmetric algorithm, an unapproved asymmetric algorithm, missing/wrong `typ`, wrong issuer/audience, or unknown `kid` are rejected. |
| AC-P0-004 | A rotated key validates during its documented grace period and is rejected after revocation/grace expiry. |
| AC-P0-005 | Redis and JWKS outage tests prove the ADR-defined degraded behavior and emit the expected metric/alert signal. |
| AC-P0-006 | Workspace gates and the cross-service revocation/version BDD suite pass with retries disabled. |

## Exit evidence

- Link the BRRTRouter and Sesame accepted commits/PRs.
- Attach the validation benchmark, outage-policy ADR, and test run covering every consumer.
- Confirm configuration and operator runbooks document key rotation, cache behavior, and
  dependency outage response.

## Delivery status

- **FR-P0-005 implementation:** consumer wiring and focused unit/integration coverage are in
  progress. The cross-service logout-to-401 BDD evidence remains required before acceptance.
- **Outage policy:** [ADR-003](../../../ADR-003-token-status-dependency-outage.md) specifies
  fail-closed, bounded Redis behavior for protected requests.
