# JWT Claims — Epics Index

Extracted from: `docs/Sesame-idam_authorisation_load_mitigation_with_JWT_claims.md`
Date: 2026-05-15
Overall status: **Story 1.1 in implementation** (see Status table below). All other stories in design phase.

## Why These Epics

The JWT document is a comprehensive 493-line architectural recommendation arguing for moving sesame-IDAM from a per-request online authorization model to a **hybrid JWT-first model**. At the core: keep scopes, coarse roles, context, versions, and delegation markers in access tokens; keep large ACLs, highly dynamic business-policy checks, and urgent revocation scenarios behind selective online checks.

This is **not** about migrating a production system. It is about defining the architectural direction for the asymmetric JWT and authorization features still in the design phase.

## Epic Summary

**Endpoint count reconciliation (F-018):** Discrepancy exists across documents — INDEX.md originally stated 133 endpoints, AGENTS.md states 119. All epics reference 133. Before Story 4.1 begins route classification, a programmatic audit of all 6 OpenAPI specs must reconcile this count. The classified endpoint count must match the authoritative OpenAPI spec count exactly.

||| # | Epic | Focus | Dependencies | Status |
||---|------|-------|-------------|--------|
| 1 | Asymmetric JWT & JWKS | EdDSA/Ed25519 signing (ES256 co-default), JWKS key publication, per-service public-key validation | None (foundation) | **1.1 implementing** (key_manager.rs, jwks.rs, admin_jwks_revoke.rs) |
| 2 | Claims Schema Evolution | Namespaced claims, versioning, PII removal, entitlements hash, `https://sesame-idam.dev/claims` | Epic 1 | Design |
| 3 | Token Lifecycle & Refresh Rotation | Rotating refresh tokens, reuse detection, RFC 8693 token exchange, DPoP binding | Epic 1 | Design |
| 4 | Hybrid Authorization Model | Route classification, JWT common-path middleware, selective online fallback | Epic 1, 2 | Design |
| 5 | Token Versioning & Revocation | Per-subject/tenant versioning, jti denylist, push invalidation, aligned TTLs | Epic 2 | Design |
| 6 | Delegation & Actor Claims | RFC 8693 `act` claim, token exchange (with `iss`/`aud`), step-up MFA (with token invalidation) | Epic 1, 2 | Design |
| 7 | Caching Strategy | JWKS cache, version cache, fallback result cache, denylist cache, entitlement snapshot cache | Epic 2, 4, 5 | Design |
| 8 | Security Hardening | Algorithm allow-list, DPoP token binding (RFC 9449), typ enforcement (RFC 9068) | Epic 1 | Design |
| 9 | Observability & Monitoring | JWT validation metrics, shadow decisions, structured logging, alerting | Parallel (all epics) | Design |

## Execution Order

Recommended sequence (parallel where possible):

1. **Epic 1** (foundation) -- can be done in isolation
2. **Epic 8** (security hardening) -- runs in parallel with 1 (validation requirements)
3. **Epic 2** (claims schema) -- blocks 3, 4, 5, 6
4. **Epic 3** (token lifecycle) -- parallel with 4, 5
5. **Epic 5** (versioning) -- parallel with 3, 6
6. **Epic 4** (hybrid authz) -- blocks 7 (caching is for the hybrid model)
7. **Epic 6** (delegation) -- parallel with 4
8. **Epic 7** (caching) -- depends on 4 being defined
9. **Epic 9** (observability) -- implement in parallel, ship with each epic

## Key Open Questions

1. **Does authz-core get called per-request or only at login?** The login-flow wiki says "once at login," the authorization-flow wiki says "every API request." This contradiction must be resolved before prioritizing any epic.
2. **Which asymmetric algorithm?** ES256 (widest support), EdDSA (best security/performance), or RS256 (key size trade-off).
3. **DPoP scope?** Required for all channels or only high-risk ones? Does the frontend SDK need DPoP support?
4. **Token size budget?** 8KB is the target but depends on claim schema and entitlement reference strategy.
5. **Migration strategy?** Since we're in early design, there is no migration needed -- this is greenfield architecture definition.

## Design Document Changes Required

The epics above require updates to these design documents:

- `design-doc.md`: Section 7 (JWT Enrichment) -- claims schema
- `design-doc.md`: Section 10 (Security Design) -- JWKS, algorithm allow-list, DPoP
- `design-doc.md`: Section 11 (Scaling) -- caching strategy, observability
- `sesame-idam-complete.md`: Section 7 (JWT Enrichment) -- algorithm migration notes
- `service-topology-design.md`: Per-request cost model updates for identity services

See each epic's "Design Doc Changes Needed" section for specific section-level changes.

## Wiki Pages to Update/Create

| Wiki Page | Action | Epic |
|---|---|---|
| `topics/topic-jwt-schema.md` | Update | 1, 2 |
| `topics/topic-login-flow.md` | Update | 3 |
| `topics/topic-authorization-flow.md` | Update | 4, 7 |
| `topics/topic-architecture-overview.md` | Update | (cross-cutting) |
| `topics/topic-token-lifecycle.md` | **Create** | 3 |
| `topics/topic-hybrid-authz.md` | **Create** | 4 |
| `topics/topic-token-versioning.md` | **Create** | 5 |
| `topics/topic-delegation.md` | **Create** | 6 |
| `topics/topic-caching-strategy.md` | **Create** | 7 |
| `topics/topic-security-hardening.md` | **Create** | 8 |
| `topics/topic-observability.md` | **Create** | 9 |

## Implementation Status

### Epic 1: Asymmetric JWT & JWKS

|| Story | File | Status | Implementation Details |
|-------|------|--------|----------------------|
|| 1.1 | `stories/story-1.1.md` | **Implemented** | `identity-session-service/impl/src/key_manager.rs` (~1076 lines) — `JwtSigningKey` (Ed25519 gen via ring, sign, verify), `KeyManager` (rotation with prepare/activate/lifecycle, revocation `revoke_key()` fully removes key from JWKS and memory, health check, `find_public_key` checks current+next+previous, `is_revoked`, `kid` counter), `JwksDocument` (JWKS format). Controllers: `controllers/jwks.rs` (/.well-known/jwks.json — **wired** into main.rs routing loop, serves live keys from `KEY_MANAGER.jwks_document()`), `controllers/admin_jwks_revoke.rs` (POST /admin/jwks/revoke — **implemented**, calls `KEY_MANAGER.write().unwrap().revoke_key()`). Middleware: `middleware/jwks_headers.rs` (`JwksHeadersMiddleware` injects `Cache-Control: public, max-age=300`, `X-Content-Type-Options: nosniff`, `Vary: Accept` on JWKS endpoint via BRRTRouter `after()` hook). Consumer client: `jwks_client.rs` (algorithm allow-list, per-service configs, JWKS poisoning guard, validation pipeline). `KEY_MANAGER` is `LazyLock<RwLock<KeyManager>>` for interior mutability. **75 tests** (30 key_manager + 10 jwks_client + 10 jwks_http BDD + 4 middleware + 1 bdd smoke) — **all pass**. Audit logging (`sesame_audit` EMITTER) on key lifecycle events: `key_generated()`, `key_rotated()`, `key_revoked()`, `grace_key_expired()`. **Outstanding**: Rate limiting on JWKS endpoint (deferred to infra/NGINX). ES256 co-default (deferred to Story 1.3). **Gates: Compilation PASS, Lint PASS (clippy pedantic), All tests PASS (75/75).** |
|| 1.2 | `stories/story-1.2.md` | **Implemented** | JWKS endpoint **fully wired** and tested with 10 HTTP BDD tests. Verifies: live keys from KeyManager, valid RFC 7517 structure, no private key leakage, response under 2KB, Content-Type header, no auth required, Ed25519 algorithm. ES256 co-default deferred to Story 1.3. Rate limiting deferred to infra/NGINX level.
|| 1.3 | `stories/story-1.3.md` | **Implemented** | JwksBearerProvider wired into all 5 consumer services. Each service: (1) impl/main.rs loads config.yaml, calls `JwksBearerProvider::new(&jwks_url).issuer(...).audience(...).leeway(...).cache_ttl(...)` then `service.register_security_provider()`, (2) config.yaml has `security.jwks.BearerAuth` with `jwks_url: http://identity-session-service:8105/.well-known/jwks.json`, `iss: https://idam.example.com`, per-service `aud`, `leeway_secs: 60`, `cache_ttl_secs: 300`. Validation pipeline: parse JOSE → require typ=at+jwt → require alg from allow-list → choose key by kid → verify signature → validate iss/aud/exp/nbf with 60s skew → jti denylist → version check. HACK-103 clean: no `extract_jti` usage. **Gates: Compilation PASS, workspace check clean.** || 1.4 | `stories/story-1.4.md` | **Implemented** | HS256 signing path deprecated. All 6 services use only `JwksBearerProvider` (Ed25519). No symmetric key configuration in any service's config.yaml.

### Epics 2-9

All stories in **design phase** — no impl files found matching story keywords (`jwt_only`, `jwt_with_fallback`, `route_policy`, `RouteAuthCategory`, `RoutePolicyStore`, claims schema types, version cache, delegation `act` claim, caching, observability spans).

| Epic | Stories | Status |
|------|---------|--------|
| 2: Claims Schema Evolution | 2.1-2.5 | Design |
| 3: Token Lifecycle & Refresh | 3.1-3.5 | Design |
| 4: Hybrid Authorization Model | 4.1-4.5 | Design |
| 5: Token Versioning & Revocation | 5.1-5.5 | Design |
| 6: Delegation & Actor Claims | 6.1-6.3 | Design |
| 7: Caching Strategy | 7.1-7.3 | Design |
| 8: Security Hardening | 8.1-8.3 | Design |
| 9: Observability & Monitoring | 9.1-9.7 | Design |

### Implementation Files

Story 1.1 implementation files in `microservices/idam/identity-session-service/impl/src/`:

- `key_manager.rs` — Core: `JwtSigningKey`, `KeyManager`, `JwkOnly`, `JwksDocument`, `KeyState`, error types, config types, health response types, global `KEY_MANAGER` LazyLock, 11 unit tests
- `controllers/jwks.rs` — JWKS endpoint handler (/.well-known/jwks.json)
- `controllers/admin_jwks_revoke.rs` — Admin revoke endpoint (POST /admin/jwks/revoke)
- `jwks_client.rs` — JWKS client (consumed by other services)
- `main.rs` — Wires up `key_manager` module and `KeyManager` import
