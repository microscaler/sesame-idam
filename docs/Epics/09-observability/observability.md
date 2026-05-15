# Epic 9: Observability & Monitoring

## Summary

Implement the comprehensive metrics, logging, and alerting stack required to validate that the JWT-first authz model is actually reducing load. Without observability, you cannot know whether load reduction is real or whether you have just hidden staleness bugs.

## Why This Epic Is Needed

The JWT document states: "For this design, observability is not optional. Without it, you will not know whether the load reduction is real or whether you have just hidden staleness bugs." The metrics list is explicit and comprehensive -- every metric maps to a specific decision point in the JWT validation pipeline.

## Current State

- No JWT-specific metrics
- No validation latency metrics
- No fallback ratio tracking
- No token size measurement
- No shadow-decision metrics for migration
- No structured logging for JWT decisions
- Tilt logs show HTTP status codes but not authz decision details

## Stories

- [ ] Story 9.1: Implement JWT validation metrics
  - `jwt_validation_total{result,reason}` -- counts of success/failure by reason (expiry, signature, issuer, audience, type)
  - `jwt_validation_latency_ms` -- p50/p95/p99 latency for common-path JWT validation
  - Track in Prometheus format

- [ ] Story 9.2: Implement JWKS cache metrics
  - `jwks_cache_hit_ratio` -- percentage of requests served from JWKS cache
  - `jwks_refresh_failures_total` -- count of failed JWKS fetches
  - Alert on refresh failures

- [ ] Story 9.3: Implement authz fallback metrics
  - `authz_fallback_total{route}` -- count of fallback calls per route
  - `authz_fallback_ratio` -- ratio of fallback calls to total requests per route
  - Alert on fallback ratio spikes (indicates JWT common path is not working)

- [ ] Story 9.4: Implement shadow-decision metrics (migration mode)
  - `authz_shadow_mismatch_total{route}` -- count of times local JWT decision differs from online decision
  - Enabled during migration, disabled after production cut-over
  - Essential for validating the hybrid model before going live

- [ ] Story 9.5: Implement token lifecycle metrics
  - `token_refresh_total`, `refresh_reuse_detected_total`, `refresh_rotation_failures_total`
  - `token_revocation_total`, `revocation_propagation_seconds`
  - `token_size_bytes`, `authorization_header_size_bytes`

- [ ] Story 9.6: Implement structured JWT logging
  - Per-request structured log fields: issuer, subject, client_id, session_id, token_id, token_version, route, decision_source (jwt/fallback/denylist/version_mismatch), actor subject when act is present
  - NEVER log raw access tokens or refresh tokens
  - Log at INFO level for audit trail, WARN for mismatches, ERROR for validation failures

- [ ] Story 9.7: Configure alerting
  - Sudden increases in invalid-token errors
  - JWKS refresh failures
  - Fallback ratio spikes
  - Token-size percentile growth
  - Refresh-token reuse detection
  - Revocation propagation exceeding route-class SLO

## OpenAPI Changes Needed

- No OpenAPI changes needed (all observability is internal)

## Design Doc Changes Needed

- `design-doc.md`: Add "Observability" section under "Scaling & Deployment"
- `design-doc.md`: Document the metric catalog
- `design-doc.md`: Document structured log format
- `design-doc.md`: Document alerting thresholds
- Wiki: Create `topics/topic-observability.md` (new)

## Gaps in the JWT Document

- Does not specify how metrics should be exported (Prometheus? OpenTelemetry? Grafana?).
- Does not specify the monitoring stack (Grafana dashboards? PagerDuty?).
- No guidance on log retention (how long to keep structured JWT logs?).
- No guidance on PII in logs (subject, client_id, session_id -- should these be hashed?).

## Dependencies

- Intersects with all other epics (observability is needed for all of them)
- Can be implemented in parallel with other epics (metrics infrastructure is independent of authz logic)
- Story 9.4 (shadow decisions) is only needed during migration, not post-production
