---
title: Hybrid Authz Model
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, Epics/04-hybrid-authz-model/hybrid.md, docs/Sesame-idam_authorisation_load_mitigation_with_JWT_claims.md]
---

# Hybrid Authorization Model (Epic 4)

## Overview

The hybrid authorization model replaces the old "call authz-core on every request" pattern. JWT claims handle the common path (coarse-grained checks with zero latency), with a lightweight online fallback for high-risk, dynamic, or high-cardinality decisions.

**Core thesis (from JWT document):** "Not put all permissions in JWTs and delete online checks." The winning design is a bounded-claims, short-lived-token, hybrid-fallback architecture.

## Route Categories

| Category | Description | Online Fallback | Authz-Core Calls |
|----------|-------------|----------------|-----------------|
| `jwt-only` | All authz decisions from JWT claims | No | Zero |
| `jwt-with-fallback` | JWT handles common path, fallback for edge cases | Yes, cached 5-30s | Only when claims don't cover decision |
| `online-only` | All decisions require online evaluation | Yes, no cache | Always |

### Route Classification (Story 4.1)

The classification is stored in a RoutePolicyStore loaded at startup. Default fail-safe is `jwt-with-fallback` (safe default — unknown routes always check online).

| Path Pattern | Methods | Category |
|--------------|---------|----------|
| `/identity/me` | GET | jwt-only |
| `/identity/me` | GET | jwt-only |
| `/identity/me` | PUT, PATCH | jwt-with-fallback |
| `/identity/me` | PUT, PATCH | jwt-with-fallback |
| `PUT /admin/users/{user_id}/email` | PUT | jwt-with-fallback |
| `/authz/authorize` | POST | online-only |
| `/authz/principals/effective` | POST | online-only |
| All API key CRUD | POST/PUT/DELETE | online-only |
| All org lifecycle | POST/PUT/DELETE | online-only |

**Split:** ~40 jwt-only + ~50 jwt-with-fallback + ~43 online-only = ~133 endpoints.

## JWT Common-Path Middleware (Story 4.2)

Implemented as BRRTRouter middleware between router and handler.

```
Client Request
  -> BRRTRouter Router (path matching)
    -> JWT Common-Path Middleware
      -> If jwt-only: validate JWT + evaluate claims → allow/deny
      -> If jwt-with-fallback or online-only: validate JWT → continue to handler
    -> Handler (business logic)
```

For `jwt-only` routes, the middleware:
1. Extracts Bearer token
2. Validates JWT (typ=at+jwt, iss, aud, exp, nbf, signature via JWKS)
3. Looks up RoutePolicy by path + method
4. Evaluates local policy: tenant match, role check, permission check
5. Returns `AuthDecision::Allowed` or `AuthDecision::Denied`

**Tenant validation is critical:** `claims.tenant_id == X-Tenant-ID` — mismatch = 401, checked before handler.

## Route-Specific Authorization (Story 4.4)

### Six Route Types

| Route Type | Strategy | Decision Logic |
|------------|----------|---------------|
| Login, callback, OTP | Server-side/session | Creates trust, doesn't evaluate it |
| Self-service reads | JWT common path | Ownership: `claims.sub == user_id` |
| Self-service low-risk writes | JWT + optional fallback | Ownership from JWT, business validation online |
| Identity resolution | Hybrid | Tenant from JWT, data-integrity via authz-core |
| API key lifecycle | Hybrid + central | Key validation + revocation check always online |
| Delegated/admin | Hybrid + act + version | `act` claim, version bump, online admin check |

### Key Decision: Login Routes Are Not Authz-Protected

Login routes (`/auth/login`, `/auth/callback/*`, `/auth/verify/*`) are handled by server-side session logic. They CREATE trust, not evaluate it. No JWT middleware evaluation occurs on login routes — authentication IS the authorization.

## Selective Online Fallback (Story 4.3)

For `jwt-with-fallback` routes, if JWT claims don't cover the decision:
1. Check Redis cache: `authz_fallback:{blake3_hash}`
2. Cache hit → return cached result
3. Cache miss → call authz-core → cache result with per-route TTL

### Cache TTL Per Route

| Route | TTL | Rationale |
|-------|-----|-----------|
| preferences PUT | 30s | Low-risk write |
| email/upsert PUT | 15s | Data integrity |
| users/me PUT | 30s | User update |
| users/query POST | 15s | Admin query |

### Cache Miss Storm Mitigation

Single-flight pattern: only one request hits authz-core for a given cache key; others wait for the result.

### Performance Impact

```
baseline_authz_qps = R
hybrid_authz_qps = (R × f) + T
reduction = 1 - hybrid_authz_qps / baseline_authz_qps
```

| R | f | Reduction |
|---|---|-----------|
| 10,000 rps | 0.5% | 99.3% (70 rps) |
| 10,000 rps | 2% | 97.8% (220 rps) |

### Monitoring

- Alert on fallback ratio > 5%
- Per-route: `authz_fallback_total{route}`, `authz_fallback_ratio`
- Latency: `authz_fallback_latency_ms`

## RFC 7662 Introspection (Story 4.5 — Optional)

Standards-based token introspection for resource servers that can't validate JWTs:
- `POST /auth/introspect` — requires API key (server-to-server only)
- Fast path: JWT validation via JWKS
- Slow path: Database fallback for unrecognized tokens
- Rate limited: 100 req/min per client
- PII (email, name, phone) NEVER included in response

## Caches

| Cache | TTL | Why |
|-------|-----|-----|
| JWKS | 5 minutes | Low churn |
| Version | 15-60 seconds | Limits central lookups |
| Fallback result | 5-30s per route | Cuts repeated fallback |
| Denylist | Until token exp | Urgent revocations |
| Entitlement snapshot | 30-300 seconds | Avoids large ACLs in tokens |

## Code Anchors

- `microservices/idam/authz-core/impl/src/` — Authorization handler, principal/effective
- `microservices/idam/identity-login-service/impl/src/` — Login handler, JWT signing
- `openapi/identity-login-service/openapi.yaml` — Login response with JWT claims
- `openapi/identity-session-service/openapi.yaml` — Introspection endpoint (Story 4.5)
- `docs/Epics/04-hybrid-authz-model/hybrid.md` — Epic plan
- `docs/Epics/04-hybrid-authz-model/stories/story-4.1.md` — Route classification
- `docs/Epics/04-hybrid-authz-model/stories/story-4.2.md` — JWT middleware
- `docs/Epics/04-hybrid-authz-model/stories/story-4.3.md` — Selective fallback
- `docs/Epics/04-hybrid-authz-model/stories/story-4.4.md` — Route-specific decisions
- `docs/Epics/04-hybrid-authz-model/stories/story-4.5.md` — RFC 7662 introspection

## Gaps / Drift

> **Open:** Route classification YAML file needs to be generated from OpenAPI specs (Story 4.1).
> **Open:** Introspection endpoint (Story 4.5) is optional — only needed for legacy/third-party integrations.
