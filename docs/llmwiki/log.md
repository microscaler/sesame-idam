# LLM Wiki — Session Log

## [2026-05-16] Epic 5 Token Versioning & Epic 6 Delegation — Full Enrichment

### Summary

Completed comprehensive testing enrichment for all stories in Epic 5 (Token Versioning) and Epic 6 (Delegation/Act). Each story received individual, hand-written test sections covering Unit, Integration/BDD, Security Regression, Edge Cases, and Cleanup.

### Stories Enriched

| Story | File | Unit | Integration | Security | Edge | Total |
|-------|------|------|-------------|----------|------|-------|
| 5.1 | `Epics/05-token-versioning/stories/story-5.1.md` | 22 | 8 | 5 | 7 | 42 |
| 5.2 | `Epics/05-token-versioning/stories/story-5.2.md` | 28 | 10 | 6 | 8 | 52 |
| 5.3 | `Epics/05-token-versioning/stories/story-5.3.md` | 26 | 10 | 7 | 8 | 51 |
| 5.4 | `Epics/05-token-versioning/stories/story-5.4.md` | 29 | 10 | 7 | 9 | 56 |
| 5.5 | `Epics/05-token-versioning/stories/story-5.5.md` | 31 | 10 | 7 | 8 | 56 |
| 6.1 | `Epics/06-delegation-act/stories/story-6.1.md` | 58 | 17 | 12 | 11 | 98 |
| 6.2 | `Epics/06-delegation-act/stories/story-6.2.md` | 33 | 14 | 12 | 10 | 69 |
| 6.3 | `Epics/06-delegation-act/stories/story-6.3.md` | 43 | 14 | 12 | 11 | 80 |

### Epic 5 — Token Versioning

**Story 5.1 (ver claim):** Tests cover JWT payload claim verification (uint64 type, not string), Redis version tracking (GET defaults to 0, INCR atomicity, SET with correct TTL), version bump on authz changes, fail-open on Redis unavailability, sid uniqueness.

**Story 5.2 (version cache):** Tests cover 15s subject TTL vs 60s tenant TTL, version comparison (claims.ver >= cached_ver), route classification gating (jwt-only skips, high-risk checks both), cache miss defaults, concurrent INCR atomicity, service restart survival, TTL expiry reset.

**Story 5.3 (jti denylist):** Tests cover denylist add with TTL matching token exp, local LRU cache hit/miss behavior, Redis lookup fallback, auto-expire via Redis TTL, denylist NOT checked for jwt-only/jwt-with-fallback routes, metrics emission, cache capacity eviction, parallel coexistence of entries.

**Story 5.4 (push invalidation):** Tests cover Redis pub/sub subscribe on startup, message parse/extract, local cache update on event receipt (subject vs tenant keys), reconnection after disconnect, missed event handling (fire-and-forget), multiple sequential events, metrics (version_bump_total, revocation_propagation_seconds), malformed event handling, concurrent event thread safety.

**Story 5.5 (version mismatch):** Tests cover HTTP 401 response format, WWW-Authenticate header, Retry-After header and JSON body, gap-based retry_after calculation (1-10 → 300s, >100 → 0s), client refresh-and-retry flow, large gap immediate re-auth, jwt-only routes bypass, metrics recording.

### Epic 6 — Delegation/Act

**Story 6.1 (RFC 8693 Token Exchange):** Tests cover RFC 8693 compliance (grant_type validation, subject_token parsing for JWT/API key/refresh token, actor_token optional), can_delegate logic (platform_admin, org_admin same/different org, service_account delegate:*), scope intersection (3-way: subject ∩ requested ∩ actor), act claim inclusion/exclusion, act.chain for nested delegation, tenant match validation, F-003 (iss/aud/iat in response), F-012 (merged audiences), F-021 (CSRF documentation), metrics emission.

**Story 6.2 (Support Impersonation):** Tests cover support_agent role requirement, cross-tenant blocking, org assignment validation, impersonation token structure (act claim, impersonated_by, impersonation_scope), admin action denial, token exchange denial, short TTL (2-5 min), audit log writing, user notification, role revocation mid-impersonation, password change during impersonation.

**Story 6.3 (Step-Up MFA):** Tests cover sx.mfa_verified claim, 6 MFA-protected actions (admin:create_org, org:config:update, admin:impersonate, api_key:create, api_key:revoke, role:assign), /auth/step-up/mfa endpoint validation, mfa_type strength (F-016: SMS blocked for high-consequence, TOTP/WebAuthn allowed), F-006 fix (old refresh token denylisted on step-up), TOTP time window, rate limiting, WebAuthn device registration.

### Wiki Pages Created

| File | Change |
|------|--------|
| `topics/topic-token-versioning.md` | **Created.** Documents ver claim design, version storage in Redis, version validation flow, TTL strategy, version bump on authz change, version mismatch handling |
| `topics/topic-delegation.md` | **Created.** Documents RFC 8693 token exchange, act claim structure, delegation chain, actor can_delegate logic, support impersonation flow, step-up MFA, mfa_type strength requirements |
| `topics/topic-mfa.md` | **Created.** Documents sx.mfa_verified claim, step-up MFA flow, mfa_type strength table, F-006 refresh token invalidation, F-016 SMS restriction |

### Commits

- `72edd2f` — docs(wiki): create topic-token-versioning.md with ver claim design, version storage, validation flow, TTL strategy
- `84894cb` — feat(stories): enrich Epic 6 stories with testing requirements (Story 6.1, 6.2, 6.3)
- `b5142b1` — feat(stories): enrich Story 5.5 with testing requirements
- `5ac7899` — feat(stories): enrich Story 5.4 with testing requirements
- `2265725` — feat(stories): enrich Story 5.3 with testing requirements
- `0ea8359` — feat(stories): enrich Story 5.2 with testing requirements
- `b2566e8` — feat(stories): enrich Story 5.1 with testing requirements

### Current Epic 5 Status

| Story | Testing Enriched | Wiki Updated |
|-------|-----------------|-------------|
| 5.1 | ✅ (committed b2566e8) | topic-token-versioning ✅ |
| 5.2 | ✅ (committed 0ea8359) | topic-token-versioning ✅ |
| 5.3 | ✅ (committed 2265725) | topic-token-versioning ✅ |
| 5.4 | ✅ (committed 5ac7899) | topic-token-versioning ✅ |
| 5.5 | ✅ (committed b5142b1) | topic-token-versioning ✅ |

### Current Epic 6 Status

| Story | Testing Enriched | Wiki Updated |
|-------|-----------------|-------------|
| 6.1 | ✅ (committed 84894cb) | topic-delegation ✅ |
| 6.2 | ✅ (committed 84894cb) | topic-delegation ✅ |
| 6.3 | ✅ (committed 84894cb) | topic-mfa ✅ |

---

## [2026-05-16] Epic 4 Hybrid Authz — Story 4.4, 4.5 Enrichment + Wiki Update

### Summary

Continued enrichment of Epic 4 (Hybrid Authorization Model) stories. Enriched Story 4.4 (Route-Specific Authorization Decisions) and Story 4.5 (RFC 7662 Introspection) with comprehensive testing sections. Updated wiki pages to reflect the complete hybrid model.

### Stories Enriched

| Story | File | New Test Sections |
|-------|------|------------------|
| 4.4 | `Epics/04-hybrid-authz-model/stories/story-4.4.md` | Unit (21 tests), Integration (9 scenarios), Security Reg (6 tests), Edge (7 tests), Cleanup (7 items) |
| 4.5 | `Epics/04-hybrid-authz-model/stories/story-4.5.md` | Unit (11 tests), Integration (9 scenarios), Security Reg (6 tests), Edge (7 tests), Cleanup (7 items) |

### Story 4.4 — Route-Specific Authorization Decisions

Key test areas covered:
- **Self-service reads:** ownership check pass/fail (claims.sub == user_id)
- **Self-service writes:** ownership + business validation trigger/skip
- **Identity resolution:** tenant validation, permission check, always-online data-integrity
- **API key lifecycle:** tenant mismatch, revocation, valid key acceptance
- **Delegated actions:** act claim presence, version mismatch, normal risk skip
- **Route classification:** login routes NOT in middleware, read routes as jwt-only, identity as hybrid
- **Security:** login routes cannot be used as JWT authz entry points, ownership claim forgery, act claim privilege escalation prevention, email upsert always verifies via authz-core

### Story 4.5 — RFC 7662 Introspection

Key test areas covered:
- **Active/inactive responses:** valid JWT, expired, revoked, invalid signature
- **PII protection:** username field always None in introspection response
- **Authz:** API key required (not Bearer), rate limiting, enumeration prevention
- **Edge cases:** empty token, oversized token (>64KB), malformed JOSE, concurrent same-token
- **Cross-issuer fallback:** JWT from unknown issuer falls back to DB lookup

### Wiki Updates

| File | Change |
|------|--------|
| `topics/topic-hybrid-authz.md` | **Created.** 6-section hybrid model doc: route categories, middleware, route-specific decisions, selective fallback, RFC 7662 introspection, caches |
| `topics/topic-authorization-flow.md` | **Rewritten.** Expanded from ~60 lines to ~300 lines. Added: hybrid model overview, route classification table, JWT middleware, route-specific decisions (Story 4.4), selective fallback (Story 4.3), RFC 7662 (Story 4.5), cache strategy, performance impact |
| `topics/topic-login-flow.md` | **Updated.** Added 2 key points: login routes NOT protected by JWT authz, and hybrid post-login model |
| `index.md` | **Updated.** Added topic-hybrid-authz entry to Topics table |

### Commits

- `b7560de` — docs(wiki): update authorization-flow and login-flow with Epic 4 hybrid authz model
- `236e2b0` — feat(stories): enrich Story 4.5 with testing requirements
- `0cf925a` — feat(stories): enrich Story 4.4 with testing requirements

### Current Epic 4 Status

| Story | Testing Enriched | Wiki Updated |
|-------|-----------------|-------------|
| 4.1 | ✅ (committed 2198fc1) | topic-authorization-flow ✅ |
| 4.2 | ✅ (committed 2198fc1) | topic-authorization-flow ✅ |
| 4.3 | ✅ (committed 2198fc1) | topic-authorization-flow ✅ |
| 4.4 | ✅ (committed 0cf925a) | topic-authorization-flow ✅, topic-hybrid-authz ✅ |
| 4.5 | ✅ (committed 236e2b0) | topic-hybrid-authz ✅ |

---

## [2026-05-15] Tiltfile Configmap Fix — Namespace + binary_name

### Summary

All 6 sesame-idam pods were stuck in `ContainerCreating` because the Tiltfile was missing two critical elements that prevented Helm from creating ConfigMaps:

1. **`binary_name` undefined variable** — `create_microservice_deployment()` referenced `binary_name` on line 304 (live_update sync path) but never defined it. Starlark crash.
2. **Missing `k8s_yaml('k8s/microservices/namespace.yaml')`** — namespace `sesame-idam` didn't exist when Tilt tried to apply Helm manifests, so configmaps were never created.

Hauliage has both of these. Sesame-IDAM's Tiltfile rewrite missed them.

### Root Cause Analysis

**Error 1 — Starlark crash:**
```
ERROR: Tiltfile:304:45: undefined: binary_name (did you mean image_name?)
```
Line 304 in the old Tiltfile: `sync(artifact_path, '/app/%s' % binary_name)` — the hauliage pattern defines `binary_name = name.replace('-', '_')` at the top of `create_microservice_deployment()`, but sesame-idam didn't. This caused Tilt to crash when processing `custom_build` for each service, preventing k8s_yaml from ever being applied.

**Error 2 — Namespace missing:**
```
ERROR: namespaces "sesame-idam" not found
```
The Tiltfile had no `k8s_yaml('k8s/microservices/namespace.yaml')` call. Hauliage creates this at module level in the Data Infrastructure section. Without it, Helm couldn't create ConfigMaps in a non-existent namespace.

**Consequence:** Helm templates rendered correctly (`helm template` works fine), but Tilt never applied them to the cluster. Pods were created (via `k8s_resource` auto-creation) but failed to mount configmaps (`configmap "org-mgmt-config" not found`).

### Fix Applied

```python
# In create_microservice_deployment(), alongside package_name:
binary_name = name.replace('-', '_')

# In Data Infrastructure section:
k8s_yaml('k8s/microservices/namespace.yaml')
```

### Verification

All 6 pods Running, all 6 configmaps created, all returning HTTP 200 on `/health`:
- org-mgmt (8104) — 200 ✅
- authz-core (8102) — 200 ✅
- api-keys (8103) — 200 ✅
- identity-login-service (8101) — 200 ✅
- identity-session-service (8105) — 200 ✅
- identity-user-mgmt-service (8106) — 200 ✅

### Files Updated

- `Tiltfile` — added `binary_name` variable + `k8s_yaml('k8s/microservices/namespace.yaml')`
- `docs/PRD-SEASAME-AUDIT-REMEDIATION.md` — added section 6b documenting the issues and resolution

---

## [2026-05-14] Phase 0b: Tiltfile Lint Path Fix + Wiki Update

### Summary

Fixed the Tiltfile `create_microservice_lint()` and `create_microservice_gen()` functions to use full YAML file paths (`openapi/idam/<service>/openapi.yaml`) instead of directory paths. Also updated the llmwiki to reflect the current correct state of sesame-idam infrastructure.

### Tiltfile Fixes

- `create_microservice_lint()`: Changed `--spec ./openapi/idam/%s` to `--spec ./openapi/idam/%s/openapi.yaml` (brrtrouter-gen needs the file path, not the directory)
- `create_microservice_lint() deps`: Changed `./openapi/idam/%s` to `./openapi/idam/%s/openapi.yaml`
- `create_microservice_gen() deps`: Changed `./openapi/idam/%s` to `./openapi/idam/%s/openapi.yaml`

### Wiki Updates

| File | Change |
|------|--------|
| `index.md` | Updated topic-architecture-overview description to note `cargo check --workspace` passes |
| `topics/topic-remediation-plan.md` | Phase 0 and Phase 1 marked ✅ Completed; build warnings documented; acceptance criteria updated |
| `topics/topic-build-infrastructure.md` | Status → verified; added build status table; Phase 2 items moved to "Planned" |
| `topics/topic-package-naming-convention.md` | Status → verified; documented final naming table; removed "target" section since fix is complete |
| `topics/topic-tiltfile-architecture.md` | Status → verified; documented current Tiltfile architecture and design decisions |
| `topics/topic-brrtrouter-codegen.md` | Fixed duplicate OpenAPI layout section; noted `openapi/idam/` nesting |

### Current Build State

- `cargo check --workspace` — ✅ 0 errors, 31 warnings
- `cargo test --workspace` — ✅ 5 tests (4 unit + 1 doc)
- `brrtrouter-gen lint` — ✅ All 6 specs pass (authz-core + identity-user-mgmt-service fixed)
- Tiltfile — ✅ Validated Starlark syntax, all path refs corrected

### OpenAPI Lint Fixes

Fixed `operation_id_casing` errors in 2 specs:

| Spec | Issues Fixed |
|------|-------------|
| `authz-core` | 10 operationIds (camelCase → snake_case) + added missing `PaginatedResponse` schema |
| `identity-user-mgmt-service` | 3 operationIds (getUserAuditEvents, exportUserAuditEvents, getUserEventCount → snake_case) |

### Codegen State After Fixes

All 18 camelCase operationIds now use snake_case convention. All specs define all referenced schemas.

---

## [2026-05-14] Sesame-IDAM Structural Audit — Wiki Updated from PRD

### Summary


[... content preserved from original ...]

---
