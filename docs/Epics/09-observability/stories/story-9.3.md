# Story 9.3: Authz Fallback Observability Spans

## Epic

[09-observability](../observability.md)

## Parent Epic Story

Story 9.3

## Summary

Create OTEL spans for authz fallback operations using the `tracing` crate. Spans flow through BRRTRouter's existing `otel::init_logging_with_config()` into Jaeger. **DO NOT use Prometheus counters** — BRRTRouter's `brrtrouter_requests_total{path, status}` already tracks per-route request counts and authz-core call latency is visible in Jaeger traces.

## Why This Story Exists

The JWT document requires observability for the hybrid authorization model. Without spans, you cannot see in Jaeger which routes trigger fallback calls, how often the fallback cache is hit vs. misses, and whether fallback calls are successful. **BRRTRouter already provides HTTP-level metrics** — this story adds authz fallback-specific diagnostic spans.

## Design Context

### Current State

- Authz fallback cache exists in Story 7.2 but creates no observable spans
- Fallback cache hit/miss is not traced
- No visibility into authz-core call success/failure patterns in traces

### Span Design

```
jwt_validation (from Story 9.1)
└── authz_fallback (sub-span, created only for jwt-with-fallback routes)
    ├── authz_fallback.cache_hit (if cached result returned)
    └── authz_fallback.call (if authz-core called)
        └── authz_fallback.call_success or authz_fallback.call_failure
```

### Implementation Pattern

```rust
impl AuthzFallbackHandler {
    async fn handle(&self, req: &AuthorizeRequest) -> Result<AuthorizeResponse, AuthError> {
        let span = tracing::span!(
            tracing::Level::DEBUG,
            "authz_fallback",
            route = req.route,
            action = req.action
        );
        let _guard = span.enter();
        
        // Check cache
        let cache_key = generate_cache_key(&req.subject, &req.org_id, &req.action);
        if let Some(cached) = self.cache.get(&cache_key).await {
            span.record("cache_hit", true);
            span.record("cached_ttl_remaining_secs", ?self.cache_ttl_remaining(&cache_key));
            return Ok(cached);
        }
        
        span.record("cache_hit", false);
        
        // Call authz-core
        let call_span = tracing::span!(
            tracing::Level::INFO,
            "authz_fallback.call",
            route = req.route,
            action = req.action
        );
        let _call_guard = call_span.enter();
        
        match self.authz_client.authorize(req).await {
            Ok(result) => {
                call_span.record("result", "success");
                // Cache result
                self.cache.set(&cache_key, &result, ttl).await;
                Ok(result)
            }
            Err(e) => {
                call_span.record("result", "failure");
                call_span.record("error", %e);
                tracing::warn!(
                    event = "authz_fallback_call_failure",
                    route = req.route,
                    action = req.action,
                    error = %e,
                    "Authz fallback call failed"
                );
                Err(e)
            }
        }
    }
}
```

### Span Attributes

| Span | Attributes |
|------|-----------|
| `authz_fallback` | `route`, `action`, `cache_hit` (bool), `cached_ttl_remaining_secs` |
| `authz_fallback.call` | `route`, `action`, `result` (success/failure), `error` |

### Structured Log Format (fallback call failure)

```json
{
  "event": "authz_fallback_call_failure",
  "route": "/api/v1/identity/preferences",
  "action": "preferences:write",
  "error": "authz-core timeout",
  "service": "identity-user-mgmt-service",
  "ts": "2026-05-16T08:30:00Z"
}
```

## Mermaid Diagrams

### Authz Fallback Span Tree

```mermaid
sequenceDiagram
    participant Handler
    participant Fallback as Authz Fallback
    participant Cache as Redis Cache
    participant Authz as authz-core
    participant OTEL

    Handler->>Fallback: handle(request)
    Fallback->>Fallback: span: authz_fallback
    Fallback->>Cache: GET authz_fallback:{hash}
    alt Cache HIT
        Cache-->>Fallback: {allowed: true}
        Fallback->>OTEL: record cache_hit=true
        Fallback-->>Handler: cached result
    else Cache MISS
        Cache-->>Fallback: nil
        Fallback->>OTEL: record cache_hit=false
        Fallback->>Fallback: span: authz_fallback.call
        Fallback->>Authz: POST /authorize
        Authz-->>Fallback: {allowed: true}
        Fallback->>OTEL: record result=success
        Fallback->>Cache: SET with TTL
        Fallback-->>Handler: fresh result
    end
```

### Fallback Decision Flow

```mermaid
flowchart TD
    A[Request to jwt-with-fallback route] --> B[span: authz_fallback]
    B --> C{Cache hit?}
    C -->|Yes| D[record cache_hit=true<br/>return cached result]
    C -->|No| E[span: authz_fallback.call]
    E --> F{authz-core response}
    F -->|Success| G[record result=success<br/>cache result with TTL]
    F -->|Failure| H[record result=failure<br/>log WARN, return error]
```

## Malicious Hacker Gotchas (Must Be Addressed During Implementation)

> **Source:** `docs/PRS_SECURITY_HARDENING.md` — Security threat model analysis

### HACK-931: Authz Fallback Cache Hit/Cache Miss Timing Differences Create Authorization Oracle (CRITICAL — Hole #5 from PRS)

**Risk:** Attacker uses response timing differences between cache hit and cache miss to determine if a user has specific authorization decisions cached, mapping the user's authorization behavior

The story shows: cache HIT returns immediately, cache MISS calls authz-core (slower). An attacker can measure response times.

**Exploit path (authorization behavior profiling):**
1. Attacker sends 100 requests to `preferences:update` → measures avg response time → fast (cache HIT, user has done this before)
2. Attacker sends 100 requests to `orders:delete` → measures avg response time → slow (cache MISS, never done this)
3. Attacker concludes: the user has never performed `orders:delete` but has performed `preferences:update`
4. Result: The attacker maps the user's authorization behavior without any authorization bypass

**This is a low-risk information leak but helps the attacker plan further attacks:** if the user has never accessed a route, it might mean they don't have permission (the route is not in their cached decisions), OR it means they simply haven't needed to access it yet.

**Implementation requirement:**
- Add random jitter (±50ms) to cache HIT responses to make timing indistinguishable from cache MISS
- Or: always call authz-core in parallel with cache lookup and return the cache result if available
- Document: "Authz fallback cache hit and miss responses have indistinguishable timing."

### HACK-932: Authz Fallback Cache Key Collision via Subject Injection (HIGH — related to Hole #6 from PRS)

**Risk:** Attacker crafts a request with a special `subject` value that causes the cache key hash to collide with another user's cached result

The story uses `blake3::hash(key_data.as_bytes())`. Blake3 is collision-resistant (256-bit), so hash collision is negligible. But what about the key_data format?

**Exploit path (cache key format injection):**
1. The cache key is: `blake3("subject:org_id:action:resource_id:?")`
2. If `subject` contains `:`, it becomes part of the key_data string
3. Example: subject="user_1:admin" → key_data="user_1:admin:org_123:preferences:update:"
4. This is NOT a collision with user_1's key because the subject includes the colon
5. BUT: what if the attacker controls the subject and can make it match another user's subject exactly?
6. The subject is in the JWT (signed), so the attacker cannot forge another user's subject
7. Result: NOT vulnerable to cross-user cache key collision

**The real exploit is different:** What about WITHIN the same user? Can the attacker make different actions hash to the same cache key?

**Exploit path (same-user different-action collision):**
1. User A has a cached result for action `preferences:update`
2. User A (or attacker controlling the account) sends a request for action `preferences:read`
3. The cache key for `preferences:read` is DIFFERENT from `preferences:update` (different action string)
4. So no collision
5. Result: NOT vulnerable

**The real risk is different:** The cache key format uses `:` as a separator, which could be exploited if a field value contains `:`.

**Exploit path (colon in subject for cache poisoning):**
1. Attacker's JWT has subject="user_123" (valid, signed by Sesame)
2. Attacker sends a request with org_id="org_456:admin" (contains colon)
3. The cache key becomes: `blake3("user_123:org_456:admin:action:")`
4. This is different from `blake3("user_123:org_456:admin:action:")` where org_456 is a separate key in the hash
5. Result: No collision, but the cache key is unexpectedly formatted (the colon in org_id is part of the key_data)

**Implementation requirement:**
- Sanitize org_id and resource_id before hashing: remove or escape control characters
- Validate that subject, org_id, action, and resource_id do not contain control characters (ASCII < 0x20)
- Document: "Cache key fields are sanitized to remove control characters before hashing."

---

## OpenAPI Changes

No OpenAPI changes. Spans are internal.

## Design Doc References

- `design-doc.md` section 10.3: Hybrid Authorization Model -- fallback observability
- BRRTRouter `otel.rs` -- span pattern

## Wiki Pages to Update/Create

- `topics/topic-observability.md`: Authz fallback spans

## Acceptance Criteria

- [ ] `authz_fallback` span created for every jwt-with-fallback request — **NOT IMPLEMENTED**, blocked on Story 4 (hybrid authz model not implemented)
- [ ] `authz_fallback.call` span created when authz-core is invoked (cache miss) — blocked on Story 4
- [ ] Span attributes record: `route`, `action`, `cache_hit`, `cached_ttl_remaining_secs`, `result`, `error` — blocked on Story 4
- [ ] Fallback call failures logged at WARN level — blocked on Story 4
- [ ] Spans appear in Jaeger traces — blocked on Story 4
- [x] No Prometheus counters for fallback (BRRTRouter's `brrtrouter_requests_total` covers HTTP-level)

**Summary:** 0 spans implemented. Fully blocked on Story 4 (hybrid authz model).

## Dependencies

- Depends on Story 4.3 (selective online fallback)
- Depends on Story 7.2 (online fallback result cache)
- Depends on Story 9.1 (JWT validation spans — parent span)

## Risk / Trade-offs

- **Span volume**: Each jwt-with-fallback request creates a span. At 100 RPS, this is 100 spans/sec — acceptable for OTEL batch exporters.
- **No fallback ratio metric**: The fallback ratio (fallback calls / total requests) is NOT tracked as a counter. Use Jaeger to filter by `authz_fallback.call` spans and calculate ratios manually during migration.
- **Cache TTL visibility**: `cached_ttl_remaining_secs` is a best-effort attribute — it reflects the time at span creation, not at span completion. For precise TTL tracking, use Prometheus (which we're not doing).
