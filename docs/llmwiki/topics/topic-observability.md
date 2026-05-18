---
title: Topic - Observability (OTEL Spans)
status: partially-verified
updated: 2026-05-18
sources: [key_manager.rs, jwks_client.rs, authz_span_middleware.rs, controllers/*]
---

# Observability — OTEL Span Catalog

> **Status:** Epic 9 implementation in progress. Core key lifecycle and JWKS spans are live. Controllers in identity-session-service are instrumented. Other services have partial coverage.

## How It Works

All 6 sesame-idam services initialize OTEL tracing via `brrtrouter::otel::init_logging_with_config()` in their `main()`. Spans flow through `tracing-opentelemetry` into the OTLP exporter when `OTEL_EXPORTER_OTLP_ENDPOINT` is set. In dev (no endpoint), spans are discarded — `tracing` calls still work.

**No custom Prometheus counters for JWT/authz observability** — BRRTRouter's `MetricsMiddleware` already provides `brrtrouter_requests_total`, `brrtrouter_request_duration_seconds`, and `brrtrouter_auth_failures_total` on `/metrics`. JWT-specific diagnostics go through `tracing::span!()`.

The `set_extra_prometheus` patch merges Lifeguard DB metrics into the same `/metrics` endpoint for a unified scrape target.

## Span Catalog

### Key Management (`key_manager.rs`)

| Span | Level | Attributes | When |
|------|-------|-----------|------|
| `key.generate` | INFO | `kid` (recorded after generation) | New key generated at bootstrap or via `generate_new_key()` |
| `key.rotate.prepare` | INFO | `from_kid`, `to_kid` | Key rotation preparation (`prepare_rotation()`) |
| `key.rotate.activate` | INFO | `new_kid` | Key rotation activation (`activate_next_key()`) |
| `key.revoke` | INFO | `kid`, `reason` | Key revocation (`revoke_key()`) — records `current_key_revoked`, `next_key_revoked`, or `key_not_found` |
| `key.health` | INFO | `key_count` | Health check endpoint (`/health/jwks`) |

**Structured logs:**
- `tracing::info!(kid = ..., "key rotation prepared")` on rotation prepare
- `tracing::info!(kid = ..., "key rotation activated")` on rotation activate
- `tracing::info!(kid = ..., "key revoked (current key)")` on current key revocation
- `tracing::info!(kid = ..., "key revoked (next key)")` on next key revocation

### JWKS (`controllers/jwks.rs`)

| Span | Level | Attributes | When |
|------|-------|-----------|------|
| `jwks.document` | INFO | `keys_count` | Serving `/.well-known/jwks.json` |

**Structured log:** `tracing::info!(keys_count, "JWKS document served")`

### JWKS Cache (`jwks_client.rs`)

| Span | Level | Attributes | When |
|------|-------|-----------|------|
| `jwks.cache.refresh` | INFO | `keys_count`, `cache_status` (hit/miss), `result` (allowed/denied), `error` | `validate_jwks_refresh()` — validates new JWKS against cached keys |

- `cache_status = "miss"` on first fetch (no previous cache)
- `cache_status = "hit"` on successful refresh with overlapping keys
- `cache_status = "miss"`, `result = "denied"` on poisoning detection (no overlap)

**Structured logs:**
- `tracing::info!("jwks cache miss (first fetch)")` on first fetch
- `tracing::info!(keys_count = ..., "jwks cache refresh OK (overlap found)")` on successful refresh
- `tracing::warn!("jwks cache refresh REJECTED (no overlap)")` on poisoning detection

### Admin JWKS Revoke (`controllers/admin_jwks_revoke.rs`)

| Span | Level | Attributes | When |
|------|-------|-----------|------|
| `key.revoke.admin` | INFO | `kid`, `result` (success/denied), `error` | Admin POST `/admin/jwks/revoke` |

**Structured logs:**
- `tracing::info!(kid = ..., "admin: key revoked via admin endpoint")` on success
- `tracing::warn!(kid = ..., error = %e, "admin: key revocation failed")` on failure

### Authz Request (`authz_span_middleware.rs`)

| Span | Level | Attributes | When |
|------|-------|-----------|------|
| `authz.request` | INFO | `route`, `method`, `result` (allowed/denied), `status` (on denied) | Every request to authz-core |

Registered as middleware in `authz-core/impl/src/main.rs` — wraps ALL incoming requests with a span that records `result = "allowed"` or `"denied"` based on HTTP status.

**Structured logs:**
- `tracing::debug!(route = ..., method = ..., "authz request started")` in `before()`
- `tracing::debug!(route = ..., method = ..., status, result = ..., "authz request completed")` in `after()`

### Token Lifecycle

| Span | Level | Attributes | When |
|------|-------|-----------|------|
| `token.issue` | INFO | `grant_type`, `user_id`, `result` (success/denied) | Token issuance in `auth_token.rs` (identity-login-service) |
| `token.refreshed` | INFO | `user_id`, `tenant_id`, `result` (success/denied) | Token refresh in `auth_refresh.rs` (identity-session-service) |
| `token.issued` | INFO | `tenant_id`, `user_id`, `result` (success) | Admin token issuance in `admin_issue_token.rs` (identity-session-service) |

### Other Services (basic coverage)

| Span | Level | Service | When |
|------|-------|---------|------|
| `user.created` | INFO | identity-user-mgmt | `create_user.rs` |
| `user.deleted` | INFO | identity-user-mgmt | `delete_user.rs` |
| `user.disabled` | INFO | identity-user-mgmt | `disable_user.rs` |
| `api_key.created` | INFO | api-keys | `create_api_key.rs` |
| `api_key.deleted` | INFO | api-keys | `delete_api_key.rs` |
| `org.deleted` | INFO | org-mgmt | `delete_org.rs` |
| `application.created` | INFO | org-mgmt | `create_application.rs` |

**Note:** Only representative controllers in each service have spans. Many controllers (especially CRUD list/read operations) do not yet have spans — these can be added on demand.

## Not Yet Implemented

### Story 9.1: Full JWT validation sub-spans
The story proposes sub-spans `jwt.typ_check`, `jwt.signature_verify`, `jwt.exp_check`, `jwt.issuer_check`, `jwt.audience_check`, `jwt.tenant_check`. These validation steps happen inside BRRTRouter's `JwksBearerProvider::validate_token()` in the BRRTRouter library itself (`/home/casibbald/Workspace/BRRTRouter/src/security/jwks_bearer/validation.rs`). Adding sub-spans would require changes to BRRTRouter, not sesame-idam. The current coverage is:
- `authz.request` span wraps the entire request in authz-core (EXTREME frequency service)
- `jwks.cache.refresh` span covers JWKS cache validation
- Key management spans cover key lifecycle events

### Story 9.3: Authz fallback spans
Blocked until Story 4 (hybrid authorization model) is implemented.

### Story 9.4: Shadow decision spans
Blocked until migration mode is implemented.

### Story 9.5: Token revocation span
Not yet implemented — no token revocation endpoint exists in current code.

### Story 9.6: Structured JWT logging
Partial — token lifecycle controllers have `tracing::info!` calls with relevant fields. JWT validation failures are logged at DEBUG level by `authz.request`. Missing: per-request structured logs with `issuer`, `subject`, `client_id`, `session_id`, `jti`, `token_version`, `decision_source`, `actor` fields.

### Story 9.7: Alerting configuration
No alerting rules created yet. The structured logs and spans are ready for Loki/Grafana alerting:
- `jwt_validation_failed` — would trigger from `authz.request` span `result=denied` 
- `shadow_mismatch` — would trigger from shadow decision span when implemented
- `jwks cache refresh REJECTED` — already logged as WARN

## Security Constraints

- Span attributes NEVER include PII (email, phone, name)
- Span attributes NEVER include raw JWT tokens or full payloads
- Only fields: `kid`, `user_id`, `tenant_id`, `route`, `method`, `result`, `error`, `keys_count`, `grant_type`
- Structured logs include `kid` for key events, `user_id`/`tenant_id` for token events

## Files with Tracing

| File | Spans |
|------|-------|
| `identity-session-service/impl/src/key_manager.rs` | key.generate, key.rotate.prepare, key.rotate.activate, key.revoke, key.health |
| `identity-session-service/impl/src/jwks_client.rs` | jwks.cache.refresh |
| `identity-session-service/impl/src/controllers/jwks.rs` | jwks.document |
| `identity-session-service/impl/src/controllers/admin_jwks_revoke.rs` | key.revoke.admin |
| `identity-session-service/impl/src/controllers/auth_refresh.rs` | token.refreshed |
| `identity-session-service/impl/src/controllers/admin_issue_token.rs` | token.issued |
| `identity-login-service/impl/src/controllers/auth_token.rs` | token.issue |
| `authz-core/impl/src/authz_span_middleware.rs` | authz.request |
| `identity-user-mgmt-service/impl/src/controllers/create_user.rs` | user.created |
| `identity-user-mgmt-service/impl/src/controllers/delete_user.rs` | user.deleted |
| `identity-user-mgmt-service/impl/src/controllers/disable_user.rs` | user.disabled |
| `api-keys/impl/src/controllers/create_api_key.rs` | api_key.created |
| `api-keys/impl/src/controllers/delete_api_key.rs` | api_key.deleted |
| `org-mgmt/impl/src/controllers/delete_org.rs` | org.deleted |
| `org-mgmt/impl/src/controllers/create_application.rs` | application.created |
