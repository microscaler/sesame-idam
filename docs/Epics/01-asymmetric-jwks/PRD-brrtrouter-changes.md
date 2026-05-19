# PRD: BRRTRouter JwksBearerProvider Enhancements for Sesame-IDAM

**Author:** Sesame-IDAM Engineering
**Date:** 2026-05-19
**Scope:** BRRTRouter (`microscaler/BRRTRouter`) changes required to support Ed25519
asymmetric JWT validation and production-grade metrics
**Dependencies:** Epic 1 Stories 1.1, 1.2, 1.3

---

## 1. Problem Statement

Sesame-IDAM has migrated from HS256 shared-secret JWTs to Ed25519 asymmetric signing
in Story 1.1. However, BRRTRouter's `JwksBearerProvider` (the component that consumer
services use to validate JWTs) does not support Ed25519. The `SUPPORTED_ALGORITHMS`
constant lists only symmetric and RSA algorithms:

```rust
// src/security/jwks_bearer/mod.rs:17-24
pub(super) const SUPPORTED_ALGORITHMS: &[jsonwebtoken::Algorithm] = &[
    jsonwebtoken::Algorithm::HS256,
    jsonwebtoken::Algorithm::HS384,
    jsonwebtoken::Algorithm::HS512,
    jsonwebtoken::Algorithm::RS256,
    jsonwebtoken::Algorithm::RS384,
    jsonwebtoken::Algorithm::RS512,
];
```

Ed25519 (EdDSA) is **not** in this list. This means:

1. **Consumer services reject Ed25519 tokens** returned by identity-session-service
   with `ValidationError::UnsupportedAlgorithm { alg: EdDSA }`
2. The JWKS keys have `"crv": "Ed25519"` and `"kty": "OKP"` but the decoder can't
   interpret them
3. No EdDSA key loading from JWKS JSON (no `crv` parameter handling)
4. No metrics on `jwks_refresh_failures_total` — only hit/miss/eviction counters exist

**Impact:** Story 1.3 is wired but non-functional. All 5 consumer services have
`JwksBearerProvider` configured but EdDSA tokens will fail validation at every request.

---

## 2. Current Architecture

```
                    ┌─────────────────────────────────────────────────────┐
                    │              BRRTRouter (brrtrouter crate)           │
                    │                                                      │
                    │  src/security/                                       │
                    │  ├── mod.rs            SecurityProvider trait         │
                    │  ├── bearer_jwt.rs     BearerJwtProvider (HS256)      │
                    │  ├── oauth2.rs         OAuth2Provider                │
                    │  ├── remote_api_key.rs RemoteApiKeyProvider           │
                    │  ├── spiffe/           SpiffeProvider (SPIFFE SVID)   │
                    │  └── jwks_bearer/     JwksBearerProvider             │
                    │       ├── mod.rs        Provider + bg refresh + cache │
                    │       └── validation.rs validate_token_impl()         │
                    │                                                      │
                    │  src/middleware/                                     │
                    │  └── jwks.rs          JwksHeadersMiddleware           │
                    └─────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼──────────────────────────────────────┐
                    │               │                                      │
         ┌──────────▼──────┐  ┌────▼──────────────┐  ┌───────────────────▼──────┐
         │ identity-login- │  │ identity-session- │  │ 5 consumer services:     │
         │ service (8101)  │  │ service (8105)    │  │  authz-core, api-keys,   │
         │ signs with      │  │ signs with        │  │  org-mgmt, user-mgmt,    │
         │ Ed25519 private │  │ Ed25519 private   │  │  identity-login-service  │
         │ key             │  │ key + publishes   │  │  validate via JwksBearer │
         │               │  │ JWKS at port 8105 │  │  Provider (reads JWKS)   │
         └───────────────┘  └───────────────────┘  └──────────────────────────┘
```

**JwksBearerProvider capabilities (already implemented):**
- Background JWKS refresh (cache_ttl - 10s interval)
- Claims caching (LRU, kid-scoped, rotation-aware)
- Cookie + header token extraction
- Token invalidation (`invalidate_token()`, `invalidate_token_with_kid()`)
- Cache statistics (`cache_stats()` returns hits/misses/evictions/size/capacity)
- HTTPS enforcement on JWKS URLs (localhost HTTP allowed for testing)
- Debounced concurrent refresh prevention
- Graceful shutdown via `stop_background_refresh()`

**Missing:**
- Ed25519/EdDSA algorithm support
- OKP curve type key decoding from JWKS
- `jwks_refresh_failures_total` metric
- `jwks_fetch_duration_seconds` histogram

---

## 3. Required Changes

### 3.1 Add Ed25519/EdDSA Algorithm Support

**File:** `src/security/jwks_bearer/mod.rs`

Add EdDSA to `SUPPORTED_ALGORITHMS`:

```rust
// CHANGE: Add EdDSA to the algorithm whitelist
pub(super) const SUPPORTED_ALGORITHMS: &[jsonwebtoken::Algorithm] = &[
    jsonwebtoken::Algorithm::HS256,
    jsonwebtoken::Algorithm::HS384,
    jsonwebtoken::Algorithm::HS512,
    jsonwebtoken::Algorithm::RS256,
    jsonjsonwebtoken::Algorithm::RS384,
    jsonwebtoken::Algorithm::RS512,
    jsonwebtoken::Algorithm::EdDSA,  // NEW: Ed25519/EdDSA
];
```

**File:** `src/security/jwks_bearer/mod.rs`

Add Ed25519 key decoding from JWKS. The `jsonwebtoken` crate can decode EdDSA keys
but the JWKS parser needs to handle the `crv` (curve) parameter:

```rust
// CHANGE: In get_key_for(), add Ed25519 OKP key path
fn get_key_for(&self, kid: &str) -> Option<jsonwebtoken::DecodingKey> {
    let cache = self.cache.read().ok()?;
    let keys = &cache.1;
    
    if let Some(key) = keys.get(kid) {
        return Some(key.clone());
    }
    
    // If key not in cache, try to load from JWKS and cache it
    // For Ed25519, the DecodingKey::from_ec_pem() won't work.
    // Need to decode from JWKS JSON directly.
    
    // NEW: Load Ed25519 key from JWKS format
    // JWKS has: {"kty":"OKP","crv":"Ed25519","kid":"...","x":"..."}
    // x is base64url-encoded 32-byte public key
    // Need to construct Ed25519 public key from raw bytes
    self.load_ed25519_key_from_jwks(kid)
}

// NEW METHOD: Decode Ed25519 public key from JWKS JSON
fn load_ed25519_key_from_jwks(&self, kid: &str) -> Option<jsonwebtoken::DecodingKey> {
    // 1. Fetch current JWKS (or use cached)
    // 2. Find key with matching kid
    // 3. Parse "crv" == "Ed25519" and "kty" == "OKP"
    // 4. Base64url-decode "x" parameter (32 bytes)
    // 5. Construct DecodingKey from raw bytes
    //    jsonwebtoken supports: DecodingKey::from_ed_raw(&bytes)
    // 6. Cache and return
}
```

**Dependencies check:** The `jsonwebtoken` crate version in BRRTRouter's Cargo.toml
must support EdDSA. Check:
```toml
# In BRRTRouter/Cargo.toml
jsonwebtoken = "9"  # or >= 9.3.0 for EdDSA support
```

### 3.2 Add JWKS Refresh Failure Metrics

**File:** `src/security/jwks_bearer/mod.rs`

Add failure counter to `JwksBearerProvider`:

```rust
// CHANGE: Add to JwksBearerProvider struct
pub(super) jwks_refresh_failures: std::sync::atomic::AtomicU64,
pub(super) jwks_fetch_durations_ns: std::sync::atomic::AtomicU64,
pub(super) jwks_fetch_count: std::sync::atomic::AtomicU64,

// CHANGE: In refresh_jwks_internal(), track success/failure/duration
fn refresh_jwks_internal(...) {
    let start = std::time::Instant::now();
    let result = Self::do_fetch_jwks(...).await;
    let elapsed = start.elapsed().as_nanos() as u64;
    
    self.jwks_fetch_count.fetch_add(1, Ordering::Relaxed);
    self.jwks_fetch_durations_ns.fetch_add(elapsed, Ordering::Relaxed);
    
    match result {
        Ok(_) => { /* success, cache updated */ }
        Err(e) => {
            self.jwks_refresh_failures.fetch_add(1, Ordering::Relaxed);
            warn!("JWKS refresh failed: {}", e);
        }
    }
}
```

**File:** `src/security/mod.rs`

Expose new metrics in `CacheStats`:

```rust
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: usize,
    pub capacity: usize,
    // NEW:
    pub jwks_refresh_failures: u64,
    pub jwks_fetch_duration_avg_ns: u64,
}

impl CacheStats {
    pub fn jwks_cache_hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { (self.hits as f64 / total as f64) * 100.0 }
    }
}
```

Add accessor method:

```rust
// CHANGE: In JwksBearerProvider::cache_stats()
pub fn cache_stats(&self) -> CacheStats {
    let fetch_count = self.jwks_fetch_count.load(Ordering::Relaxed);
    let fetch_durations = self.jwks_fetch_durations_ns.load(Ordering::Relaxed);
    let avg_duration = if fetch_count > 0 {
        fetch_durations / fetch_count
    } else {
        0
    };
    
    CacheStats {
        hits: self.cache_hits.load(Ordering::Relaxed),
        misses: self.cache_misses.load(Ordering::Relaxed),
        evictions: self.cache_evictions.load(Ordering::Relaxed),
        size: ...,
        capacity: self.claims_cache_size,
        jwks_refresh_failures: self.jwks_refresh_failures.load(Ordering::Relaxed),
        jwks_fetch_duration_avg_ns: avg_duration,
    }
}
```

### 3.3 Prometheus Export Endpoint

**File:** `src/server/mod.rs` or `src/server/http_server.rs`

Expose metrics endpoint. Each `JwksBearerProvider` should expose Prometheus metrics:

```rust
// CHANGE: In AppService, collect JWKS provider metrics
// For each registered JwksBearerProvider, add to prometheus registry:
//   brrtrouter_jwks_cache_hit_ratio{scheme="BearerAuth"} = 97.3
//   brrtrouter_jwks_cache_misses_total{scheme="BearerAuth"} = 2647
//   brrtrouter_jwks_refresh_failures_total{scheme="BearerAuth"} = 3
//   brrtrouter_jwks_fetch_duration_seconds{scheme="BearerAuth", quantile="0.5"} = 0.045
//   brrtrouter_jwks_keys_loaded_total{scheme="BearerAuth", kid="..."} = 1
```

### 3.4 Algorithm Allow-list Per-Scheme

**File:** `src/security/jwks_bearer/mod.rs`

Allow per-scheme algorithm configuration instead of global whitelist:

```rust
// CHANGE: Add algorithm_allow_list to JwksBearerProvider
pub(super) algorithm_allow_list: Option<Vec<jsonwebtoken::Algorithm>>,

// In new():
pub fn allowed_algorithms(mut self, algs: Vec<jsonwebtoken::Algorithm>) -> Self {
    self.algorithm_allow_list = Some(algs);
    self
}

// In validate_token_internal():
// Replace: if !SUPPORTED_ALGORITHMS.contains(&header.alg)
// With:     let allowed = self.algorithm_allow_list.as_deref().unwrap_or(&SUPPORTED_ALGORITHMS);
//           if !allowed.contains(&header.alg)
```

This allows Sesame-IDAM to restrict consumer services to only EdDSA:

```rust
// In authz-core impl/main.rs security init:
let mut p = JwksBearerProvider::new(&jwks.jwks_url);
p = p.allowed_algorithms(vec![jsonwebtoken::Algorithm::EdDSA]);
```

---

## 4. Implementation Plan

| Step | Change | Effort | Risk |
|------|--------|--------|------|
| 1 | Add `EdDSA` to `SUPPORTED_ALGORITHMS` | 15 min | Low |
| 2 | Add `load_ed25519_key_from_jwks()` method | 2h | Medium (crypto) |
| 3 | Wire Ed25519 key loading in `get_key_for()` | 30 min | Low |
| 4 | Add `jwks_refresh_failures` atomic counter | 30 min | Low |
| 5 | Add `jwks_fetch_durations_ns` + track in refresh | 1h | Low |
| 6 | Update `CacheStats` with new fields | 15 min | Low |
| 7 | Add `allowed_algorithms()` builder method | 30 min | Low |
| 8 | Wire `allowed_algorithms` in `validate_token_internal()` | 15 min | Low |
| 9 | Prometheus metric registration in `AppService` | 2h | Medium (infra) |
| 10 | Integration tests for Ed25519 validation | 3h | Medium (test infra) |

**Total estimated effort: ~10 hours**

---

## 5. Verification Criteria

1. **Ed25519 token validation passes** — identity-session-service signs with Ed25519,
   consumer services with `allowed_algorithms=[EdDSA]` successfully validate
2. **Non-EdDSA tokens rejected** — HS256 tokens are explicitly rejected when EdDSA
   is the only allowed algorithm
3. **JWKS refresh failure metric exported** — `jwks_refresh_failures_total` appears
   on `/metrics` endpoint with `scheme` label
4. **Cache hit ratio available** — `jwks_cache_hit_ratio` computed correctly from
   hits/(hits+misses)
5. **Existing tests still pass** — no regression in HS256/RS256 validation

---

## 6. Open Questions

1. **jsonwebtoken crate version** — Does BRRTRouter's version of `jsonwebtoken`
   include EdDSA? If not, we need to bump the dependency version.
2. **Ed25519 key encoding** — The `x` parameter in JWKS is base64url-encoded raw
   32-byte public key. Does `jsonwebtoken::DecodingKey::from_ed_raw()` exist, or
   do we need to construct the PEM manually?
3. **Prometheus crate version** — Does BRRTRouter's `prometheus` crate version
   support histogram quantiles? If not, we can use counters + separate duration
   tracking.
4. **Multiple providers per service** — Can a service register multiple
   `JwksBearerProvider` instances with different algorithm allow-lists? This
   matters if HS256 migration isn't fully complete.

---

## 7. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| jsonwebtoken doesn't support EdDSA | Medium | High | Fallback to custom Ed25519 verifier using `ring` |
| Breaking change to `CacheStats` struct | Low | Medium | Add new fields with `#[serde(skip)]` for backward compat |
| Prometheus metric conflict | Low | Medium | Unique metric prefix `brrtrouter_jwks_` |
| Ed25519 key format mismatch | Medium | High | Test with actual Sesame-IDAM JWKS output |

---

## 8. Out of Scope

- ES256 (ECDSA P-256) support — deferred to a future story
- SPIFFE provider Ed25519 changes — separate from `JwksBearerProvider`
- Token introspection endpoint — not part of Sesame-IDAM's API contract
- DPoP (RFC 9449) support — Epic 8 territory
- Rate limiting — deferred to NGINX/infra layer
