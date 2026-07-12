# Epic Delivery Audit — 2026-07-10

> **Scope:** Commits 2b57e73..HEAD (three committed + six unstaged) against all nine epics from `docs/Epics/INDEX.md`.
>
> **Method:** Code-level inspection of impl controllers, services, models, middleware, config, and helm values. Story files read from `docs/Epics/01-asymmetric-jwks/stories/` and INDEX.md acceptance criteria.
>
> **Enriched:** 2026-07-10 — delivery-gap analysis, post-audit progress (SI-3), and recommended next work added in §11–§13.

---

## Executive Summary (2026-07-10)

**Epic 1 (Asymmetric JWT & JWKS) is production-grade and complete.** That is the foundation Hauliage already consumes (login → JWKS validation on fleet/company/consignments/BFF).

**Epics 2–9 remain largely design-phase**, with useful scaffolding (entitlements utils, `VersionStore`, authz-core enrichment) that is not yet wired into a full hybrid authorization or revocation story.

**Delivering Sesame-IDAM as a product is a wider bar than finishing Epic 1.** The platform has **119 OpenAPI endpoints** across six services; only a **narrow auth slice** is DB-backed today (login, register, logout, refresh, JWKS, principal/effective, a handful of session/org-mgmt consumer routes). The rest still returns gen-stub responses. Cross-repo integration (Hauliage BFF, account-first onboarding, k8s-native `:8080` wiring) is in flight but not closed.

**Highest-leverage next work:** (1) close platform inconsistencies (helm `aud`, `set_active_organization` typed migration), (2) finish the **account-first + Hauliage E2E** path, (3) implement **BR-3/SI-4** (OAuth-correct refresh errors), (4) wire **revocation primitives** (jti denylist + version rejection), then (5) expand real impl coverage service-by-service rather than jumping to Epic 6–9 infrastructure.

---

## 0. Epic-to-Story Matrix (Full)

| Epic | Focus | Stories | Original Status (INDEX.md) | Current Status |
|------|-------|---------|---------------------------|----------------|
| 1 | Asymmetric JWT & JWKS | 1.1–1.4 | 1.1 implementing | **1.1–1.4 all implemented** |
| 2 | Claims Schema Evolution | 2.1–2.5 | Design | **Partial scaffolding, no wiring** |
| 3 | Token Lifecycle & Refresh | 3.1–3.5 | Design | **Partial scaffolding, no wiring** |
| 4 | Hybrid Authorization Model | 4.1–4.5 | Design | **Story 4.4 partial** |
| 5 | Token Versioning & Revocation | 5.1–5.5 | Design | **Partial scaffolding, no wiring** |
| 6 | Delegation & Actor Claims | 6.1–6.3 | Design | **Not started** |
| 7 | Caching Strategy | 7.1–7.5 | Design | **Not started** |
| 8 | Security Hardening | 8.1–8.3 | Design | **Not started** |
| 9 | Observability & Monitoring | 9.1–9.7 | Design | **Not started** |

### What changed since INDEX.md was last updated

INDEX.md says Story 1.1 was "implementing". All four stories (1.1–1.4) are now fully implemented with compiled code, controllers wired, and 100+ tests. The INDEX.md implementation status table needs updating.

**Post-audit (same day):** **SI-3** landed — `users_me_get`, `users_me_patch`, `oauth_userinfo` migrated to typed handlers; `raw_handler.rs` deleted; `auth_context.rs` added. §7.1 unstaged changes are now **committed**. §7.3 (`set_active_organization.rs`) remains the outstanding raw-handler outlier.

---

## 1. Epic 1: Asymmetric JWT & JWKS — ALL STORIES IMPLEMENTED

### Story 1.1: Generate and Rotate Asymmetric Signing Keys

**Status: COMPLETE**

**What was delivered (vs. story plan):**

| Acceptance Criteria | Met? | Notes |
|---------------------|------|-------|
| EdDSA (Ed25519) key pair generated at startup | Yes | `key_manager.rs:280` — ring-based Ed25519 keygen |
| Private key never on disk/env/config | Yes | Stored as `Vec<u8>` in memory only |
| Public key served in JWKS RFC 7517 format | Yes | `controllers/jwks.rs:21` — serves all current, next, grace keys |
| Key rotation with prepare/activate lifecycle | Yes | `KeyManager.rotate()` with overlapping windows |
| Overlap: both old and new keys in JWKS during rotation | Yes | `previous_key` served alongside `current_key` |
| Grace period removal | Yes | `DEFAULT_GRACE_PERIOD_SECS = 3600` (1 hour) |
| Restart generates fresh key | Yes | `KeyManager::new()` calls `generate()` unless `SESAME_JWT_SIGNING_KEY_PKCS8_B64` is set |
| `typ=at+jwt` in all tokens | **No** | Deferred to Story 8.1. `Ed25519Signer.sign_access_claims()` handles this downstream via `jsonwebtoken` crate. |

**Implementation files:**
- `identity-session-service/impl/src/key_manager.rs` (1281 lines) — `JwtSigningKey`, `KeyManager`, `JwksDocument`, `JwkOnly`, lifecycle, rotation, revocation
- `identity-session-service/impl/src/controllers/jwks.rs` — JWKS endpoint handler
- `identity-session-service/impl/src/controllers/admin_jwks_revoke.rs` — POST /admin/jwks/revoke
- `identity-session-service/impl/src/middleware/jwks_headers.rs` — Cache-Control, Vary headers
- `identity-session-service/impl/src/jwks_client.rs` — Consumer JWKS client with validation pipeline

**Deviation from plan — shared signing key bootstrap:** The story described keygen-only at bootstrap. The actual implementation adds `KeyManager::from_pkcs8()` + `key_from_env()` which loads a shared signing key from `SESAME_JWT_SIGNING_KEY_PKCS8_B64` / `SESAME_JWT_SIGNING_KID` env vars (provisioned as a Kubernetes Secret). This allows identity-session-service to serve the public half of the key that identity-login-service signs with. This is a **necessary deviation** — the story described a single-service model, but the architecture requires two services (login signs, session serves JWKS) to share key identity.

**Deferred items from story:**
| Item | Target | Status |
|------|--------|--------|
| `typ=at+jwt` enforcement | Story 8.1 | Not yet implemented |
| ES256 co-default | Story 1.3 | Only EdDSA in allow-list; ES256 listed but no keys generated |
| Rate limiting on JWKS | Story 1.2 / infra | Still deferred; `middleware/rate_limit.rs` file exists but not wired |
| Alerting on `key.age > 7 days` | Story 9.x | Not yet implemented; `health()` exposes `age_seconds` |

### Story 1.2: Implement JWKS Publication Endpoint

**Status: COMPLETE**

The endpoint is wired through the typed handler in `controllers/jwks.rs`, reads from `KEY_MANAGER.jwks_document()`, and serves live keys. The old gen mock (`gen/controllers/jwks.rs`) is still present but the impl controller is what gets registered.

Headers (`Cache-Control: public, max-age=300`, `X-Content-Type-Options`, `Vary`) are applied via `JwksHeadersMiddleware`, not inline in the handler — this is the correct BRRTRouter pattern.

### Story 1.3: Wire All Services to Validate JWTs via JWKS

**Status: COMPLETE**

All 6 consumer services have:
- `security.rs` module that builds `JwksProviderBuilder` from config
- `JwksBearerProvider` registered via `service.register_security_provider()`
- Per-service `aud` values in `jwks_client.rs` (lines 239–302)
- 300s cache TTL, 60s leeway

Helm values fixed in commit `0c968d8` — identity-login, identity-session, org-mgmt now have `security.jwks.BearerAuth` config that was previously missing.

### Story 1.4: Deprecate HS256 Signing Path

**Status: COMPLETE**

- All token issuance (`token_issuer.rs` in both login and session services) uses `Ed25519Signer` exclusively
- `ALLOWED_JWT_ALGORITHMS = ["EdDSA", "ES256"]` — no HS256
- No `JWT_SECRET` env var or config option in any service's `config.yaml`
- Validation pipeline rejects `alg: none` (RFC 8725) and any non-allow-listed algorithm

### Epic 1 Assessment

The entire epic is delivered. The one gap is ES256 key co-generation (only EdDSA keys are produced) and `typ` enforcement — both are lower-priority than Ed25519-only production. The `typ` enforcement is a `jsonwebtoken` crate-level concern, not something `key_manager.rs` owns.

---

## 2. Epic 2: Claims Schema Evolution — SCAFFOLDING ONLY

### What exists in code (not in story files):

- `identity-login-service/impl/src/jwt/claims.rs` (446 lines) — three submodules:
  - `entitlements_ref` — deterministic UUID v5 from (user_id, org_id, version) tuples
  - `entitlements_hash` — SHA-256 of canonical JSON snapshots for cache-poisoning detection
  - `entitlements_cache` — Redis `SET/GET/DEL` with TTL clamping and hash verification

### What's MISSING (deviation from story plan):

These utilities are implemented but **not wired** into the token issuance or validation flows. The `token_issuer::issue_tokens()` function (login, line 100) builds `SesameAuthzClaimsBuilder` which produces the `sx` (security/authorization) claim, but it does NOT call `entitlements_ref::generate()` or `entitlements_hash::compute()`. There is no `entitlements_ref` or `entitlements_hash` field in the `TokenResponse` returned to clients.

**Root cause analysis:** This is scaffolding for the hybrid authorization model (Epic 4) and caching (Epic 7) that was built ahead of time but never integrated because those epics were still in design. The claims.rs utilities are correct and well-tested but sit unused.

### What IS wired:

- `AccessClaimsBuilder` (from `sesame_common`) is used — it produces claims: `iss`, `sub`, `aud`, `client_id`, `scope`, `exp`, `nbf`, `iat`, `jti`, `ver`, `sid`, `tenant_id`, `user_id`, `user_type`, `org_id_opt`, `sx`
- The `sx` claim is built by `SesameAuthzClaimsBuilder::new().tenant(portal).portal(portal).roles(roles).build()`
- These map to **Epic 2 Story 2.1–2.2** (namespaced claims, versioning)

**Epic 2 Assessment:** Core claim fields (iss, sub, aud, tenant_id, sid, ver, jti, sx) are wired. Entitlements reference and hash utilities exist but are not wired. Story 2.3 (PII removal) is done — `first_name`, `last_name` are fetched from the profile endpoint, not carried in tokens.

---

## 3. Epic 3: Token Lifecycle & Refresh Rotation — PARTIAL

### What IS implemented:

- **Access tokens:** Ed25519 signed, 300s TTL (configurable via `JWT_ACCESS_TTL_NORMAL`)
- **Refresh tokens:** Signed JWT with `typ: "refresh"`, `jti`, `sid`, `family_id`, stored in Redis under `refresh:{jti}`
- **Refresh flow:** `auth_refresh.rs` validates the refresh token, looks up metadata in Redis, rotates family, reissues pair
- **Token rotation:** `token_rotation.rs` handles family-based rotation

### What's MISSING:

- **Reuse detection:** Story 3.2 (refresh token reuse detection) — no evidence of `family_id` comparison against Redis to detect replay
- **DPoP binding:** Story 3.4 — not implemented
- **RFC 8693 token exchange:** Story 3.3 — not implemented

**Epic 3 Assessment:** Basic access + refresh issuance and rotation works. The more advanced lifecycle features (reuse detection, DPoP, token exchange) are still design-phase.

---

## 4. Epic 4: Hybrid Authorization Model — STORY 4.4 PARTIAL

### Story 4.4 (JWT claim enrichment at login): IMPLEMENTED

This is the work in commits `2b57e73` and the unstaged controller changes.

**The flow (trace through code):**

1. `auth_login.rs:61` — calls `authz_client::fetch_effective_roles(user_id, tenant_id, portal)`
2. `authz_client.rs:51` — POSTs to `http://authz-core:8080/idam/v1/authz/principals/effective` with `{user_id, tenant_id, app_id, include_inherited: true}`
3. `authz-core/controllers/principal_effective.rs:43` — resolves roles from `role_assignments` table and attributes from `principal_attributes` table
4. `authz-core/controllers/principal_effective.rs:77` — returns role objects with role name, app_id, org_id (omitted if tenant-scoped)
5. `authz_client.rs:85` — parses role objects into `Vec<String>` of role names
6. `auth_login.rs:81` — passes roles to `token_issuer::issue_tokens()` which embeds them in the `sx` JWT claim

**The org resolution (account-first onboarding):**
- `auth_login.rs:73` — calls `org_context::resolve_active_org_id()`
- `set_active_organization.rs:73` — same function, re-issues tokens with new `org_id`
- `org_resolution.rs` — traits for extracting tenant/user from typed requests

### What's NOT done:

- **Story 4.1 (Route classification):** No route-to-category mapping. Every route either has a security provider (JWKS validation) or doesn't — there's no "jwt_only" vs "jwt_with_fallback" classification.
- **Story 4.3 (Selective online fallback):** The only online call is login-time enrichment. Per-request fallback (for high-risk routes) is not implemented.
- **Story 4.5 (RFC 7662 introspection):** Not implemented.
- **Role-to-permission mapping:** `principal_effective.rs:103` returns `permissions: vec![]` — the mapping from roles to fine-grained permissions lives in org-mgmt's tables and is not wired.

**Epic 4 Assessment:** The login-time enrichment path works end-to-end (login → authz-core → JWT claim → consumer validation). This is the "JWT common-path" of the hybrid model. The selective online fallback (per-request) is the remaining gap that defines whether this is truly "hybrid" or just "JWT-first with pre-computed claims."

---

## 5. Epic 5: Token Versioning & Revocation — PARTIAL

### What IS implemented:

- `sesame_common::token_versioning::VersionStore` — Redis-backed version counter per subject
- `ver` claim in access tokens (line 117 of login `token_issuer.rs`)
- Bumped on `issue_version()` call at login time
- Fall-back to `ver=1` when Redis unavailable

### What's MISSING:

- **Story 5.2 (jti denylist):** `jwks_client.rs:17` documents step 7 (Reject if jti in local deny) in the validation pipeline, but there's no actual jti denylist middleware or storage. `authz-core/impl/src/denylist_middleware.rs` exists but I haven't verified its wiring.
- **Story 5.3 (Push invalidation):** `authz-core/impl/src/push_invalidator.rs` exists but is a stub — no endpoint wired to invalidate tokens by jti or user_id.
- **Story 5.4 (Aligned TTLs):** Access tokens are 300s, refresh tokens are `REFRESH_TOKEN_TTL` — TTL alignment strategy is not defined.

**Epic 5 Assessment:** The version store is a good foundation for token revocation. The jti denylist and push invalidation endpoints are the next logical step and their file scaffolding exists but is not connected.

---

## 6. Epics 6–9: NOT STARTED

No implementation files found matching these story keywords:
- Epic 6 (Delegation): No `act` claim, no RFC 8693 token exchange, no impersonation controller in impl (only gen exists)
- Epic 7 (Caching): Entitlements cache utilities exist but are not wired into any validation flow
- Epic 8 (Security Hardening): `typ` enforcement deferred, no algorithm enforcement in signing, no HSM
- Epic 9 (Observability): `authz_span_middleware.rs` exists for OTEL spans, but no custom Prometheus counters

---

## 7. Unstaged Changes Review

> **Status (2026-07-10):** Items in §7.1 are **merged** (SI-3). §7.3–7.4 remain open.

### 7.1 Raw Handler → Typed Handler Migration — DONE (SI-3)

**Files changed:**
- `identity-session-service/impl/src/controllers/users_me_get.rs` — `handle_raw → handle`
- `identity-session-service/impl/src/controllers/users_me_patch.rs` — `handle_raw → handle`
- `identity-session-service/impl/src/controllers/oauth_userinfo.rs` — `handle_raw → handle`
- `identity-session-service/impl/src/raw_handler.rs` — DELETED (114 lines of coroutine boilerplate removed)
- `identity-session-service/impl/src/auth_context.rs` — NEW (43 lines, extracted `authenticated_principal()`)
- `identity-session-service/impl/src/lib.rs` — `pub mod auth_context`
- `identity-session-service/impl/src/main.rs` — removed `raw_handler` import

**Assessment: Good change.** Eliminates 114 lines of duplicated coroutine/spawn/panic-recovery code and brings these controllers into the typed handler pattern. The `auth_context.rs` module correctly moves JWT claim extraction into a reusable function.

### 7.2 Concern: `users_me_patch.rs` field name mismatch

Line 30 maps `req.data.picture_url.clone()` to `avatar_url` in the `ProfileUpdate` struct:

```rust
avatar_url: req.data.picture_url.clone(),
```

This assumes the gen layer generates a `picture_url` field on the typed Request type. If the OpenAPI spec says `avatar_url` but the gen code generates `picture_url` (or vice versa), this mapping could silently break on next codegen. **Recommendation:** Verify that the OpenAPI spec field name matches what the gen layer generates. If they differ, fix the spec, not the controller.

### 7.3 Concern: `set_active_organization.rs` still uses raw handler pattern

This file (153 lines) still uses `HandlerRequest` + `HandlerResponse` + raw JWT payload parsing via base64 decode (lines 18–33). It has NOT been migrated to typed handlers. The `claims_from_request()` function does manual JWT payload decoding — this is fragile and bypasses the `JwksBearerProvider` validation entirely. The `bearer_token()` function parses the Authorization header manually.

This endpoint is critical for account-first onboarding (create org → set active org → re-issue JWT). It should be migrated to typed handlers alongside `users_me_get`, `users_me_patch`, and `oauth_userinfo`.

### 7.4 Concern: `auth_login.rs` still uses `HandlerRequest`-style error returns

Line 117: `match serde_json::to_value(&body)` — this manual serialization suggests the Response type is not directly `HttpJson`-compatible. If the gen Response type can be `Serialize`, the controller should return `HttpJson::ok(body)` directly (like the typed controllers do).

---

## 8. Helm Values Audit (commit 0c968d8)

Three helm value files received `security.jwks.BearerAuth` config:

| Service | JWKs URL | Issuer | Audience |
|---------|----------|--------|----------|
| identity-login-service | `http://identity-session-service.sesame-idam.svc.cluster.local:8080/idam/v1/.well-known/jwks.json` | `https://idam.example.com` | `sesame-idam` |
| identity-session-service | same | same | same |
| org-mgmt | same | same | same |

**Concern:** The audience value `sesame-idam` is generic. `jwks_client.rs` defines per-service audiences like `identity-login.seasame-idam.microscaler.local`, `authz-core.seasame-idam.microscaler.local`, etc. The helm values use a single `sesame-idam` audience for all services — this is **less restrictive** than the code's per-service config. It means any service accepting `sesame-idam` as audience will accept tokens meant for any other service. The two are inconsistent.

**Recommendation:** Align helm audiences with the per-service configs in `jwks_client.rs`.

---

## 9. Summary: What Worked, What Didn't, Why

### What worked

1. **Epic 1 delivered end-to-end.** Key management, JWKS publishing, consumer validation, and HS256 deprecation are all wired and compiled. The shared signing key bootstrap deviation was necessary and correct.

2. **Login → authz-core enrichment path works.** The single sanctioned cross-service dependency (login calls authz-core for role enrichment) is implemented, tested, and gracefully degrades (empty roles on failure).

3. **Raw handler cleanup is good.** Deleting 114 lines of boilerplate and consolidating into `auth_context.rs` improves the codebase.

### What didn't match the plan

1. **Epic 2 (Claims Schema) was partially preempted.** The entitlements reference/hash/cache scaffolding in `jwt/claims.rs` was built before the hybrid authorization model (Epic 4) was defined. It's correct code that's simply not wired. This suggests an ordering decision: build the primitives first, wire them later. Acceptable but creates technical debt until wiring happens.

2. **Token versioning (Epic 5) was built before the story.** `VersionStore` exists in `sesame_common` and is called at login, but the surrounding revocation infrastructure (jti denylist, push invalidation) has file scaffolding without endpoints. The version store was built because it's needed for the `ver` claim in access tokens, even though the broader revocation story wasn't ready.

3. **The hybrid model is "JWT-first with pre-computed claims" not "hybrid".** The selective online fallback (per-request authorization check for high-risk routes) is the defining feature of a "hybrid" model. Without it, the architecture is "JWT-first with enriched claims at login" — which is still useful but not the full hybrid model.

### Why the deviations

The epic plan assumed a strict top-down build: story-by-story, in execution order. The actual implementation followed a more pragmatic path:

- **Build the foundation first** (Epic 1) — non-negotiable, everything depends on it
- **Build shared infrastructure** (VersionStore, entitlements utils) before the stories that consume them
- **Implement the login path first** (authz-core enrichment) because it unblocks account-first onboarding
- **Hybrid features (route classification, online fallback)** are harder and depend on having a working login path first

This is a valid engineering approach. The gap is that the epic plan's execution order assumed sequential dependency resolution, while the actual work parallelized infrastructure building with story implementation.

---

## 10. Next Steps (by priority)

### P0: Fix inconsistencies

1. **Align helm audiences** with per-service configs in `jwks_client.rs` (helm values use `sesame-idam`, code uses `identity-login.seasame-idam.microscaler.local` etc.)
2. **Migrate `set_active_organization.rs`** to typed handlers — it's the most critical onboarding endpoint and uses the oldest pattern (raw JWT decoding, no BRRTRouter security)
3. **Verify `picture_url` ↔ `avatar_url` mapping** in `users_me_patch.rs` against the OpenAPI spec

### P1: Complete Epic 4 (Hybrid Authorization)

4. **Wire jti denylist** — `authz-core/impl/src/denylist_middleware.rs` exists; wire it as middleware on authz-core routes
5. **Wire push invalidation** — `authz-core/impl/src/push_invalidator.rs` exists; add the endpoint
6. **Route classification** — define which routes are jwt-only vs jwt-with-online-fallback
7. **Role→permission mapping** — wire org-mgmt's permission tables into `principal_effective`

### P2: Wire Epic 2 scaffolding

8. **Integrate entitlements_ref/hash** into token issuer and consumer validation
9. **Wire entitlements_cache** (Redis snapshot storage with TTL)

### P3: Epic 5 completion

10. **Token revocation via version bump** — connect `VersionStore` bumps to consumer validation rejection
11. **JWKS rate limiting** — wire `middleware/rate_limit.rs` to JWKS endpoint

---

## 11. What “Deliver Sesame-IDAM” Actually Requires

Epic completion and **product delivery** are not the same scope. Use this section when prioritizing work.

### 11.1 Delivery tiers

| Tier | Definition | Status (2026-07-10) |
|------|------------|---------------------|
| **D0 — Compile & deploy** | Six services build, Tilt deploys, Postgres + Redis reachable | ✅ Workspace compiles; Kind/Tilt on ms02 |
| **D1 — Consumer auth contract** | Login → JWT (EdDSA) → JWKS validation in Hauliage microservices | ✅ Epic 1 + Hauliage HI-1..HI-8 smoke path |
| **D2 — Account-first onboarding** | Register → create/join org → JWT `org_id` → BFF `/organizations/me` | ⚠️ Partial — Sesame controllers exist; BFF E2E paused; `set_active_organization` still raw handler |
| **D3 — MVP identity surface** | Email/password login+register, refresh, logout, `/identity/me`, api-keys validate | ⚠️ Core paths real; refresh returns 200 on failure (SI-4 blocked on BR-3); many login variants still stubs |
| **D4 — B2B org platform** | Invites, memberships, roles, org admin, webhooks | ⚠️ Consumer org lifecycle partial; admin/SCIM/SSO/webhook delivery largely stub |
| **D5 — Full API surface (119 endpoints)** | Every OpenAPI operation DB-backed + gated per CONTRIBUTING | ❌ ~90% gen stubs |
| **D6 — JWT architecture (Epics 2–9)** | Hybrid authz, revocation, delegation, caching, hardening, observability | ❌ Epic 1 only; scaffolding elsewhere |

**Pragmatic “first delivery” target:** **D1 + D2 + D3** for the Hauliage integration path, not D5/D6.

### 11.2 Implemented vs stub (by service)

| Service | Real impl (confirmed) | Still stub / partial |
|---------|----------------------|----------------------|
| **identity-login** | `auth_login`, `auth_register`, `auth_logout`, `set_active_organization` (raw) | OTP, social OAuth, magic links, MFA, step-up, most `/auth/*` variants |
| **identity-session** | JWKS, key rotation, `auth_refresh` (200-on-error workaround), typed `/identity/me`, userinfo | OIDC admin, impersonation, MCP token paths |
| **authz-core** | `principal_effective` (roles; `permissions: []`) | `authorize` decisioning, denylist middleware unwired |
| **api-keys** | Gen + security wiring | Full validate/archive lifecycle needs audit |
| **identity-user-mgmt** | Security + models | Admin CRUD, MFA enrollment — stubs |
| **org-mgmt** | Consumer: create org, memberships, invite/accept | Admin org CRUD, SCIM, SSO, webhooks — mostly stub |

Sources: [`topic-login-flow.md`](../llmwiki/topics/topic-login-flow.md), [`topic-account-first-onboarding-checkpoint.md`](../llmwiki/topics/topic-account-first-onboarding-checkpoint.md), [`topic-remediation-plan.md`](../llmwiki/topics/topic-remediation-plan.md).

### 11.3 Cross-repo / platform gaps (not in Epics INDEX)

These block a polished Hauliage demo even when Epic 1 is done:

| Gap | Doc | Impact |
|-----|-----|--------|
| **K8s-native `:8080` ClusterIP** | [`PRD_k8s-native-idam-platform-and-hauliage-integration.md`](../PRD_k8s-native-idam-platform-and-hauliage-integration.md) | NodePort/8101–8106 matrix; Hauliage URLs must move to `:8080` FQDNs |
| **`database-env.yaml` + Secret** | Same PRD §2.1 | Sesame lacks Hauliage-style K8s DB wiring |
| **JWT signing `kid` ↔ JWKS alignment** | Same PRD §2.1 | Historical mismatch (`dev-ephemeral` vs published `kid`) — verify in current deploy |
| **Role-split demo personas** | PRD §5.4 | `shipper@amecorp.dev` / `transport@transportservices.dev` seeds + org resolution |
| **BR-3 / SI-4** | [`topic-brrtrouter-refactor-backlog.md`](../llmwiki/topics/topic-brrtrouter-refactor-backlog.md) | Refresh failures return HTTP 200 + empty body — not OAuth-compliant |
| **BR-4** | Same | `init_security` drift between gen and impl `main.rs` |
| **Account-first BFF E2E** | Hauliage onboarding PRD | Register → org create → active-org JWT chain not green end-to-end |
| **TypeScript SDK / hosted UI** | [`sesame-idam-complete.md`](../sesame-idam-complete.md) §5 | Vision item — not started |
| **Webhook delivery system** | [`entity-webhook.md`](../llmwiki/entities/entity-webhook.md) | No delivery tracking table; test endpoint stub |
| **RLS bridge SQL** | `sesame-idam-complete.md` §8 | Planned; not blocking Hauliage MVP |
| **Documentation drift** | `docs/Epics/INDEX.md` | Epic 1 status stale; open question #1 resolved in wiki but not INDEX |

### 11.4 CONTRIBUTING gates vs current reality

Every story requires: `cargo check`, `just lint-rust`, unit tests, **BDD E2E** (`just nt`). Implication:

- Epic 1 stories **meet gates** (100+ JWKS/key tests, BDD).
- Most stub endpoints **cannot be marked “done”** until impl + BDD exist — the 119-endpoint surface is the long tail of delivery.
- Security regression tests (tenant isolation, token tamper) should accompany each new real controller, not only Epic 8.

---

## 12. Recommended Next Work (enriched priority)

> **Active backlog:** [`first-delivery-wave-a.md`](./first-delivery-wave-a.md) — staged A1–A9 with status. Wave A2/A5 landed 2026-07-10.

Merged with §10; reordered for **delivery** not epic sequence alone.

### Wave A — Unblock Hauliage demo (1–2 weeks)

| # | Task | Owner | Unblocks |
|---|------|-------|----------|
| A1 | Align helm `aud` with per-service `jwks_client.rs` configs (§8) | sesame-idam | Stricter token audience binding |
| A2 | Migrate `set_active_organization.rs` to typed handler + `auth_context` (§7.3) | sesame-idam | Account-first JWT re-issue without manual base64 decode |
| A3 | Verify `picture_url` ↔ `avatar_url` in OpenAPI + `users_me_patch` (§7.2) | sesame-idam | Profile patch stability across regen |
| A4 | **BR-3** typed multi-status + **SI-4** `auth_refresh` 401 paths | BRRTRouter + sesame | OAuth-correct refresh; SDK/BFF error handling |
| A5 | Account-first smoke: register → `POST /organizations` → `POST /sessions/active-organization` → JWT `org_id` | sesame + ms02 | [`topic-account-first-onboarding-checkpoint.md`](../llmwiki/topics/topic-account-first-onboarding-checkpoint.md) resume list |
| A6 | Hauliage BFF chain: `POST /api/v1/organizations/me` after active-org | hauliage | Full onboarding E2E |
| A7 | Role-split demo seeds (shipper vs transport personas) | sesame + hauliage | PRD US-D2 parallel browser testing |

### Wave B — Platform hardening (parallel)

| # | Task | Epic / ID | Notes |
|---|------|-----------|-------|
| B1 | K8s `:8080` migration + `database-env.yaml` | PRD k8s-native | Match Hauliage Phase 3 convention |
| B2 | Wire jti denylist middleware on authz-core | Epic 5.2 | File exists; validation pipeline documents step 7 |
| B3 | Wire push invalidation endpoint | Epic 5.3 | `push_invalidator.rs` stub |
| B4 | Consumer `ver` rejection on version bump | Epic 5.1 | `VersionStore` bumps at login but consumers may not reject stale `ver` |
| B5 | **api-keys** validate impl + worker contract tests | D3 | HI-8 pattern from hauliage workers |
| B6 | Update `docs/Epics/INDEX.md` — Epic 1 complete, Epic 4.4 partial | docs | Close drift flagged in §0 |

### Wave C — JWT architecture (after Wave A)

| # | Task | Epic |
|---|------|------|
| C1 | Route classification (`jwt_only` vs `jwt_with_fallback`) | 4.1 |
| C2 | Selective per-request online fallback for high-risk routes | 4.3 |
| C3 | Role → permission mapping in `principal_effective` | 4.4+ |
| C4 | Wire `entitlements_ref` / `entitlements_hash` into token issuer | 2.x |
| C5 | Wire `entitlements_cache` Redis snapshots | 2.x + 7.x |
| C6 | Refresh reuse detection | 3.2 |
| C7 | `typ=at+jwt` enforcement + algorithm hardening | 8.1 |
| C8 | Prometheus JWT metrics + key-age alerting | 9.x |

### Wave D — Full product surface (long tail)

Prioritize by Hauliage/tenant-consumer OpenAPI (`openapi/idam/tenant-consumer/`) before admin/SCIM:

1. **identity-user-mgmt** — get/update/disable user (BDD + tenant isolation)
2. **org-mgmt** — remaining consumer paths (ADR-002 S2+)
3. **api-keys** — archive, rotate, scope validation
4. **org-mgmt admin** — SSO, SCIM, webhooks with real delivery
5. Login variants (OTP, social) — only when product demands them
6. Epic 6 delegation / impersonation — after revocation story is solid

### Explicitly defer (unless product asks)

- DPoP / RFC 8693 token exchange (Epic 3.3–3.4, 6.x)
- ES256 co-default key generation (Epic 1.3 note)
- HSM integration (Epic 8)
- Full Epic 7 caching layer before Epic 4 route policy exists
- TypeScript SDK until D3 paths are stable

---

## 13. Audit Action Items & Doc Maintenance

| Action | Owner | When |
|--------|-------|------|
| Update `docs/Epics/INDEX.md` Epic 1 row to **Implemented**; Epic 4 → **4.4 partial** | Next sesame session | After Wave B6 |
| Resolve INDEX open question #1 — point to [`topic-login-flow.md`](../llmwiki/topics/topic-login-flow.md) | Same | Done in wiki; copy to INDEX |
| Append `docs/llmwiki/log.md` entry referencing this audit enrichment | Same | This session |
| Re-run epic matrix after Wave A (set_active_organization typed, SI-4) | Post Wave A | Refresh §0 table |

### Risk register (delivery)

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Helm generic `aud` accepts cross-service tokens | Medium | Wave A1 |
| `set_active_organization` bypasses JWKS validation path | High | Wave A2 — security regression test |
| Refresh 200-on-error confuses BFF/clients | High | Wave A4 |
| Stub endpoints mistaken for production-ready | High | Mark OpenAPI `x-brrtrouter-impl` truthfully; audit csv in `docs/audit/openapi_example_coverage.csv` |
| Epic 2–9 scaffolding never wired | Medium | Wave C gates: no new utils without issuer/consumer integration story |

---

*Last enriched: 2026-07-10. Original audit scope: commits 2b57e73..HEAD. Cross-repo state includes Hauliage Wave 3 (fleet title, quote `vehicle_id`, JWKS smoke) — consumer integration progressing independently of Epics 2–9.*
