---
title: BRRTRouter Refactor Backlog (Workaround Cleanup)
status: verified
updated: 2026-07-06
sources: [topic-http-client-policy.md, topic-brrtrouter-codegen.md, log.md]
---

# BRRTRouter Refactor Backlog (Workaround Cleanup)

Cross-repo backlog for removing Sesame-IDAM workarounds that exist because of BRRTRouter codegen or runtime gaps. **Not a broad BRRTRouter rewrite** — targeted fixes only.

## Canonical implementation spec

| Repo | Page |
|------|------|
| BRRTRouter (sibling) | [`BRRTRouter/docs/llmwiki/topics/sesame-idam-workarounds-cleanup.md`](../../../../BRRTRouter/docs/llmwiki/topics/sesame-idam-workarounds-cleanup.md) |
| Hauliage (consumer) | [`hauliage/docs/llmwiki/topics/sesame-idam-brrtrouter-integration.md`](../../../../hauliage/docs/llmwiki/topics/sesame-idam-brrtrouter-integration.md) |

## Workarounds in sesame-idam today

| Workaround | Where | Trigger |
|------------|-------|---------|
| No global `security` on login/session specs | `openapi/idam/identity-login-service/openapi.yaml`, `identity-session-service/openapi.yaml` | BRRTRouter treats `security: []` as “inherit global” → public routes got 401 in-cluster (H6.1) |
| Raw handlers for principal endpoints | `identity-session-service/impl/src/raw_handler.rs` | Typed dispatch drops `HandlerRequest::jwt_claims` |
| Per-route explicit `BearerAuth` | All protected ops in login/session specs | Compensates for removed global security |
| Full `init_security` in impl `main.rs` | `identity-login-service/impl/src/security.rs` | Gen `main.rs` registers providers; impl had JWKS-only init → deploy 401 |
| Refresh errors → HTTP 200 + empty body | `identity-session-service/impl/src/controllers/auth_refresh.rs` | Typed handler success-schema-only; no 401 path |
| `sesame_common::http` re-export | `microservices/common/src/http.rs` | Correct — not a workaround; canonical outbound path |

## Task backlog (cross-repo IDs)

### P1 — Correctness (unblocks proper OpenAPI security)

| ID | Owner | Task | Unblocks |
|----|-------|------|----------|
| **BR-1** | BRRTRouter | Fix `security: []` semantics in `build.rs` + oas3 presence tracking | Restore global `security` on mixed public/protected specs |
| **SI-1** | sesame-idam | After BR-1: re-add global `BearerAuth` on login/session specs; keep `security: []` on public ops only | Spec hygiene; fewer per-route copies |
| **SI-2** | sesame-idam | Regression test: public login/register/refresh/JWKS/OIDC return 200 without providers when global security restored | H6.1 guard |

### P2 — Ergonomics (optional for hauliage P1)

| ID | Owner | Task | Unblocks |
|----|-------|------|----------|
| **BR-2** | BRRTRouter | Pass validated `jwt_claims` into typed handler context | Delete `raw_handler.rs` pattern for `/identity/me`, userinfo |
| **BR-3** | BRRTRouter | Typed multi-status responses (or codegen `HttpJson` for error schemas) | OAuth-compliant 401 on refresh failure |
| **BR-4** | BRRTRouter | Codegen `init_security` helper from spec security schemes | Stop impl/gen provider registration drift |
| **SI-3** | sesame-idam | Migrate `/identity/me`, userinfo to typed handlers after BR-2 | Less boilerplate |
| **SI-4** | sesame-idam | Migrate `auth_refresh` error paths after BR-3 | Proper 401/400 on bad refresh |

### P3 — Platform hygiene (post-hauliage)

| ID | Owner | Task | Notes |
|----|-------|------|-------|
| **BR-5** | BRRTRouter | JWKS background refresh: `std::thread` → `may::go!` | Consistency with coroutine runtime |
| **BR-6** | BRRTRouter | Shed transitive `reqwest` (OTEL grpc-only, jsonschema no-network) | See [`topic-http-client-policy.md`](./topic-http-client-policy.md) |
| **BR-7** | BRRTRouter | Sub-spans inside `JwksBearerProvider` (`jwt.signature_verify`, etc.) | Epic 9 / Story 9.1 |
| **HI-1** | hauliage | Pin BRRTRouter after BR-1+BR-2; verify `JwksBearerProvider` against sesame JWKS (H7.2) | Sesame integration gate |
| **HI-2** | hauliage | Adopt `HttpJson` on identity-adjacent routes per existing PRD | [`PRD_HAULIAGE_TYPED_HANDLER_HTTP_STATUS.md`](../../../../hauliage/docs/PRD_HAULIAGE_TYPED_HANDLER_HTTP_STATUS.md) |

## Already done (do not re-litigate)

| Item | Status |
|------|--------|
| `brrtrouter::http` + security provider migration | ✅ BRRTRouter Phase 1 (2026-07-06) |
| Sesame outbound HTTP via `sesame_common::http` | ✅ |
| may_minihttp client epic | ❌ Deferred — server-only for Sesame |

## Recommended sequencing

```
Sprint next (P1)
  BR-1  →  SI-1 + SI-2  →  HI-1 (H7.2 JWKS smoke)

Then (P2, when OAuth status codes matter)
  BR-2/BR-3  →  SI-3/SI-4

Later (P3)
  BR-4..BR-7, HI-2
```

## Code anchors

- BRRTRouter security inheritance bug: `BRRTRouter/src/spec/build.rs` (~615–619)
- Sesame raw handler: `microservices/idam/identity-session-service/impl/src/raw_handler.rs`
- Global security removal fix: commit `26b4aba`

## Gaps / Drift

> **Open:** `docs/plan/hauliage-readiness-plan.md` still lists some BRRTRouter items under HTTP migration — update when BR-1 lands.
