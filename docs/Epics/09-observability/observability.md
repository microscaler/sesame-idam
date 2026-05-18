# Epic 9: Observability & Monitoring

## Summary

Instrument the JWT authz pipeline using BRRTRouter's existing OTEL tracing and structured logging stack. JWT validation steps become OTEL spans visible in Jaeger; JWT diagnostics become structured logs flowing to Loki. **Do NOT build custom Prometheus counters** — BRRTRouter's `MetricsMiddleware` already provides `brrtrouter_requests_total`, `brrtrouter_request_duration_seconds`, and `brrtrouter_auth_failures_total` on `/metrics`.

## Why This Epic Is Needed

The JWT document states: "For this design, observability is not optional. Without it, you will not know whether the load reduction is real or whether you have just hidden staleness bugs." The JWT validation pipeline needs visibility into: which step failed (typ, signature, exp, issuer, audience, tenant), validation latency per route, shadow decision mismatches, and token lifecycle events. **All of this must use `tracing` crate spans/logs** — they flow through `tracing-opentelemetry` into OTEL automatically.

## Hauliage Observability Pattern (MANDATORY)

**DO NOT build snowflake metrics. Reuse hauliage's OTEL stack.**

| Layer | What BRRTRouter already provides | How sesame-idam extends it |
|-------|----------------------------------|---------------------------|
| **Metrics** | `brrtrouter_requests_total{path, status}`, `brrtrouter_request_duration_seconds`, `brrtrouter_auth_failures_total`, `brrtrouter_active_requests` | `tracing::span!()` → OTEL spans (not custom counters) |
| **Traces** | `brrtrouter::otel::init_logging_with_config()` → OTLP → Jaeger | JWT validation steps as named spans with attributes |
| **Logs** | JSON structured logs via OTLP | JWT diagnostics as structured `tracing` calls |
| **Health** | `/health` endpoint from BRRTRouter | No changes needed |

### Pattern from hauliage (Lifeguard example)

```rust
// Lifeguard uses tracing::span!() which flows through tracing-opentelemetry
let span = tracing::span!(tracing::Level::DEBUG, "lifeguard.acquire_connection", pool_tier = ?tier);
let _guard = span.enter();
// ... connection logic ...
span.exit();
```

**Key point:** No separate OTEL SDK calls. Just `tracing::span!()` and `tracing::info!()` — the BRRTRouter `otel::init_logging_with_config()` sets up the global subscriber that routes these to OTLP when `OTEL_EXPORTER_OTLP_ENDPOINT` is set.

### How BRRTRouter middleware hooks into the pipeline

```rust
// BRRTRouter's MetricsMiddleware already instruments HTTP requests
// Add JWT-specific spans inside the JWT middleware or authz handler

impl JwtMiddleware {
    async fn call(&self, req: HttpRequest, next: Next) -> HttpResponse {
        // 1. Create span for JWT validation
        let span = tracing::span!(
            tracing::Level::INFO,
            "jwt_validation",
            route = req.path(),
            method = %req.method()
        );
        let _guard = span.enter();
        
        // 2. Validate JWT with steps
        let result = self.validate_token(&req).await;
        
        // 3. Record result in span attributes
        match &result {
            Ok(_) => span.record("result", "success"),
            Err(e) => {
                span.record("result", "denied");
                span.record("error", %e);
            }
        }
        
        // 4. Continue to next handler
        next.run(req).await
    }
}
```

**This is the ONLY approach for Epic 9.** Do NOT use `prometheus::register_counter!` or `prometheus::register_histogram!` for JWT/authz observability. Those go through BRRTRouter's middleware or `set_extra_prometheus`. JWT-specific diagnostics go through `tracing`.

## Current State

- BRRTRouter metrics middleware is already wired up in all 6 services
- OTEL init (`brrtrouter::otel::init_logging_with_config()`) is in all 6 services' main()
- No JWT-specific spans exist (JWT validation happens but creates no spans)
- No structured logging for JWT decisions (only HTTP-level logs)
- No shadow decision observability
- `/health` works (BRRTRouter provides it)
- `/metrics` works (BRRTRouter provides it)

## Stories

All stories use `tracing::span!()` for spans and `tracing::info!()/warn!()/error!()` for structured logs. No custom Prometheus counters.

Implemented commit: `7cad4ab feat(idam): Epic 9 observability - OTEL spans across all 6 microservices`

### 9.1 JWT validation OTEL spans — PARTIALLY IMPLEMENTED

Implemented spans covering key lifecycle (which is the foundation for JWT validation since keys sign the tokens), but per-validation-step sub-spans (`jwt.typ_check`, `jwt.signature_verify`, `jwt.exp_check`, `jwt.issuer_check`, `jwt.audience_check`, `jwt.tenant_check`) cannot be implemented here — they happen inside BRRTRouter's `JwksBearerProvider::validate_token()` in `/home/casibbald/Workspace/BRRTRouter/src/security/jwks_bearer/validation.rs`. To add those, changes must be made to the BRRTRouter monorepo, not sesame-idam.

**Implemented in sesame-idam:**
- `authz.request` span in `authz-span_middleware.rs` — wraps ALL authz-core requests with route, method, result (allowed/denied)
- `key.generate`, `key.rotate.prepare`, `key.rotate.activate`, `key.revoke`, `key.health` in `key_manager.rs` — key lifecycle that underpins JWT signing
- `jwks.document` in `controllers/jwks.rs` — JWKS document served with key count
- `jwks.cache.refresh` in `jwks_client.rs` — cache validation with hit/miss tracking

**Missing (requires BRRTRouter changes):** Per-validation-step sub-spans listed above.

### 9.2 JWKS cache observability spans — PARTIALLY IMPLEMENTED

Implemented: `jwks.cache.refresh` in `jwks_client.rs` with `cache_status` attribute (hit/miss), `result` (allowed/denied), and `error` (no_overlap). Logs at WARN on poisoning detection.

**Missing:** `jwks_cache.hit` and `jwks_cache.miss` as separate top-level spans. The current `jwks.cache.refresh` span covers the cache lifecycle but does not create separate hit/miss sub-spans on each token validation. Note: token validation itself happens in BRRTRouter, so BRRTRouter would need to emit these spans on cache lookup. `jwks_cache.refresh_failure` would need a separate error path in the JWKS fetch pipeline.

### 9.3 Authz fallback observability spans — NOT IMPLEMENTED

Blocked until Story 4 (hybrid authz model) is implemented. The fallback caching path does not exist in the codebase. When implemented, add `authz_fallback.cache_hit`, `authz_fallback.call`, `authz_fallback.cache_miss` spans.

### 9.4 Shadow decision observability spans — NOT IMPLEMENTED

Blocked until Story 4 migration mode is implemented. `shadow_decision.compare` span should only be created when migration mode is enabled (env var or config flag). No shadow decision infrastructure exists yet.

### 9.5 Token lifecycle observability spans — PARTIALLY IMPLEMENTED

Implemented:
- `token.issue` in `auth_token.rs` (identity-login-service) — with `grant_type` attribute
- `token.refreshed` in `auth_refresh.rs` (identity-session-service) — with `user_id`, `tenant_id`, `result` attributes
- `token.issued` in `admin_issue_token.rs` (identity-session-service) — with `tenant_id`, `user_id` attributes

**Missing:** `token.revoked` and `token.refresh_reuse_detected` — no token revocation endpoint exists yet in the codebase.

### 9.6 Structured JWT logging — PARTIALLY IMPLEMENTED

Implemented: Token lifecycle controllers emit `tracing::info!()` calls with relevant attributes (user_id, tenant_id, grant_type, result). The `authz.request` middleware emits debug-level structured logs on request start/completion.

**Missing:** Per-request structured JWT fields at the authz-core level — `issuer`, `subject`, `client_id`, `session_id`, `token_id` (jti), `token_version`, `decision_source`, `actor`. These would need to be extracted from JWT claims in the JWT middleware or authz-core handler before validation. Currently the `authz.request` span only has route/method/result.

### 9.7 Alerting configuration — NOT IMPLEMENTED

No Loki/Grafana alerting rules created. The structured logs and spans are ready for alerting configuration once the OTEL/Loki pipeline is deployed. Existing spans can trigger alerts on:
- `authz.request` with `result=denied` → `jwt_validation_failed`
- `jwks.cache.refresh` with `result=denied` → JWKS poisoning alert
- `key.revoke` → key revocation alert

## Completion Summary

| Story | Status | Spans Implemented | Blockers |
|-------|--------|------------------|----------|
| 9.1 JWT validation spans | Partial | 8 spans (key lifecycle + authz.request) | Per-step sub-spans require BRRTRouter changes |
| 9.2 JWKS cache spans | Partial | 1 span (jwks.cache.refresh with hit/miss) | Hit/miss as separate spans need BRRTRouter |
| 9.3 Authz fallback spans | Not started | 0 | Blocked on Story 4 (hybrid authz) |
| 9.4 Shadow decision spans | Not started | 0 | Blocked on Story 4 (migration mode) |
| 9.5 Token lifecycle spans | Partial | 3 spans (issue, refreshed, issued) | token.revoked and token.refresh_reuse_detected missing |
| 9.6 Structured JWT logging | Partial | 6 controller spans with fields | JWT claim extraction at authz level missing |
| 9.7 Alerting | Not started | 0 | Requires Loki/Grafana pipeline deployment |

**Total: 19 unique spans across 15 files in 4 of 6 services. 0 spans in api-keys and org-mgmt controllers (except create/delete).**

## OpenAPI Changes Needed

- No OpenAPI changes (observability is internal)

## Design Doc Changes Needed

- `design-doc.md`: Add "Observability" section noting BRRTRouter OTEL stack usage
- `design-doc.md`: Document which tracing spans are created by JWT middleware
- Wiki: Create `topics/topic-observability.md` (new) — document OTEL span catalog

## Gaps to Address

- Log retention: JWT structured logs should follow the same retention as other BRRTRouter logs (managed by OTel Collector/Loki configuration)
- PII in logs: NEVER include email, phone, or name in structured logs — only `user_id`, `tenant_id`, `jti` (token ID, not full token)
- Shadow mode toggle: `shadow_decision` spans should only be created when migration mode is enabled (env var or config flag)

## Dependencies

- Intersects with all other epics (observability is needed for all of them)
- Uses existing BRRTRouter `otel` module (already initialized in all 6 services)
- Story 9.4 (shadow decisions) is only needed during migration, not post-production
