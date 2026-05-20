# Observability — OTEL Spans, Structured Logging & Alerting

## Summary

Sesame-IDAM uses BRRTRouter's existing OTEL tracing and structured logging stack. JWT validation and token lifecycle events are instrumented with `tracing::span!()` and `tracing::info!()/warn!()/error!()` calls. These flow through `tracing-opentelemetry` into Jaeger when `OTEL_EXPORTER_OTLP_ENDPOINT` is set. **No custom Prometheus counters are used** — BRRTRouter's `MetricsMiddleware` already provides `brrtrouter_requests_total`, `brrtrouter_request_duration_seconds`, `brrtrouter_auth_failures_total`, and `brrtrouter_active_requests` on `/metrics`.

**DO NOT build snowflake metrics. Reuse hauliage's OTEL stack.**

## OTEL Span Catalog

### JWT Validation & Authorization (Epic 9.1)

| Span | Service | File | Attributes | Level |
|------|---------|------|------------|-------|
| `authz.request` | authz-core | `impl/src/authz_span_middleware.rs` | `route`, `method`, `result` (allowed/denied), `status` (on denial) | INFO |

**Note:** Per-validation-step sub-spans (`jwt.typ_check`, `jwt.signature_verify`, `jwt.exp_check`, `jwt.issuer_check`, `jwt.audience_check`, `jwt.tenant_check`) **require BRRTRouter changes** — they happen inside `JwksBearerProvider::validate_token()` at `/home/casibbald/Workspace/BRRTRouter/src/security/jwks_bearer/validation.rs`.

### Key Lifecycle (Epic 9.1 — foundation for JWT signing observability)

| Span | Service | File | Attributes | Level |
|------|---------|------|------------|-------|
| `key.generate` | identity-session-service | `impl/src/key_manager.rs` | (key gen parameters) | INFO |
| `key.rotate.prepare` | identity-session-service | `impl/src/key_manager.rs` | (rotation params) | INFO |
| `key.rotate.activate` | identity-session-service | `impl/src/key_manager.rs` | (activation params) | INFO |
| `key.revoke` | identity-session-service | `impl/src/key_manager.rs` | `kid` | INFO |
| `key.health` | identity-session-service | `impl/src/key_manager.rs` | (health check result) | INFO |
| `key.revoke.admin` | identity-session-service | `impl/src/controllers/admin_jwks_revoke.rs` | `kid` | INFO |

### JWKS Document & Cache (Epic 9.1, 9.2)

|| Span | Service | File | Attributes | Level |
||------|---------|------|------------|-------|
|| `jwks.document` | identity-session-service | `impl/src/controllers/jwks.rs` | (key count info) | INFO |
|| `jwks.cache.refresh` | identity-session-service | `impl/src/jwks_client.rs` | `keys_count_bucket` ("1-2"/"3-5"/"6+"), `cache_status` (hit/miss), `result` (allowed/denied), `error` | INFO |

**Note:** Separate `jwks_cache.hit` and `jwks_cache.miss` as top-level spans on each token validation require BRRTRouter changes.

**Security (HACK-921):** Key counts are bucketized into ranges ("1-2", "3-5", "6+") to prevent rotation schedule mapping via span attribute analysis. Exact key counts are never recorded in span attributes.

### Token Lifecycle (Epic 9.5)

| Span | Service | File | Attributes | Level |
|------|---------|------|------------|-------|
| `token.issue` | identity-login-service | `impl/src/controllers/auth_token.rs` | `grant_type` | INFO |
| `token.refreshed` | identity-session-service | `impl/src/controllers/auth_refresh.rs` | `user_id`, `tenant_id`, `result` | INFO |
| `token.issued` | identity-session-service | `impl/src/controllers/admin_issue_token.rs` | `tenant_id`, `user_id` | INFO |

**Missing:** `token.revoked` and `token.refresh_reuse_detected` — no token revocation endpoint exists yet in the codebase.

### Controller-Level Spans (Epic 9.x baseline)

| Span | Service | File | Purpose |
|------|---------|------|---------|
| `create_user` | identity-user-mgmt-service | `impl/src/controllers/create_user.rs` | User creation tracking |
| `delete_user` | identity-user-mgmt-service | `impl/src/controllers/delete_user.rs` | User deletion tracking |
| `disable_user` | identity-user-mgmt-service | `impl/src/controllers/disable_user.rs` | User disable tracking |
| `create_application` | org-mgmt | `impl/src/controllers/create_application.rs` | App creation tracking |
| `delete_org` | org-mgmt | `impl/src/controllers/delete_org.rs` | Org deletion tracking |
| `create_api_key` | api-keys | `impl/src/controllers/create_api_key.rs` | API key creation tracking |
| `delete_api_key` | api-keys | `impl/src/controllers/delete_api_key.rs` | API key deletion tracking |

## Structured Logging

### Token Lifecycle Events

| Event | Level | Fields |
|-------|-------|--------|
| `token_issued` | INFO | `user_id`, `tenant_id`, `grant_type` |
| `token_refreshed` | INFO | `user_id`, `tenant_id`, `result` |
| `jwks_cache_refresh_failure` | WARN | Poisoning detection with "JWKS cache refresh REJECTED" |
| `authz request started` | DEBUG | `route`, `method` |
| `authz request completed` | DEBUG | `route`, `method`, `status`, `result` |

### Security Events

| Event | Level | Fields |
|-------|-------|--------|
| JWT validation failures | WARN | `route`, `user_id`, `error` (planned for per-validation spans) |
| JWKS poisoning | WARN | `error`, `cache_status` |
| Token theft indicators | WARN (planned) | `user_id` (when refresh reuse detected endpoint exists) |

## What Is NOT Implemented (Blocked)

### Blocked on BRRTRouter Changes
- Per-validation-step sub-spans: `jwt.typ_check`, `jwt.signature_verify`, `jwt.exp_check`, `jwt.issuer_check`, `jwt.audience_check`, `jwt.tenant_check`
- `jwks_cache.hit` and `jwks_cache.miss` as separate spans per token validation
- `token.validation` span as child of `jwt_validation`
- Token size budget measurement in spans

### Blocked on Story 4 (Hybrid Authz Model)
- `authz_fallback.cache_hit`, `authz_fallback.call`, `authz_fallback.cache_miss` spans
- `shadow_decision.compare` span (migration mode only)
- `decision_source` field in structured logs (`jwt_claims`, `fallback_cached`, `fallback_online`, `denylist`, `version_mismatch`, `online_only`)
- Per-request structured JWT logging with claim fields (`issuer`, `subject`, `client_id`, `session_id`, `token_id`, `token_version`, `actor_subject`)

### Blocked on Infrastructure
- `token.revoked` and `token.refresh_reuse_detected` spans (endpoint doesn't exist yet)
- Alerting rules in Loki/Grafana (Story 9.7 — requires Loki/Grafana pipeline deployment)

## Security Constraints on Observability Data

### PII Safety — NEVER Log These
- Email, phone, or name fields in span attributes or structured logs
- Raw JWT tokens (access or refresh)
- Full JWT payload contents

### What IS Logged (Safe Fields)
- `user_id` (opaque identifier, not email/phone)
- `tenant_id` (for routing/debugging)
- `jti` (token ID, not full token)
- `kid` (key ID for key management)
- Validation result booleans (`typ_valid`, `sig_valid`, etc.)

### Span Attribute Security
- Span attributes contain only validation result booleans and metadata
- No raw JWT claims (roles, permissions) in attributes
- No PII fields anywhere in spans or logs

## Alerting (Planned — Story 9.7)

Alerts use Loki log filtering (NOT Prometheus) since there are no custom Prometheus counters for JWT observability.

| Alert | Log Query | Severity | Response |
|-------|-----------|----------|----------|
| TokenReuseDetected | `event="refresh_token_reuse_detected"` | CRITICAL | Page on-call |
| TokenRotationFailure | `event="token_rotation_failure"` | CRITICAL | Page on-call |
| JwtValidationSpike | `event="jwt_validation_failed"` | CRITICAL/WARNING | Page/Slack |
| JwksRefreshFailure | `event="jwks_refresh_failure"` | CRITICAL | Page on-call |
| ShadowMismatch | `event="shadow_mismatch"` | WARNING | Slack |

## Integration Pattern (How It Works)

```
Request arrives
  -> BRRTRouter MetricsMiddleware: brrtrouter_requests_total{path, status}
  -> AuthzSpanMiddleware: span!(\"authz.request\", route, method)
  -> JWT validation in JwksBearerProvider (BRRTRouter) — spans TBD
  -> Controller handler: domain-specific spans (token.issued, key.revoke, etc.)
  -> Response recorded in authz.request span result attribute
  -> All tracing calls flow through tracing-opentelemetry -> OTLP -> Jaeger
  -> All tracing logs flow through OTLP -> Loki for alerting
```

## Key Implementation Pattern

```rust
// This is the ONLY approach for Epic 9. DO NOT use prometheus::register_counter!.
// Just tracing::span!() and tracing::info!()/warn!()/error!().

let span = tracing::span!(
    tracing::Level::INFO,
    \"jwt_validation\",
    route = req.path(),
    method = %req.method()
);
let _guard = span.enter();

// ... validation logic ...

span.record(\"result\", \"success\"); // or \"denied\"
span.record(\"error\", %e); // only when denied
```

## References

- [Epic 9: Observability & Monitoring](../../Epics/09-observability/observability.md)
- [Story 9.1: JWT Validation OTEL Spans](../../Epics/09-observability/stories/story-9.1.md)
- [Story 9.2: JWKS Cache Observability Spans](../../Epics/09-observability/stories/story-9.2.md)
- [Story 9.3: Authz Fallback Observability Spans](../../Epics/09-observability/stories/story-9.3.md)
- [Story 9.4: Shadow Decision Observability Spans](../../Epics/09-observability/stories/story-9.4.md)
- [Story 9.5: Token Lifecycle Observability Spans](../../Epics/09-observability/stories/story-9.5.md)
- [Story 9.6: Structured JWT Logging](../../Epics/09-observability/stories/story-9.6.md)
- [Story 9.7: Alerting Configuration](../../Epics/09-observability/stories/story-9.7.md)
