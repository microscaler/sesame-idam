---
title: Dynamic Token-Status Enforcement
status: partially-verified
updated: 2026-07-14
sources: [ADR-003-token-status-dependency-outage.md, roadmap/launch-1.0/p0-harden-core/README.md]
---

# Dynamic Token-Status Enforcement

## Runtime flow

Every protected Sesame service registers a BRRTRouter `JwksBearerProvider` with an EdDSA
allow-list and a shared `SesameTokenStatusChecker` implementation.

1. BRRTRouter validates bearer format, `typ=at+jwt`, configured algorithm, `kid`, signature,
   configured issuer/audience, expiry, and scopes before invoking the dynamic hook.
2. Before authentication succeeds, the provider passes validated claims to the token-status
   checker on both cryptographic cache misses and hits.
3. The checker requires `jti`, `sub`, `tenant_id`, and `ver`, then pipelines Redis `EXISTS` for
   `denylist:{jti}` with subject and tenant version reads.
4. Revoked, stale, unavailable, and malformed status decisions reject authentication. The server
   returns the same invalid-bearer 401 response used for other authentication failures.
5. Claims extraction runs only after validation and does not repeat the dynamic lookup.

## Cache and outage invariants

- Active results are never cached, so the next protected request observes a new denylist key.
- Revoked and stale results are monotonic and cached in a bounded 10,000-entry map. Eviction causes
  a Redis recheck, never an allow decision.
- Redis uses 16 lazy connection shards with 250 ms connect/read/write timeouts.
- Any Redis error fails closed. There is no fail-open configuration for P0.
- `/metrics` includes `sesame_token_status_checks_total` with fixed `result` labels and no token
  or claim values.

## Code anchors

- `microservices/idam/common/src/token_status.rs`
- `microservices/idam/*/impl/src/security.rs`
- `microservices/idam/*/impl/src/main.rs`
- `../BRRTRouter/src/security/jwks_bearer/validation.rs`
- `docs/ADR-003-token-status-dependency-outage.md`

## Status: partially-verified

Focused BRRTRouter and Sesame tests cover provider/cache behavior. P0 acceptance still needs the
live Redis outage metric/timing test and the logout-to-401 matrix across protected consumers.

## Gaps / Drift

> **Open:** Record the accepted BRRTRouter and Sesame commits after the dirty cross-repository P0
> change set is reviewed and committed.
>
> **Open:** FR-P0-001 separately requires explicit `nbf` validation. The current BRRTRouter
> validation config requires `exp` but does not yet enable `validate_nbf`; do not treat this
> FR-P0-005 delivery as closing the broader standard-claims requirement.
