# JWT Claims — Epics Index

Extracted from: `docs/Sesame-idam_authorisation_load_mitigation_with_JWT_claims.md`
Date: 2026-05-15
Status: Design phase -- no code changes

## Why These Epics

The JWT document is a comprehensive 493-line architectural recommendation arguing for moving sesame-IDAM from a per-request online authorization model to a **hybrid JWT-first model**. At the core: keep scopes, coarse roles, context, versions, and delegation markers in access tokens; keep large ACLs, highly dynamic business-policy checks, and urgent revocation scenarios behind selective online checks.

This is **not** about migrating a production system. It is about defining the architectural direction for the asymmetric JWT and authorization features still in the design phase.

## Epic Summary

| # | Epic | Focus | Dependencies |
|---|------|-------|-------------|
| 1 | Asymmetric JWT & JWKS | EdDSA/Ed25519 signing (ES256 co-default), JWKS key publication, per-service public-key validation | None (foundation) |
| 2 | Claims Schema Evolution | Namespaced claims, versioning, PII removal, entitlements hash, `https://sesame-idam.dev/claims` | Epic 1 |
| 3 | Token Lifecycle & Refresh Rotation | Rotating refresh tokens, reuse detection, RFC 8693 token exchange, DPoP binding | Epic 1 |
| 4 | Hybrid Authorization Model | Route classification, JWT common-path middleware, selective online fallback | Epic 1, 2 |
| 5 | Token Versioning & Revocation | Per-subject/tenant versioning, jti denylist, push invalidation, aligned TTLs | Epic 2 |
| 6 | Delegation & Actor Claims | RFC 8693 `act` claim, token exchange (with `iss`/`aud`), step-up MFA (with token invalidation) | Epic 1, 2 |
| 7 | Caching Strategy | JWKS cache, version cache, fallback result cache, denylist cache, entitlement snapshot cache | Epic 2, 4, 5 |
| 8 | Security Hardening | Algorithm allow-list, DPoP token binding (RFC 9449), typ enforcement (RFC 9068) | Epic 1 |
| 9 | Observability & Monitoring | JWT validation metrics, shadow decisions, structured logging, alerting | Parallel (all epics) |

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
