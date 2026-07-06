# Modularity Audit — Sesame-IDAM

> **Generated:** 2026-06-10
> **Scope:** All 6 microservices + common crate, impl/src only (no gen/, no tests/bdd/)
> **Total impl/src files:** 235 | **Total lines:** 17,820

---

## Executive Summary

| Size Bucket | File Count | Lines | Status |
|---|---|---|---|
| 0–50 lines | 220 (94%) | 5,200 | Healthy |
| 50–100 lines | 6 | 450 | OK |
| 100–200 lines | 7 | 920 | Watch |
| 200–500 lines | 12 | 3,900 | Split |
| 500–1000 lines | 5 | 3,800 | Urgent |
| 1000+ lines | 3 | 4,550 | Critical |

**99 of 235 files exceed 200 lines.** The problem is heavily concentrated: 3 files at 1000+ lines and 5 at 500-1000 lines account for 26% of all lines with only 8 files (3.4%).

---

## Current Linting State

`clippy.toml` defines numeric thresholds, but **all are warn-level only** — none block compilation:

| Threshold | Value | Enforced? |
|---|---|---|
| `cognitive-complexity-threshold` | 30 | warn (clippy::cognitive_complexity) |
| `too-many-lines-threshold` | 200 | warn (clippy::too_many_lines) |
| `too-many-arguments-threshold` | 8 | warn (clippy::too_many_arguments) |
| `type-complexity-threshold` | 300 | warn (clippy::type_complexity) |
| File length | — | **not checked** |
| Deny-grade complexity | — | **never enabled** |

`just lint-rust` runs `cargo clippy -- -D warnings -W clippy::pedantic`. None of the complexity thresholds above are denied. Functions well beyond all limits compile and pass CI.

---

## CRITICAL: Files 1000+ Lines

### 1. `common/src/jwks_cache.rs` — 1,453 lines, 30 pub fn, 27 tests

Contains: JWKS cache struct/builder, background refresh loop, HTTP fetch (`http_get`/`fetch_and_parse`), key selection (`get_key`/`get_any_valid_key`), stale tolerance, health check, metrics.

| Function | Line | Lines | Concern |
|---|---|---|---|
| (impl block — builder) | L398 | ~400 | Builder pattern |
| (impl block — cache) | L405 | ~393 | Cache operations |
| `refresh` | L563 | 167 | HTTP fetch + JSON parse |
| `start_background_refresh` | L729 | 70 | Background task |

**Split target (3 files):**
- `jwks_cache/mod.rs` — struct, builder, cache operations (L1–L390)
- `jwks_cache/refresh.rs` — HTTP fetch, background loop, health check (L395–L1060)
- `jwks_cache/tests.rs` — 27 tests (L1065–end)

---

### 2. `common/src/fallback_cache.rs` — 1,260 lines, 28 pub fn, 28 tests

Contains: `FallbackCache` struct/builder, Redis connection (`redis_url`/`redis_get`/`redis_set`), authz decision serialization (`AuthzDecision`), JWT claims coverage (`JwtClaimsCoverDecision`), metrics.

| Function | Line | Lines | Concern |
|---|---|---|---|
| `redis_url` | L316 | 146 | Redis configuration |
| `redis_get` | L499 | 42 | Redis read |
| `redis_set` | L540 | 27 | Redis write |
| `redis_db_size` | L566 | 79 | Redis metrics |

**Split target (3 files):**
- `fallback_cache/mod.rs` — struct, builder, decision serialization, metrics (L1–L310)
- `fallback_cache/redis.rs` — Redis connection, get/set/db_size (L315–L750)
- `fallback_cache/tests.rs` — 28 tests (L755–end)

---

### 3. `common/src/middleware.rs` — 1,234 lines, 25 pub fn, 26 tests

Contains: `AuthDecision` enum, `PolicyStore` for policy registration, JWT-only evaluation, local policy evaluation, tenant validation, middleware `before`/`after` hooks.

| Function | Line | Lines | Concern |
|---|---|---|---|
| `new` | L63 | 99 | Builder pattern |
| `evaluate_local_policy` | L43 | 89 | Local policy eval |
| `before` | L209 | 45 | Middleware hook |

**Split target (3 files):**
- `middleware/mod.rs` — struct, builder, policy store (L1–L330)
- `middleware/eval.rs` — evaluate_jwt_only, evaluate_local_policy, validate_tenant, evaluate (L335–L760)
- `middleware/tests.rs` — 26 tests (L765–end)

---

### 4. `identity-session-service/impl/middleware/rate_limit.rs` — 886 lines, 42 pub fn, 23 tests

Contains: `RateLimitConfig`, `RateLimitSection`, `JwksRateLimitConfig`, `GlobalRateLimitConfig`, `RateLimiterState`, per-endpoint policies, metrics.

| Function | Line | Lines | Concern |
|---|---|---|---|
| `new` | L398 | 214 | Builder |

**Split target (3 files):**
- `rate_limit/mod.rs` — structs, config, builder (L1–L410)
- `rate_limit/state.rs` — RateLimiterState, rate limiting logic (L415–L660)
- `rate_limit/tests.rs` — 23 tests (L665–end)

---

## URGENT: Files 500–1000 Lines

### 5. `identity-session-service/impl/key_manager.rs` — 1,246 lines, 55 pub fn, 20 tests

Contains: `JwtSigningKey`, `KeyManager` (rotation: prepare/activate/deactivate/rotate), JWKS document serving, key revocation, `JwksDocument`, `KeyManagerError`, health checks.

| Function | Line | Lines | Concern |
|---|---|---|---|
| `generate` | L278 | 59 | Key generation |
| `revoke_key` | L763 | 58 | Key revocation |
| `health` | L872 | 56 | Health check |

**Split target (4 files):**
- `key_manager/mod.rs` — `JwtSigningKey`, `KeyState`, `KeyManagerError`, `JwksDocument` (L1–L200)
- `key_manager/rotation.rs` — KeyManager with prepare/activate/deactivate lifecycle (L205–L760)
- `key_manager/jwks.rs` — JWKS serving, find_public_key, is_revoked, revoke_key (L765–L1060)
- `key_manager/tests.rs` — 20 tests (L1065–end)

---

### 6. `common/src/denylist/cache.rs` — 838 lines, 16 pub fn, 0 tests

Contains: LRU cache, `is_revoked`, add-to-cache, eviction, TTL/jitter calculation, Redis integration. **Zero tests.**

| Function | Line | Lines | Concern |
|---|---|---|---|
| (unnamed impl) | L279–L345 | ~67 | Cache operations |

**Split target (2 files):**
- `denylist/cache.rs` — struct, builder, `is_revoked`, add/remove/clear (L1–L400)
- `denylist/eviction.rs` — evict_oldest, calculate_ttl, apply_jitter, metrics (L405–end)
- Add: `denylist/tests.rs` — test harness

---

### 7. `common/src/entitlement_cache/mod.rs` — 790 lines

Contains: `EntitlementCache` struct/builder, cache operations, TTL, invalidation, truncation. **mod.rs used as implementation (violates convention).**

| Function | Line | Lines | Concern |
|---|---|---|---|
| `new` | L130 | 214 | Builder |
| `truncate_ref` | L396 | 9 | Data truncation |

**Split target (3 files):**
- `entitlement_cache/mod.rs` — struct, builder, cache operations (L1–L420)
- `entitlement_cache/truncate.rs` — truncate_ref, truncation logic (L425–L440)
- `entitlement_cache/tests.rs` — tests (L445–end)

---

### 8. `common/src/jwt_logging.rs` — 812 lines, 27 pub fn, 17 tests

Contains: JWT logging builder pattern, emit functions for validation results, denials, version mismatches, binding mismatches.

| Function | Line | Lines | Concern |
|---|---|---|---|
| `default` | L107 | 39 | Builder default |

**Split target (2 files):**
- `jwt_logging/mod.rs` — struct, builder, emit functions (L1–L570)
- `jwt_logging/tests.rs` — 17 tests (L575–end)

---

### 9. `common/src/dpop.rs` — 925 lines, 14 pub fn, 21 tests

Contains: DPoP proof generation, key pair generation (Ed25519/P256), proof verification, JWK computation.

| Function | Line | Lines | Concern |
|---|---|---|---|
| `verify_proof_signature` | L326 | 40 | Signature verification |
| `create_dpop_proof_jwt` | L365 | 24 | JWT creation |

**Split target (3 files):**
- `dpop/mod.rs` — struct, generate_* functions, compute_jkt (L1–L380)
- `dpop/verify.rs` — verify_dpop_proof, verify_proof_signature, parse_dpop_proof (L385–L500)
- `dpop/tests.rs` — 21 tests (L505–end)

---

### 10. `identity-session-service/impl/jwks_client.rs` — 511 lines, 23 pub fn, 10 tests

Contains: JWKS HTTP client, validation, health checks.

| Function | Line | Lines | Concern |
|---|---|---|---|
| `build_with_config` | L224 | 88 | Client builder |

**Split target (2 files):**
- `jwks_client/mod.rs` — struct, builder, validation (L1–L300)
- `jwks_client/tests.rs` — 10 tests (L305–end)

---

### 11. `identity-login-service/impl/jwt/claims.rs` — 447 lines, 25 pub fn, 15 tests

Contains: `SubjectClaims`, `ActorClaim`, canonical JSON key extraction, claim parsing.

**Split target (2 files):**
- `jwt/claims.rs` — struct, builder, canonicalization (L1–L300)
- `jwt/tests.rs` — 15 tests (L305–end)

---

### 12. `identity-login-service/impl/jwt/ttl.rs` — 514 lines, 32 pub fn, 16 tests

Contains: `TtlConfig` builder, TTL validation.

**Split target (2 files):**
- `jwt/ttl.rs` — struct, builder, validation (L1–L350)
- `jwt/tests.rs` — 16 tests (L355–end)

---

### 13. `identity-user-mgmt-service/impl/jwt/ttl.rs` — 577 lines, 35 pub fn, 18 tests

Contains: `TtlConfig` builder (duplicate of login service).

**Split target (2 files):**
- `jwt/ttl.rs` — struct, builder, validation (L1–L400)
- `jwt/tests.rs` — 18 tests (L405–end)

---

### 14. `identity-session-service/impl/jwt/ttl.rs` — 587 lines, 35 pub fn, 18 tests

Contains: `TtlConfig` builder (third duplicate across services).

**Split target (2 files):**
- `jwt/ttl.rs` — struct, builder, validation (L1–L420)
- `jwt/tests.rs` — 18 tests (L425–end)

---

## NOTABLE: Files 200–500 Lines

| File | Lines | Pub Fn | Tests | Structs | Split Target |
|---|---|---|---|---|---|
| `common/src/token_versioning/subscriber.rs` | 915 | 8 | 2 | 0 | subscriber.rs + tests.rs |
| `identity-session-service/impl/services/token_rotation.rs` | 341 | 6 | 0 | 2 | rotation.rs (small, keep together) |
| `identity-session-service/impl/redis.rs` | 216 | 16 | 0 | 0 | OK as-is (thin wrapper) |
| `authz-core/impl/denylist_middleware.rs` | 221 | 11 | 0 | 2 | OK as-is (moderate) |
| `identity-session-service/impl/controllers/auth_refresh.rs` | 209 | 1 | 0 | 0 | OK as-is (single handler) |
| `identity-session-service/impl/main.rs` | 215 | 1 | 0 | 0 | OK as-is (entrypoint) |

---

## mod.rs Violations (Implementation Code in mod.rs)

`mod.rs` should contain only module declarations and `pub use` re-exports. These files violate that:

| File | Lines | Problem |
|---|---|---|
| `common/src/jwt/mod.rs` | 1,828 | Full JWT engine (builder, validation, serialization, claims, hashing) |
| `common/src/entitlement_cache/mod.rs` | 790 | Full cache implementation |

---

## Tests Embedded in Source

Tests live in the same file as implementation code, making both harder to read and compile:

| File | Test Count |
|---|---|
| `authz/impl/auth_error.rs` | 54 |
| `login/impl/controllers/auth_token.rs` | 51 |
| `common/src/jwt/mod.rs` | 46 |
| `session/impl/key_manager.rs` | 20 |
| `common/src/fallback_cache.rs` | 28 |
| `common/src/jwks_cache.rs` | 27 |
| `common/src/middleware.rs` | 26 |
| `session/impl/middleware/rate_limit.rs` | 23 |
| `common/src/dpop.rs` | 21 |
| `common/src/jwt_logging.rs` | 17 |
| `session/impl/jwks_client.rs` | 10 |
| `login/impl/jwt/claims.rs` | 15 |
| `login/impl/jwt/ttl.rs` | 16 |
| `user-mgmt/impl/jwt/ttl.rs` | 18 |

**Rule:** All tests move to `<module>/tests.rs` or the service's `tests/bdd/` directory.

---

## Full File Listing (impl/src, ordered by size)

### Common Crate

| File | Lines | Pub Fn | Tests | Structs | Status |
|---|---|---|---|---|---|
| `common/src/jwks_cache.rs` | 1,453 | 30 | 27 | 2 | **CRITICAL** |
| `common/src/fallback_cache.rs` | 1,260 | 28 | 28 | 6 | **CRITICAL** |
| `common/src/middleware.rs` | 1,234 | 25 | 26 | 6 | **CRITICAL** |
| `common/src/jwt/mod.rs` | 1,828 | 47 | 46 | 2 | **CRITICAL** |
| `common/src/dpop.rs` | 925 | 14 | 21 | 0 | **URGENT** |
| `common/src/jwt_logging.rs` | 812 | 27 | 17 | 0 | **URGENT** |
| `common/src/denylist/cache.rs` | 838 | 16 | 0 | 0 | **URGENT** |
| `common/src/entitlement_cache/mod.rs` | 790 | 10 | 4 | 1 | **URGENT** |
| `common/src/token_versioning/subscriber.rs` | 915 | 8 | 2 | 0 | NOTABLE |
| `common/src/audit/event.rs` | 857 | 67 | 0 | 0 | (many accessors) |
| `common/src/audit/emitter.rs` | 380 | 16 | 0 | 0 | OK as-is |

### Identity Login Service

| File | Lines | Pub Fn | Tests | Structs | Status |
|---|---|---|---|---|---|
| `login/impl/controllers/auth_token.rs` | 1,976 | 74 | 51 | 5 | **CRITICAL** |
| `login/impl/jwt/ttl.rs` | 514 | 32 | 16 | 1 | **URGENT** |
| `login/impl/jwt/claims.rs` | 447 | 25 | 15 | 0 | **URGENT** |

### Identity Session Service

| File | Lines | Pub Fn | Tests | Structs | Status |
|---|---|---|---|---|---|
| `session/impl/key_manager.rs` | 1,246 | 55 | 20 | 12 | **CRITICAL** |
| `session/impl/middleware/rate_limit.rs` | 886 | 42 | 23 | 6 | **CRITICAL** |
| `session/impl/jwks_client.rs` | 511 | 23 | 10 | 4 | **URGENT** |
| `session/impl/jwt/ttl.rs` | 587 | 35 | 18 | 1 | **URGENT** |
| `session/impl/services/token_rotation.rs` | 341 | 6 | 0 | 2 | NOTABLE |
| `session/impl/redis.rs` | 216 | 16 | 0 | 0 | OK |
| `session/impl/main.rs` | 215 | 1 | 0 | 0 | OK |
| `session/impl/controllers/auth_refresh.rs` | 209 | 1 | 0 | 0 | OK |

### Identity User Mgmt Service

| File | Lines | Pub Fn | Tests | Structs | Status |
|---|---|---|---|---|---|
| `user-mgmt/impl/jwt/ttl.rs` | 577 | 35 | 18 | 1 | **URGENT** |

### Authz Core

| File | Lines | Pub Fn | Tests | Structs | Status |
|---|---|---|---|---|---|
| `authz/impl/auth_error.rs` | 1,055 | 67 | 54 | 3 | **CRITICAL** |
| `authz/impl/denylist_middleware.rs` | 221 | 11 | 0 | 2 | OK |

### API Keys

| File | Lines | Pub Fn | Tests | Structs | Status |
|---|---|---|---|---|---|
| `api-keys/impl/main.rs` | 157 | 1 | 0 | 0 | OK |
| All others | ≤ 120 | — | — | — | OK |

### Org Mgmt

| File | Lines | Pub Fn | Tests | Structs | Status |
|---|---|---|---|---|---|
| `org-mgmt/impl/main.rs` | 187 | 3 | 0 | 0 | OK |
| `org-mgmt/impl/config.rs` | 118 | 1 | 0 | 0 | OK |
| All others | ≤ 75 | — | — | — | OK |

---

## Split Plan (Priority Order)

For each file: move tests to `tests.rs`, extract HTTP/Redis logic to separate impl files, keep `mod.rs` as re-exports + struct declarations only.

| # | File | Lines | Split To | Priority | Done? |
|---|---|---|---|---|---|
| 1 | `common/src/jwt/mod.rs` | 1,828 | mod.rs + builder.rs + validation.rs + serializing.rs + claims.rs + entitlements.rs + tests.rs | **P0** | |
| 2 | `login/impl/controllers/auth_token.rs` | 1,976 | mod.rs + token_builder.rs + parser.rs + handlers.rs + tests.rs | **P0** | |
| 3 | `session/impl/key_manager.rs` | 1,246 | mod.rs + rotation.rs + jwks.rs + tests.rs | **P0** | |
| 4 | `authz/impl/auth_error.rs` | 1,055 | types.rs + tests.rs | **P0** | |
| 5 | `common/src/jwks_cache.rs` | 1,453 | mod.rs + fetch.rs + tests.rs | **P1** | |
| 6 | `common/src/fallback_cache.rs` | 1,260 | mod.rs + redis.rs + tests.rs | **P1** | |
| 7 | `common/src/middleware.rs` | 1,234 | mod.rs + eval.rs + tests.rs | **P1** | |
| 8 | `session/impl/middleware/rate_limit.rs` | 886 | mod.rs + state.rs + tests.rs | **P1** | |
| 9 | `common/src/denylist/cache.rs` | 838 | cache.rs + eviction.rs + tests.rs | **P2** | |
| 10 | `common/src/entitlement_cache/mod.rs` | 790 | mod.rs + truncate.rs + tests.rs | **P2** | |
| 11 | `common/src/dpop.rs` | 925 | mod.rs + verify.rs + tests.rs | **P2** | |
| 12 | `common/src/jwt_logging.rs` | 812 | mod.rs + tests.rs | **P2** | |
| 13 | `session/impl/jwks_client.rs` | 511 | mod.rs + tests.rs | **P2** | |
| 14 | `session/impl/jwt/ttl.rs` | 587 | mod.rs + tests.rs | **P2** | |
| 15 | `common/src/token_versioning/subscriber.rs` | 915 | subscriber.rs + tests.rs | **P2** | |
| 16 | `login/impl/jwt/claims.rs` | 447 | mod.rs + tests.rs | **P2** | |
| 17 | `login/impl/jwt/ttl.rs` | 514 | mod.rs + tests.rs | **P3** | |
| 18 | `user-mgmt/impl/jwt/ttl.rs` | 577 | mod.rs + tests.rs | **P3** | |

---

## Split Convention

Every split follows this pattern:

```
src/module_name/
  mod.rs          # pub struct/enum declarations + pub use re-exports + #[cfg(test)] mod tests
  impl_file.rs    # Implementation code (HTTP, Redis, business logic, etc.)
  tests.rs        # #[cfg(test)] tests (all tests moved here)
```

**mod.rs rules:**
1. All `pub struct`, `pub enum`, `pub trait`, `pub type` declarations live here
2. `pub use impl_file::*;` exports implementation items
3. `#[cfg(test)] mod tests;` imports test file
4. **No implementation code** — no `fn` bodies outside `#[cfg(test)]`

---

## Post-Split: Linting Rules to Add

### clippy.toml changes

```toml
# Phase 2: deny-grade complexity
cognitive-complexity-threshold = 20
too-many-lines-threshold = 150
```

### justfile `lint-rust` additions

- File length check: scan impl/src .rs files, error on files > 500 lines
- Run `just lint-rust` as pre-commit hook

---

## Progress Tracker

|| # | File | Lines | Target | Priority | Status |
||---|---|---|---|---|---|
|| 1 | jwt/mod.rs | 1,828 | 7 files | P0 | DONE |
|| 2 | auth_token.rs | 1,976 | 5 files | P0 | |
|| 3 | key_manager.rs | 1,246 | 4 files | P0 | |
|| 4 | auth_error.rs | 1,055 | 2 files | P0 | DONE |
|| 5 | jwks_cache.rs | 1,453 | 3 files | P1 | DONE |
|| 6 | fallback_cache.rs | 1,260 | 3 files | P1 | DONE |
|| 7 | middleware.rs | 1,234 | 3 files | P1 | |
| 8 | rate_limit.rs | 886 | 3 files | P1 | |
| 9 | denylist/cache.rs | 838 | 3 files | P2 | |
| 10 | entitlement_cache/mod.rs | 790 | 3 files | P2 | |
| 11 | dpop.rs | 925 | 3 files | P2 | |
| 12 | jwt_logging.rs | 812 | 2 files | P2 | |
| 13 | jwks_client.rs | 511 | 2 files | P2 | |
| 14 | session jwt/ttl.rs | 587 | 2 files | P2 | |
| 15 | subscriber.rs | 915 | 2 files | P2 | |
| 16 | login claims.rs | 447 | 2 files | P2 | |
| 17 | login jwt/ttl.rs | 514 | 2 files | P3 | |
| 18 | user-mgmt jwt/ttl.rs | 577 | 2 files | P3 | |
| | **Total lines to split** | **21,518** | **~55 files** | | |
