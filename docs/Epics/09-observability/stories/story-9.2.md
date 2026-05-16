# Story 9.2: JWKS Cache Observability Spans

## Epic

[09-observability](../observability.md)

## Parent Epic Story

Story 9.2

## Summary

Create OTEL spans for JWKS cache operations using the `tracing` crate. Spans flow through BRRTRouter's existing `otel::init_logging_with_config()` into Jaeger. **DO NOT use Prometheus counters** — BRRTRouter's `brrtrouter_request_duration_seconds` histogram can detect slow JWKS fetches, and structured logs capture failures.

## Why This Story Exists

The JWT document requires observability for JWKS cache operations. Without spans, you cannot see in Jaeger how often the cache is hit vs. missed, how long refreshes take, and when refreshes fail. **BRRTRouter already provides HTTP-level metrics** — this story adds JWKS-specific diagnostic spans.

## Design Context

### Current State

- JWKS cache exists in Story 7.1 but creates no observable spans
- `jwks_refresh_failures` are logged as errors but not traced
- No visibility into cache hit/miss patterns in traces

### Span Design

```
jwt_validation (from Story 9.1)
└── jwks_cache (sub-span, created when key not found)
    ├── jwks_cache.hit (if key found)
    ├── jwks_cache.miss + jwks_cache.refresh (if key not found)
    │   └── jwks_cache.refresh_success or jwks_cache.refresh_failure
    └── jwks_cache.stale_accept (if cache stale but within tolerance)
```

### Implementation Pattern

```rust
impl JwksCache {
    pub async fn get_key(&self, kid: &str) -> Option<Jwk> {
        let span = tracing::span!(
            tracing::Level::DEBUG,
            "jwks_cache",
            kid = kid,
            route = ?"unknown" // passed from middleware
        );
        let _guard = span.enter();
        
        // Check cache
        if let Some(key) = self.keys.read().await.get(kid) {
            span.record("cache_hit", true);
            span.record("cache_age_seconds", ?self.cache_age());
            return Some(key.clone());
        }
        
        span.record("cache_hit", false);
        
        // Background refresh triggered
        if let Some(refresh_tx) = self.background_refresh_tx.as_ref() {
            let _ = refresh_tx.send(RefreshRequest { kid: kid.to_string() });
            span.record("refresh_triggered", true);
        }
        
        None
    }
    
    async fn refresh(&self) -> Result<(), JwksError> {
        let span = tracing::span!(
            tracing::Level::INFO,
            "jwks_cache.refresh",
            endpoint = self.endpoint
        );
        let _guard = span.enter();
        
        match self.fetch_jwks().await {
            Ok(keys) => {
                span.record("keys_count", keys.len());
                span.record("result", "success");
                Ok(())
            }
            Err(e) => {
                span.record("result", "failure");
                span.record("error", %e);
                tracing::warn!(
                    event = "jwks_refresh_failure",
                    endpoint = self.endpoint,
                    error = %e,
                    "JWKS refresh failed"
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
| `jwks_cache` | `kid`, `cache_hit` (bool), `cache_age_seconds` |
| `jwks_cache.refresh` | `endpoint`, `keys_count`, `result` (success/failure), `error` |

### Structured Log Format (JWKS refresh failure)

```json
{
  "event": "jwks_refresh_failure",
  "endpoint": "https://idam.example.com/.well-known/jwks.json",
  "error": "connection refused",
  "service": "identity-login-service",
  "ts": "2026-05-16T08:30:00Z"
}
```

## Mermaid Diagrams

### JWKS Cache Span Tree

```mermaid
sequenceDiagram
    participant Handler
    participant JWKS as JWKS Cache
    participant OTEL as tracing → OTEL
    participant Endpoint

    Handler->>JWKS: get_key(kid_1)
    JWKS->>JWKS: span: jwks_cache
    JWKS->>JWKS: key found in cache
    JWKS->>OTEL: record cache_hit=true
    JWKS-->>Handler: key_1
    
    Handler->>JWKS: get_key(kid_2)
    JWKS->>JWKS: span: jwks_cache
    JWKS->>JWKS: key NOT found
    JWKS->>OTEL: record cache_hit=false
    JWKS->>JWKS: span: jwks_cache.refresh
    JWKS->>Endpoint: GET /.well-known/jwks.json
    Endpoint-->>JWKS: {keys: [key_1, key_2]}
    JWKS->>OTEL: record keys_count=2, result=success
    JWKS-->>Handler: key_2
```

### Cache Miss Storm (with single-flight)

```mermaid
flowchart TD
    A[1000 concurrent requests for key X] --> B{jwks_cache span}
    B --> C{Cache hit?}
    C -->|No| D{In-flight?}
    D -->|No| E[span: jwks_cache.refresh]
    D -->|Yes| F[wait on existing refresh]
    E --> G{Refresh succeed?}
    G -->|Yes| H[record keys_count=N, result=success]
    G -->|No| I[record result=failure, log WARN]
    F --> H
    F --> I
```

## OpenAPI Changes

No OpenAPI changes. Spans are internal.

## Design Doc References

- `design-doc.md` section 10.11: Caching Strategy -- JWKS cache observability
- BRRTRouter `otel.rs` -- span pattern

## Wiki Pages to Update/Create

- `topics/topic-observability.md`: JWKS cache spans

## Acceptance Criteria

- [ ] `jwks_cache` span created on every key lookup
- [ ] `jwks_cache.refresh` span created on every background or on-demand refresh
- [ ] Span attributes record: `kid`, `cache_hit`, `cache_age_seconds`, `keys_count`, `result`, `error`
- [ ] Refresh failures logged at WARN level with `event: "jwks_refresh_failure"`
- [ ] Spans appear in Jaeger traces
- [ ] No Prometheus counters for JWKS cache (use BRRTRouter's `brrtrouter_request_duration_seconds` histogram for latency)

## Dependencies

- Depends on Story 7.1 (JWKS caching strategy)
- Depends on Story 9.1 (JWT validation spans — parent span)

## Risk / Trade-offs

- **Span overhead**: Each JWKS cache miss creates an additional span. At 100 RPS with 10% miss rate, this is 10 spans/sec — acceptable.
- **No hit ratio metric**: The hit ratio (hits / (hits + misses)) is NOT tracked as a counter. Use structured logs in Loki for that analysis, or calculate from `brrtrouter_requests_total` and span counts.
- **Background refresh spans**: The background refresh loop runs independently. Its spans appear in Jaeger as separate traces (not child spans of HTTP requests). This is correct — refreshes are not HTTP request operations.
