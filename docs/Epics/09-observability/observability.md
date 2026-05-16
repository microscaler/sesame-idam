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

- [ ] Story 9.1: JWT validation OTEL spans
  - Create spans for each validation step: `jwt.typ_check`, `jwt.signature_verify`, `jwt.exp_check`, `jwt.issuer_check`, `jwt.audience_check`, `jwt.tenant_check`
  - Span attributes: `result` (success/denied), `error` (reason), `route`
  - Structured log on validation failure: `{"event": "jwt_validation_failed", "route": "/api/...", "user_id": "usr_...", "error": "token_expired", "tenant_id": "tenant_abc"}`
  - Verify spans appear in Jaeger traces for each HTTP request
  - **DO NOT use Prometheus counters** — BRRTRouter already provides `brrtrouter_requests_total`

- [ ] Story 9.2: JWKS cache observability spans
  - Create spans: `jwks_cache.hit`, `jwks_cache.miss`, `jwks_cache.refresh`, `jwks_cache.refresh_failure`
  - Span attributes: `keys_count`, `cache_age_seconds`, `endpoint`
  - Log at WARN level on refresh failure (not metrics)
  - Verify spans appear in Jaeger traces
  - **DO NOT use `jwks_cache_hit_ratio` counter** — use BRRTRouter's `brrtrouter_request_duration_seconds` histogram to detect slow JWKS fetches

- [ ] Story 9.3: Authz fallback observability spans
  - Create spans: `authz_fallback.cache_hit`, `authz_fallback.call`, `authz_fallback.cache_miss`
  - Span attributes: `route`, `result` (allowed/denied), `cache_ttl`
  - Log fallback decisions at DEBUG level (high volume, low severity)
  - Verify spans appear in Jaeger traces
  - **DO NOT use `authz_fallback_total{route}` counter** — BRRTRouter's `brrtrouter_requests_total` already tracks per-route request counts

- [ ] Story 9.4: Shadow decision observability spans (migration mode)
  - Create span: `shadow_decision.compare` (only when migration mode is enabled)
  - Span attributes: `jwt_decision`, `online_decision`, `mismatch` (true/false), `reason`
  - Log at WARN level on mismatch: `{"event": "shadow_mismatch", "route": "...", "jwt_decision": "allowed", "online_decision": "denied", "reason": "jwt_allowed_but_online_denied"}`
  - When migration mode is disabled, no span is created (no-op)
  - Verify spans appear in Jaeger traces during migration, not after cutover
  - **DO NOT use `authz_shadow_mismatch_total` counter** — use structured logging for mismatch events

- [ ] Story 9.5: Token lifecycle observability spans
  - Create spans: `token.issued`, `token.refreshed`, `token.revoked`, `token.refresh_reuse_detected`
  - Span attributes: `user_id`, `tenant_id`, `token_version`, `reason` (for revocation)
  - Structured log on token issue: `{"event": "jwt_issued", "user_id": "...", "tenant_id": "...", "token_version": 42, "scopes": "..."}`
  - **DO NOT use `token_refresh_total` or `token_revocation_total` counters** — use structured logs

- [ ] Story 9.6: Structured JWT logging
  - Per-request structured log fields via `tracing`:
    - `issuer`, `subject`, `client_id`, `session_id`, `token_id` (jti), `token_version`, `route`, `decision_source` (jwt/fallback/denylist/version_mismatch), `actor` (when act claim is present)
  - Log levels: INFO for normal operations, WARN for mismatches/failures, ERROR for security events
  - NEVER log raw access tokens or refresh tokens (only jti, not the full token string)
  - Use `brrtrouter::otel::LogConfig::from_env()` for log redaction and sampling (already configured)

- [ ] Story 9.7: Alerting configuration
  - Alert on WARN/ERROR level structured log events in Loki/Grafana:
    - `jwt_validation_failed` at WARN level
    - `shadow_mismatch` at WARN level
    - `jwt_refresh_reuse_detected` at ERROR level
    - `jwt_revocation` at WARN level
    - JWKS refresh failures (logged by BRRTRouter's otel::init_logging_with_config when OTLP endpoint is unreachable)
  - **DO NOT create Prometheus alerting rules** for JWT metrics — use log-based alerting in Loki/Grafana
  - Leverage existing BRRTRouter metrics (`brrtrouter_auth_failures_total`, `brrtrouter_request_duration_seconds`) for HTTP-level alerting

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
