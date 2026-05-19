# Epic 1 Test Coverage Report

**Date:** 2026-05-19
**Status:** CRITICAL GAPS IDENTIFIED — Consumer services have virtually no JWT validation tests

---

## 1. Executive Summary

| Metric | Value |
|--------|-------|
| **Total tests across all 6 services** | 106 |
| **Services with meaningful test coverage** | 1 of 6 (identity-session-service) |
| **Consumer services with JWT validation tests** | 0 of 5 |
| **HTTP BDD tests for JWT validation** | 10 (JWKS endpoint only) |
| **Unit tests for JwksBearerProvider behavior** | 0 |
| **End-to-end JWT signing + validation tests** | 0 |

**VERDICT:** Epic 1 is wired but testing is concentrated 100% on identity-session-service.
The 5 consumer services that actually USE JwksBearerProvider have exactly 1 smoke test each
and ZERO tests that validate JWT behavior, key rotation handling, or security paths.

---

## 2. Test Inventory by Service

### 2.1 identity-session-service (PRODUCER — signing service)

| Test File | Lines | Type | Count | What's Tested |
|-----------|-------|------|-------|---------------|
| `key_manager.rs` | ~1242 | Unit | 30 | Key gen, rotation, revocation, grace period, find_public_key, cleanup |
| `jwks_client.rs` | — | Unit | 10 | Algorithm allow-list, JWKS poisoning guard, claim extraction |
| `jwks_http.rs` | 322 | HTTP BDD | 10 | JWKS endpoint: live keys, RFC 7517 structure, no private key leakage, size <2KB, Content-Type, no auth, Ed25519 |
| `smoke.rs` | 10 | BDD | 1 | Service startup |
| **Total** | | | **51** | |

**Coverage Assessment:**
- KeyManager logic: **EXCELLENT** — comprehensive unit test suite
- JWKS endpoint: **GOOD** — 10 HTTP BDD tests covering structure, headers, content
- JWT validation paths: **NONE** — no tests for token signing/verification
- Key rotation with active validation: **NONE** — keys rotate but nobody validates during rotation
- Grace period behavior: **PARTIAL** — tested in key_manager but not as HTTP endpoints

### 2.2 authz-core (CONSUMER — EXTREME frequency)

| Test File | Lines | Type | Count | What's Tested |
|-----------|-------|------|-------|---------------|
| `smoke.rs` | 10 | BDD | 1 | Service startup |
| **Total** | | | **1** | |

**Coverage Assessment:**
- JwksBearerProvider wiring: **UNTESTED** — config.yaml has correct JWKS URL but no test hits it
- JWT validation: **ZERO tests** — no test sends a JWT and checks validation
- Invalid token handling: **ZERO tests**
- Missing kid token: **ZERO tests**
- Expired token: **ZERO tests**
- Wrong signature: **ZERO tests**

### 2.3 api-keys (CONSUMER — HIGH frequency)

| Test File | Lines | Type | Count | What's Tested |
|-----------|-------|------|-------|---------------|
| `smoke.rs` | 10 | BDD | 1 | Service startup |
| **Total** | | | **1** | |

**Coverage Assessment:** Same gaps as authz-core

### 2.4 org-mgmt (CONSUMER — LOW frequency)

| Test File | Lines | Type | Count | What's Tested |
|-----------|-------|------|-------|---------------|
| `smoke.rs` | 10 | BDD | 1 | Service startup |
| **Total** | | | **1** | |

**Coverage Assessment:** Same gaps as authz-core

### 2.5 identity-user-mgmt-service (CONSUMER — MEDIUM frequency)

| Test File | Lines | Type | Count | What's Tested |
|-----------|-------|------|-------|---------------|
| `smoke.rs` | 10 | BDD | 1 | Service startup |
| **Total** | | | **1** | |

**Coverage Assessment:** Same gaps as authz-core

### 2.6 identity-login-service (CONSUMER — HIGH frequency, ALSO PRODUCER)

| Test File | Lines | Type | Count | What's Tested |
|-----------|-------|------|-------|---------------|
| `smoke.rs` | 10 | BDD | 1 | Service startup |
| **Total** | | | **1** | |

**Coverage Assessment:** Same gaps as other consumers. Notably, this service ALSO signs JWTs
(it calls authz-core at login), so it needs BOTH consumer tests (validating others' tokens)
AND producer tests (validating its own tokens work).

---

## 3. Missing Test Categories

### 3.1 JWT Validation BDD Tests (CRITICAL — 0 tests)

Every consumer service needs tests that:

| Test | Purpose | Risk if Missing |
|------|---------|-----------------|
| Valid Ed25519 JWT → 200 | Happy path: token signed with current key | If EdDSA not supported, ALL requests fail silently |
| Expired JWT → 401 | Token past exp time | Security: tokens don't expire |
| Wrong signature → 401 | Tampered token | CRITICAL: accepts forged tokens |
| Missing kid header → 401 | No key ID in JWT | Should fail, not accept |
| Unknown kid → 401 | Key ID not in JWKS | Should fail, not accept |
| Wrong issuer → 401 | Token from different issuer | Security: accepts tokens from wrong IDP |
| Wrong audience → 401 | Token not for this service | Security: cross-service token reuse |
| No Authorization header → 401 | Unauthenticated request | Endpoint unprotected |
| Bearer prefix missing → 401 | Malformed header | Should still parse "token" directly |
| Token with HS256 alg → 401 | Algorithm mismatch | Security: accepts weaker algorithm |

### 3.2 Key Rotation Tests (CRITICAL — 0 tests)

| Test | Purpose | Risk if Missing |
|------|---------|-----------------|
| Valid token from rotated key → 200 | Token signed before rotation still works | If previous_key not checked, tokens rejected during rotation overlap |
| Token from expired key → 401 | Key expired + grace period passed | If not enforced, revoked keys still work |

### 3.3 JWKS Error Handling Tests (HIGH — 0 tests)

| Test | Purpose | Risk if Missing |
|------|---------|-----------------|
| JWKS unavailable → use cached keys | Provider retains old key set | If provider panics/exits, all requests fail |
| Malformed JWKS JSON → error logged | Provider rejects bad JWKS | If not handled, crash on malformed response |
| JWKS refresh failure → retry later | Debounce and retry logic | If no retry, stale keys persist indefinitely |

### 3.4 Metrics Tests (MEDIUM — 0 tests)

| Test | Purpose | Risk if Missing |
|------|---------|-----------------|
| Cache hit counted correctly | jwks_cache_hit_ratio accuracy | Blind to cache performance |
| Cache miss counted correctly | Miss rate indicates JWKS fetch load | Blind to refresh frequency |
| Refresh failure counted | jwks_refresh_failures_total visibility | Blind to provider health |

### 3.5 Security Boundary Tests (HIGH — 0 tests)

| Test | Purpose | Risk if Missing |
|------|---------|-----------------|
| alg:none attack → 401 | Reject algorithm confusion | CRITICAL: bypasses all validation |
| alg=RS256 when EdDSA expected → 401 | Algorithm allow-list enforcement | Security: accepts weaker RSA keys |
| Oversized JWT → 401 | DoS protection | Resource exhaustion |

---

## 4. BRRTRouter Library-Level Test Coverage

BRRTRouter's own test suite (in `BRRTRouter/`):

| Module | Test Coverage | Notes |
|--------|--------------|-------|
| `router/` | Tests present | Radix trie, path matching |
| `middleware/cors/` | Tests present | CORS policy application |
| `middleware/metrics/` | Tests present | Metrics collection |
| `middleware/jwks.rs` | **NO TESTS** | JwksHeadersMiddleware — just Cache-Control injection |
| `security/jwks_bearer/mod.rs` | **NO UNIT TESTS** | JwksBearerProvider — no tests for cache, rotation, background refresh |
| `security/jwks_bearer/validation.rs` | **NO UNIT TESTS** | validate_token_impl — no tests for any validation path |
| `security/bearer_jwt.rs` | **NO UNIT TESTS** | BearerJwtProvider — HS256 path, no tests |
| `security/spiffe/` | **NO UNIT TESTS** | SPIFFE provider |
| `dispatcher/` | Tests present | HandlerRequest/HandlerResponse |
| `server/` | Tests present | AppService wiring |

**Critical Gap:** JwksBearerProvider has zero unit tests in BRRTRouter itself.
The entire validation logic (validation.rs, ~543 lines) is completely untested at the library level.

---

## 5. Test Gap Summary Matrix

| Test Category | identity-session-service | Consumer Services (5) | BRRTRouter Library | Priority |
|--------------|------------------------|----------------------|-------------------|----------|
| Unit tests (core logic) | ✅ 30 tests | ❌ 0 | ❌ 0 | CRITICAL |
| HTTP BDD tests (handler level) | ✅ 10 tests | ❌ 0 | N/A | CRITICAL |
| JWT validation tests | ❌ 0 | ❌ 0 | ❌ 0 | CRITICAL |
| Key rotation tests | ❌ 0 | ❌ 0 | ❌ 0 | HIGH |
| Error handling tests | ❌ 0 | ❌ 0 | ❌ 0 | HIGH |
| Security boundary tests | ❌ 0 | ❌ 0 | ❌ 0 | HIGH |
| Metrics tests | ❌ 0 | ❌ 0 | ❌ 0 | MEDIUM |
| End-to-end (sign + validate) | ❌ 0 | ❌ 0 | N/A | CRITICAL |

---

## 6. Recommendations

### 6.1 Immediate (Block on Story 1.3 completion)

1. **Add JWT validation BDD tests to 1 consumer service first (authz-core)**
   - Uses handler-level testing via brrtrouter's HandlerRequest/HandlerResponse channels
   - Tests all 10 validation paths listed in section 3.1
   - Reuse the jwks_http.rs pattern (322 lines) — same approach, different service

2. **Add a mock JWKS server for tests**
   - Returns valid JWKS with Ed25519 keys
   - Can serve keys of different ages (current, rotated, expired)
   - Can return errors (500, malformed JSON)
   - Can serve different algorithms (EdDSA, HS256)

3. **Add end-to-end test: sign in identity-session-service → validate in authz-core**
   - Use the same Ed25519 key pair across both services in tests
   - Proves the full pipeline: sign → JWKS publish → validate

### 6.2 Short-term (Within 2 weeks)

4. **Mirror authz-core JWT validation tests to remaining 4 services**
   - Same test logic, different config (port, base URL)
   - Can be templated/parameterized

5. **Add key rotation BDD tests**
   - Rotate key in identity-session-service via admin endpoint
   - Verify tokens from old key still work during overlap
   - Verify expired keys are rejected

6. **Add BRRTRouter library-level unit tests**
   - Unit tests for validation.rs: each ValidationError variant
   - Unit tests for jwks_bearer/mod.rs: cache, background refresh, invalidation

### 6.3 Medium-term (Next iteration)

7. **Add metrics integration tests**
   - Verify /metrics endpoint includes jwks metrics
   - Verify hit/miss/failure counts are accurate

8. **Add fuzz testing for JWT validation**
   - Malformed JWTs
   - Edge-case claims
   - Buffer overflow attempts

---

## 7. Risk Assessment

| Risk | Probability | Impact | Description |
|------|-----------|--------|-------------|
| EdDSA tokens silently rejected | HIGH | CRITICAL | 0 tests means we don't know if Ed25519 actually works in consumers |
| Revoked keys still accepted | HIGH | CRITICAL | Without tests, key revocation may not work across all services |
| Stale JWKS keys after rotation | MEDIUM | HIGH | Without tests, key rotation may break validation |
| Algorithm confusion attacks | LOW | CRITICAL | No alg:none or alg-mismatch tests |
| Cache invalidation broken | MEDIUM | MEDIUM | No tests for claims_cache behavior |

---

## 8. Test Implementation Strategy

### 8.1 Handler-Level Testing Pattern (Recommended)

Use the same approach as `jwks_http.rs` in identity-session-service:

```rust
// Pattern from jwks_http.rs:
use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse, HeaderVec};
use brrtrouter::ids::RequestId;
use brrtrouter::typed::{Handler, TypedHandlerRequest};

#[test]
fn test_valid_jwks_jwt_returns_200() {
    // 1. Create HandlerRequest with valid Ed25519 JWT in Authorization header
    // 2. Invoke the handler through Handler::handle()
    // 3. Assert HandlerResponse.status == 200
}
```

This approach:
- Tests the actual handler code path (not mocking)
- Exercises the full JwksBearerProvider validation pipeline
- Can be parameterized for each consumer service
- Follows the existing brrtrouter pattern

### 8.2 Mock JWKS Server Design

```rust
// Simple HTTP server that serves JWKS
struct MockJwksServer {
    keys: Vec<Jwk>,
    port: u16,
}

impl MockJwksServer {
    fn new(ed25519_keys: Vec<Jwk>) -> Self { ... }
    fn serve_jwks(&self) -> String { ... }  // Returns {"keys": [...]}
    fn rotate_key(&mut self, new_key: Jwk) { ... }  // Swap current key
    fn expire_key(&mut self, kid: &str) { ... }  // Remove key
    fn serve_error(&self, code: u16) -> Response { ... }
}
```

---

## 9. Conclusion

**Epic 1 is wired but critically under-tested.** The producer service (identity-session-service)
has 51 tests covering key management and JWKS endpoint. The 5 consumer services that USE
JwksBearerProvider have exactly 1 smoke test each — no JWT validation tests whatsoever.

**Before Story 1.3 can be considered "done":**
1. At minimum: JWT validation BDD tests in authz-core (the EXTREME frequency service)
2. Ideally: Same tests across all 5 consumer services
3. Required: End-to-end sign-in-identity-session-service, validate-in-consumer test

**Without these tests, we are deploying an unverified security pipeline.**
