# Epic 4: Hybrid Authorization Model

## Summary

Implement the hybrid authorization model described in the JWT document: JWT claims handle the common path (coarse-grained checks), with a lightweight online fallback for high-risk, dynamic, or high-cardinality decisions. Route classification determines which path is used per endpoint.

## Why This Epic Is Needed

The JWT document's core thesis: "not put all permissions in JWTs and delete online checks" but rather a "hybrid model" where JWT claims are used for the common path and online fallback is reserved for high-risk writes, admin actions, delegated actions, and high-cardinality resource ACLs. This is the single most impactful change for reducing authz-core load.

## Current State

Two auth levels are documented:
- **Coarse-grained**: JWT claims directly -- "Is Admin?" -- "Has invoices:write?" -- Zero latency
| **Fine-grained**: `POST /authz/authorize` with action + resource context -- ABAC rules -- Cached in Redis 30s TTL

The topology design says authz-core is called on **every** consumer API request. But the login flow says it's called **once at login** for JWT enrichment. This contradiction needs resolution.

## Stories

- [ ] Story 4.1: Classify routes into auth path categories
  - Define 3 route categories: `jwt-only`, `jwt-with-fallback`, `online-only`
  - Audit all 133 endpoints and assign them to a category
  - Store classification in a route policy table or config

- [ ] Story 4.2: Implement JWT common-path authorization middleware
  - Gateway-level middleware that validates JWT (typ, iss, aud, exp, signature)
  - Extracts claims from validated JWT
  - Evaluates local policy from `scope`, `roles`, `permissions`, and `tenant` context
  - Returns allow/deny without calling authz-core

- [ ] Story 4.3: Implement selective online fallback
  - For `jwt-with-fallback` routes: if JWT claims don't cover the decision, call authz-core
  - Cache fallback results in Redis (5-30s TTL per document)
  - Track fallback ratio for monitoring

- [ ] Story 4.4: Implement route-specific authorization decisions
  | Route Type | Strategy | Rationale |
  |---|---|---|
  | Login, callback, OTP | Server-side/session logic | Not the authz bottleneck |
  | Self-service reads (users/me, preferences GET) | JWT common path | Ownership checks are stable |
  | Self-service low-risk writes (preferences PUT) | JWT + optional fallback | Business validation stays online |
  | Identity resolution (email/upsert, user lookup) | Hybrid | Cross-service, hot paths need freshness |
  | API key lifecycle (api-keys/validate) | Hybrid, leaning central | Revocation wants freshness |
  | Delegated/admin actions | Hybrid with `act`, step-up, version | High consequence if stale |

- [ ] Story 4.5: Implement RFC 7662 introspection endpoint (optional)
  - Standards-based fallback endpoint for token validation
  - Not currently visible in public API
  - Can be added as a future enhancement

## OpenAPI Changes Needed

- The global `ApiKeyHeader` security requirement needs to be changed to support `bearer` JWT + JWKS for endpoints that will use JWT common-path authz
- Routes in `jwt-with-fallback` and `online-only` categories need explicit documentation of their fallback behavior

## Design Doc Changes Needed

- `design-doc.md`: Update the "Authentication & Authorization" section to document the hybrid model
- `design-doc.md`: Add route classification table
- `design-doc.md`: Update the authz-core service description to include fallback caching
- Wiki: Update `topics/topic-authorization-flow.md` with hybrid model details
- Wiki: Create `topics/topic-hybrid-authz.md` (new)

## Gaps in the JWT Document

- The document provides a decision matrix by endpoint type but doesn't specify how to implement the classification in code.
- Does not define the `RoutePolicy` struct or how route-specific policies are loaded (config file, database, inline).
- Does not address the shadow-decision mode for migration (comparing online vs local decisions on the same traffic).
- The recommendation of 30s TTL for fallback authz result cache seems aggressive for high-risk routes. Should have per-route TTL.

## Dependencies

- Depends on Epic 1 (JWKS) and Epic 2 (Claims Schema) for JWT validation
- Blocks nothing but is the primary authz load-reduction mechanism
