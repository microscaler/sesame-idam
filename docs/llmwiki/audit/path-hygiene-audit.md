# Path Hygiene Audit — Sesame-IDAM OpenAPI Paths

> **Date:** 2026-05-16
> **Status:** analysis-complete
> **Scope:** All 6 services, 119 endpoints, all path definitions
> **Goal:** Evaluate path naming, consistency, and propose a unified `/idam` subpath strategy
> **Execution:** Each service has a checkbox checklist below. Check off each item as you modify its OpenAPI spec.

---

## Executive Summary

The 6 Sesame-IDAM microservices have 119 endpoints across inconsistent path styles:

1. **Root-level paths** (`/`, `/{org_id}`, `/{key_id}`) — no prefix, relies on service routing
2. **`/login/*`** — login-service auth flows
3. **`/users/{id}/*`** — user-mgmt service, bare entity names at root
4. **`/api/v1/identity/users/me`** — session service, nested identity namespace
5. **`/api/v1/am/*`** — org-mgmt service, "access management" namespace (v1)
6. **`/.well-known/*`** — OIDC discovery, bare subdomain paths
7. **`/principals/*`** — authz-core, bare entity names at root
8. **`/admin/*`** — impersonation, bare noun at root
9. **`/mcp/*`** — MCP token flows, bare noun at root

The core problem: there is **no unifying API version or product prefix**. Each service independently chose its own naming convention (or none at all). This makes gateway routing fragile and future multi-product APIs impossible.

**Recommendation:** Prefix all paths with `/idam` (product) + version segment, then service-logical grouping. Example: `/idam/v1/auth/login`, `/idam/v1/identity/users/me`, `/idam/v1/authz/principal/effective`.

---

## Execution Order

Do services in this order. Each service is independent (no cross-spec dependencies in paths):

1. **api-keys** — 10 paths, simplest, least risk. Good warm-up.
2. **authz-core** — 11 paths, self-contained. Only dependency is login calling `/principal/effective` at login time (that's a service call, not an OpenAPI path concern).
3. **identity-login-service** — 20 paths, the largest batch under `/auth/` namespace.
4. **identity-user-mgmt-service** — 28 paths, all under `/admin/` namespace.
5. **identity-session-service** — 13 paths, most mixed styles, highest cognitive load.
6. **org-mgmt** — 33 paths, largest spec, has two existing path styles to unify.

---

## Pre-flight Checklist

- [ ] Read full audit (this document) to understand the proposed structure
- [ ] Back up current state: `git stash` or commit current specs
- [ ] Verify `cargo check --workspace` passes before starting
- [ ] Note: Do NOT edit `gen/` code — only edit `openapi/idam/{service}/openapi.yaml`
- [ ] After each service, run `just gen-{service}` to verify codegen succeeds
- [ ] After all 6 services, run `cargo check --workspace` to verify nothing broke

---

## 1. api-keys (Port 8103) — 10 paths

**File to edit:** `openapi/idam/api-keys/openapi.yaml`

**Scope:** Add `/idam/v1/api-keys/` prefix to all paths. Replace bare root `/` with `/idam/v1/api-keys`.

| # | Current Path | New Path | Checkbox |
|---|---|---|---|
| 1 | `/` (POST) | `/idam/v1/api-keys` | [ ] |
| 2 | `/{key_id}` (DELETE) | `/idam/v1/api-keys/{key_id}` | [ ] |
| 3 | `/current` (GET) | `/idam/v1/api-keys/current` | [ ] |
| 4 | `/archived` (GET) | `/idam/v1/api-keys/archived` | [ ] |
| 5 | `/archived/{key_id}` (GET) | `/idam/v1/api-keys/archived/{key_id}` | [ ] |
| 6 | `/usage` (GET) | `/idam/v1/api-keys/usage` | [ ] |
| 7 | `/import` (POST) | `/idam/v1/api-keys/import` | [ ] |
| 8 | `/validate` (POST) | `/idam/v1/api-keys/validate` | [ ] |
| 9 | `/validate/personal` (POST) | `/idam/v1/api-keys/validate/personal` | [ ] |
| 10 | `/validate/org` (POST) | `/idam/v1/api-keys/validate/org` | [ ] |

**After updating paths:**
- [ ] Run `just gen-api-keys` to verify codegen succeeds
- [ ] Run `cargo check --workspace` to verify no downstream breakage
- [ ] Update server base path if needed (change `/api/v1/am/api-keys/*` to `/idam/v1/api-keys/*` in `servers.paths`)

---

## 2. authz-core (Port 8102) — 11 paths

**File to edit:** `openapi/idam/authz-core/openapi.yaml`

**Scope:** Add `/idam/v1/authz/` prefix to all paths. Group authorize/principals and audit under separate sub-namespaces.

| # | Current Path | New Path | Checkbox |
|---|---|---|---|
| 1 | `/authorize` (POST) | `/idam/v1/authz/authorize` | [ ] |
| 2 | `/principal/effective` (POST) | `/idam/v1/authz/principals/effective` | [ ] |
| 3 | `/principals/roles` (POST, DELETE) | `/idam/v1/authz/principals/roles` | [ ] |
| 4 | `/principals/attributes` (POST) | `/idam/v1/authz/principals/attributes` | [ ] |
| 5 | `/audit/events` (POST) | `/idam/v1/authz/audit/events` | [ ] |
| 6 | `/audit/events/stats` (POST) | `/idam/v1/authz/audit/events/stats` | [ ] |
| 7 | `/audit/events/{id}` (GET) | `/idam/v1/authz/audit/events/{id}` | [ ] |
| 8 | `/audit/export` (POST) | `/idam/v1/authz/audit/export` | [ ] |
| 9 | `/audit/export/{export_id}` (GET) | `/idam/v1/authz/audit/export/{export_id}` | [ ] |
| 10 | `/audit/retention` (GET, POST) | `/idam/v1/authz/audit/retention` | [ ] |
| 11 | `/audit/retention/{id}` (PATCH, DELETE) | `/idam/v1/authz/audit/retention/{id}` | [ ] |

**After updating paths:**
- [ ] Run `just gen-authz-core` to verify codegen succeeds
- [ ] Run `cargo check --workspace` to verify no downstream breakage
- [ ] Update server base path in `servers.paths`
- [ ] **Important:** identity-login-service calls `/principal/effective` at login time. Verify that call target is updated (this is runtime code, not OpenAPI — check `identity-login-service/impl/` for the authz-core client URL)

---

## 3. identity-login-service (Port 8101) — 20 paths

**File to edit:** `openapi/idam/identity-login-service/openapi.yaml`

**Scope:** All paths under `/idam/v1/auth/`. Move OAuth under `/idam/v1/oauth/` (standard). Move SSO logout to `/idam/v1/oauth/logout`.

| # | Current Path | New Path | Checkbox |
|---|---|---|---|
| 1 | `/login` (POST) | `/idam/v1/auth/login` | [ ] |
| 2 | `/login/dual-otp` (POST) | `/idam/v1/auth/login/dual-otp` | [ ] |
| 3 | `/login/email-otp` (POST) | `/idam/v1/auth/login/email-otp` | [ ] |
| 4 | `/login/phone-otp` (POST) | `/idam/v1/auth/login/phone-otp` | [ ] |
| 5 | `/login/magic-link` (POST) | `/idam/v1/auth/login/magic-link` | [ ] |
| 6 | `/login/magic-link/verify` (POST) | `/idam/v1/auth/login/magic-link/verify` | [ ] |
| 7 | `/login/phone-magic-link` (POST) | `/idam/v1/auth/login/phone-magic-link` | [ ] |
| 8 | `/login/phone-magic-link/verify` (POST) | `/idam/v1/auth/login/phone-magic-link/verify` | [ ] |
| 9 | `/register` (POST) | `/idam/v1/auth/register` | [ ] |
| 10 | `/logout` (POST) | `/idam/v1/auth/logout` | [ ] |
| 11 | `/forgot-password` (POST) | `/idam/v1/auth/password/forgot` | [ ] |
| 12 | `/reset-password` (POST) | `/idam/v1/auth/password/reset` | [ ] |
| 13 | `/token` (POST) | `/idam/v1/auth/token` | [ ] |
| 14 | `/oauth/authorize` (GET) | `/idam/v1/oauth/authorize` | [ ] |
| 15 | `/signup/validate` (GET) | `/idam/v1/auth/signup/validate` | [ ] |
| 16 | `/verify/email-otp` (POST) | `/idam/v1/auth/verify/email-otp` | [ ] |
| 17 | `/verify/phone-otp` (POST) | `/idam/v1/auth/verify/phone-otp` | [ ] |
| 18 | `/verify/dual-otp` (POST) | `/idam/v1/auth/verify/dual-otp` | [ ] |
| 19 | `/social/{provider}/login` (GET) | `/idam/v1/auth/social/{provider}/login` | [ ] |
| 20 | `/social/{provider}/callback` (POST) | `/idam/v1/auth/social/{provider}/callback` | [ ] |

**After updating paths:**
- [ ] Run `just gen-identity-login` to verify codegen succeeds
- [ ] Run `cargo check --workspace` to verify no downstream breakage
- [ ] Update server base path in `servers.paths`
- [ ] Check for internal calls to authz-core `/principal/effective` — update client URL (runtime code, not OpenAPI)

---

## 4. identity-user-mgmt-service (Port 8106) — 28 paths

**File to edit:** `openapi/idam/identity-user-mgmt-service/openapi.yaml`

**Scope:** All user paths under `/idam/v1/admin/users/`. All audit paths under `/idam/v1/admin/audit/`. OAuth logout under `/idam/v1/oauth/`.

| # | Current Path | New Path | Checkbox |
|---|---|---|---|
| 1 | `/users` (POST) | `/idam/v1/admin/users` | [ ] |
| 2 | `/users/{user_id}` (DELETE) | `/idam/v1/admin/users/{user_id}` | [ ] |
| 3 | `/users/email` (GET) | `/idam/v1/admin/users/email` | [ ] |
| 4 | `/users/username` (GET) | `/idam/v1/admin/users/username` | [ ] |
| 5 | `/users/query` (GET) | `/idam/v1/admin/users/query` | [ ] |
| 6 | `/users/migrate` (POST) | `/idam/v1/admin/users/migrate` | [ ] |
| 7 | `/users/migrate-password` (POST) | `/idam/v1/admin/users/migrate-password` | [ ] |
| 8 | `/users/{user_id}/disable` (POST) | `/idam/v1/admin/users/{user_id}/disable` | [ ] |
| 9 | `/users/{user_id}/enable` (POST) | `/idam/v1/admin/users/{user_id}/enable` | [ ] |
| 10 | `/users/{user_id}/email` (PUT) | `/idam/v1/admin/users/{user_id}/email` | [ ] |
| 11 | `/users/{user_id}/email/verify` (POST) | `/idam/v1/admin/users/{user_id}/email/verify` | [ ] |
| 12 | `/users/{user_id}/phone` (POST) | `/idam/v1/admin/users/{user_id}/phone` | [ ] |
| 13 | `/users/{user_id}/phone/verify` (POST) | `/idam/v1/admin/users/{user_id}/phone/verify` | [ ] |
| 14 | `/users/{user_id}/mfa/disable` (POST) | `/idam/v1/admin/users/{user_id}/mfa/disable` | [ ] |
| 15 | `/users/{user_id}/mfa/setup` (POST) | `/idam/v1/admin/users/{user_id}/mfa/setup` | [ ] |
| 16 | `/users/{user_id}/mfa/verify` (POST) | `/idam/v1/admin/users/{user_id}/mfa/verify` | [ ] |
| 17 | `/users/{user_id}/password` (DELETE) | `/idam/v1/admin/users/{user_id}/password` | [ ] |
| 18 | `/users/{user_id}/social/link` (POST) | `/idam/v1/admin/users/{user_id}/social/link` | [ ] |
| 19 | `/users/{user_id}/social/tokens` (GET) | `/idam/v1/admin/users/{user_id}/social/tokens` | [ ] |
| 20 | `/users/{user_id}/social/tokens/{provider}/refresh` (GET) | `/idam/v1/admin/users/{user_id}/social/tokens/{provider}/refresh` | [ ] |
| 21 | `/users/{user_id}/resend-email-confirmation` (POST) | `/idam/v1/admin/users/{user_id}/resend-email-confirmation` | [ ] |
| 22 | `/users/{user_id}/magiclink` (POST) | `/idam/v1/admin/users/{user_id}/magiclink` | [ ] |
| 23 | `/users/{user_id}/employee` (GET) | `/idam/v1/admin/users/{user_id}/employee` | [ ] |
| 24 | `/users/{user_id}/logout-all-sessions` (POST) | `/idam/v1/admin/users/{user_id}/logout-all-sessions` | [ ] |
| 25 | `/audit/user/events` (POST) | `/idam/v1/admin/audit/events` | [ ] |
| 26 | `/audit/user/{user_id}/events/count` (GET) | `/idam/v1/admin/audit/users/{user_id}/events/count` | [ ] |
| 27 | `/audit/user/{user_id}/events/compliance-export` (POST) | `/idam/v1/admin/audit/users/{user_id}/events/compliance-export` | [ ] |
| 28 | `/oauth/logout` (POST) | `/idam/v1/oauth/logout` | [ ] |

**After updating paths:**
- [ ] Run codegen (`just gen-identity-user-mgmt` or full `just gen`) to verify codegen succeeds
- [ ] Run `cargo check --workspace` to verify no downstream breakage
- [ ] Update server base path in `servers.paths`

---

## 5. identity-session-service (Port 8105) — 13 paths

**File to edit:** `openapi/idam/identity-session-service/openapi.yaml`

**Scope:** Mixed styles need consolidation. OIDC paths get `/idam/v1/.well-known/`. Admin under `/idam/v1/admin/`. Identity/me under `/idam/v1/identity/me/`. MCP under `/idam/v1/mcp/`. Session under `/idam/v1/session/`.

| # | Current Path | New Path | Checkbox |
|---|---|---|---|
| 1 | `/.well-known/jwks.json` (GET) | `/idam/v1/.well-known/jwks.json` | [ ] |
| 2 | `/.well-known/openid-configuration` (GET) | `/idam/v1/.well-known/openid-configuration` | [ ] |
| 3 | `/admin/impersonate` (POST) | `/idam/v1/admin/impersonate` | [ ] |
| 4 | `/admin/impersonate/restore` (POST) | `/idam/v1/admin/impersonate/restore` | [ ] |
| 5 | `/api/v1/identity/users/me` (GET, PATCH) | `/idam/v1/identity/me` | [ ] |
| 6 | `/api/v1/identity/users/me/token` (POST) | `/idam/v1/identity/me/token` | [ ] |
| 7 | `/api/v1/identity/users/me/userinfo` (GET) | `/idam/v1/identity/userinfo` | [ ] |
| 8 | `/api/v1/platform/mcp/agents` (GET, POST) | `/idam/v1/mcp/agents` | [ ] |
| 9 | `/api/v1/platform/mcp/agents/{agent_id}` (GET, DELETE) | `/idam/v1/mcp/agents/{agent_id}` | [ ] |
| 10 | `/mcp/token` (POST) | `/idam/v1/mcp/token` | [ ] |
| 11 | `/mcp/token/validate` (POST) | `/idam/v1/mcp/token/validate` | [ ] |
| 12 | `/refresh` (POST) | `/idam/v1/session/refresh` | [ ] |
| 13 | `/verify/step-up` (POST) | `/idam/v1/auth/verify/step-up` | [ ] |

**After updating paths:**
- [ ] Run codegen to verify codegen succeeds
- [ ] Run `cargo check --workspace` to verify no downstream breakage
- [ ] Update server base path in `servers.paths`
- [ ] **Note:** Path 13 (`/verify/step-up`) moves to identity-login-service namespace (`/idam/v1/auth/verify/step-up`). This is a session-service endpoint that conceptually belongs to auth. Verify the handler logic still applies.

---

## 6. org-mgmt (Port 8104) — 33 paths

**File to edit:** `openapi/idam/org-mgmt/openapi.yaml`

**Scope:** 33 paths, two existing styles (`/{org_id}/*` and `/api/v1/am/*`). Unify to `/idam/v1/organizations/` for org paths, `/idam/v1/applications/` for app paths, `/idam/v1/sso/` for SAML paths.

| # | Current Path | New Path | Checkbox |
|---|---|---|---|
| 1 | `/` (GET) | `/idam/v1/organizations` | [ ] |
| 2 | `/{org_id}` (GET, PUT, DELETE) | `/idam/v1/organizations/{org_id}` | [ ] |
| 3 | `/{org_id}/users` (GET, POST, DELETE) | `/idam/v1/organizations/{org_id}/users` | [ ] |
| 4 | `/{org_id}/users/{user_id}/role` (PATCH) | `/idam/v1/organizations/{org_id}/users/{user_id}/role` | [ ] |
| 5 | `/{org_id}/domains` (PUT) | `/idam/v1/organizations/{org_id}/domains` | [ ] |
| 6 | `/{org_id}/invite-user` (POST) | `/idam/v1/organizations/{org_id}/invitations` | [ ] |
| 7 | `/{org_id}/invite-user-by-id` (POST) | `/idam/v1/organizations/{org_id}/invitations/by-id` | [ ] |
| 8 | `/{org_id}/migrate-to-isolated` (POST) | `/idam/v1/organizations/{org_id}/migrate-to-isolated` | [ ] |
| 9 | `/{org_id}/oidc-metadata` (POST) | `/idam/v1/organizations/{org_id}/oidc-metadata` | [ ] |
| 10 | `/{org_id}/pending-invites` (DELETE) | `/idam/v1/organizations/{org_id}/pending-invitations` | [ ] |
| 11 | `/{org_id}/allow-saml` (POST) | `/idam/v1/sso/saml/allow` | [ ] |
| 12 | `/{org_id}/enable-saml` (POST) | `/idam/v1/sso/saml/enable` | [ ] |
| 13 | `/{org_id}/create-saml-link` (POST) | `/idam/v1/sso/saml/link` | [ ] |
| 14 | `/{org_id}/disallow-saml` (POST) | `/idam/v1/sso/saml/disable` | [ ] |
| 15 | `/{org_id}/saml` (DELETE) | `/idam/v1/sso/saml` | [ ] |
| 16 | `/{org_id}/saml-metadata` (PUT) | `/idam/v1/sso/saml/metadata` | [ ] |
| 17 | `/{org_id}/role-mappings` (GET) | `/idam/v1/organizations/{org_id}/role-mappings` | [ ] |
| 18 | `/{org_id}/subscribe-role-mapping` (PUT) | `/idam/v1/organizations/{org_id}/role-mappings/subscribe` | [ ] |
| 19 | `/{org_id}/scim/users` (GET, POST) | `/idam/v1/organizations/{org_id}/scim/users` | [ ] |
| 20 | `/{org_id}/scim/users/{user_id}` (PUT, DELETE) | `/idam/v1/organizations/{org_id}/scim/users/{user_id}` | [ ] |
| 21 | `/{org_id}/scim/groups` (GET) | `/idam/v1/organizations/{org_id}/scim/groups` | [ ] |
| 22 | `/{org_id}/scim/groups/{group_id}` (GET) | `/idam/v1/organizations/{org_id}/scim/groups/{group_id}` | [ ] |
| 23 | `/{org_id}/webhooks` (GET) | `/idam/v1/organizations/{org_id}/webhooks` | [ ] |
| 24 | `/{org_id}/webhooks/{subscription_id}` (DELETE) | `/idam/v1/organizations/{org_id}/webhooks/{subscription_id}` | [ ] |
| 25 | `/{org_id}/webhooks/{subscription_id}/test` (POST) | `/idam/v1/organizations/{org_id}/webhooks/{subscription_id}/test` | [ ] |
| 26 | `/{org_id}/admin/users/{user_id}/invalidate-all-keys` (POST) | `/idam/v1/organizations/{org_id}/admin/users/{user_id}/invalidate-all-keys` | [ ] |
| 27 | `/api/v1/am/applications` (GET, POST) | `/idam/v1/applications` | [ ] |
| 28 | `/api/v1/am/applications/{app_id}` (GET) | `/idam/v1/applications/{app_id}` | [ ] |
| 29 | `/api/v1/am/applications/{app_id}/roles` (GET, POST) | `/idam/v1/applications/{app_id}/roles` | [ ] |
| 30 | `/api/v1/am/applications/{app_id}/roles/{role_id}` (GET) | `/idam/v1/applications/{app_id}/roles/{role_id}` | [ ] |
| 31 | `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` (GET, POST, DELETE) | `/idam/v1/applications/{app_id}/roles/{role_id}/permissions` | [ ] |
| 32 | `/{org_id}/users` (GET, POST) — duplicate org-users listing | `/idam/v1/organizations/{org_id}/users` (merged) | [ ] |
| 33 | `/{org_id}/users/{user_id}` (DELETE) | `/idam/v1/organizations/{org_id}/users/{user_id}` | [ ] |
| 34 | `/{org_id}/users/{user_id}` (PATCH) | `/idam/v1/organizations/{org_id}/users/{user_id}` | [ ] |

**Notes for org-mgmt:**
- Items 32, 33, 34 are duplicate `/{org_id}/users` entries (different HTTP methods) — they all merge to `/idam/v1/organizations/{org_id}/users` with GET/POST/DELETE/PATCH methods combined
- SAML paths (11-16) move from `/{org_id}/` to `/idam/v1/sso/saml/` — they are org-agnostic SSO configuration
- Application paths (27-31) drop the `/api/v1/am/` prefix and use clean `/idam/v1/applications/`

**After updating paths:**
- [ ] Run codegen to verify codegen succeeds
- [ ] Run `cargo check --workspace` to verify no downstream breakage
- [ ] Update server base path in `servers.paths`
- [ ] Check for internal client calls to old paths in other services

---

## Post-Migration Verification Checklist

### Codegen
- [ ] Run `just gen` (all 6 services) — verify 0 errors
- [ ] Run `cargo check --workspace` — verify 0 errors
- [ ] Run `cargo build --workspace` — verify clean build
- [ ] Run `cargo clippy -- -D warnings` — verify no clippy warnings

### OpenAPI Lint
- [ ] Run `just lint-openapi` — verify all 6 specs pass

### Runtime Verification
- [ ] Start Tilt: `just tilt-up`
- [ ] Verify all 6 pods are Running
- [ ] Test health endpoints on each service port
- [ ] Verify no 404s from handler routing (handlers should match new path patterns)

### Generated Code
- [ ] Check `gen/` directories — path params should reference new paths
- [ ] Check `impl/` route registration — handlers should be registered under new paths
- [ ] Verify handler signatures still compile (path param types unchanged, only paths changed)

### Cross-Service References
- [ ] **CRITICAL:** identity-login-service calls authz-core `/principal/effective` at login time. Verify the client URL in `identity-login-service/impl/` is updated to `/idam/v1/authz/principals/effective`.
- [ ] Check for any hardcoded path references in `impl/` code across all 6 services
- [ ] Check gateway/proxy config for path routing updates

### Tests
- [ ] Identify all test files referencing old paths
- [ ] Update BDD/integration test paths
- [ ] Run `cargo test --workspace` — verify all tests pass
- [ ] Verify no test references old paths in comments or assertions

### Documentation
- [ ] Update `docs/llmwiki/audit/path-hygiene-audit.md` checkbox status
- [ ] Update `docs/llmwiki/reference/ref-api-surface.md` — API surface reference
- [ ] Update `docs/service-topology-design.md` — service topology design (has old paths)
- [ ] Update `docs/design-doc.md` if it references specific paths
- [ ] Update `docs/sesame-idam-complete.md` if it references specific paths
- [ ] Update `docs/llmwiki/log.md` with session entry

---

## Summary Table: Execution Progress

| Service | Spec Edited | Codegen Passes | Build Passes | Tests Updated | Docs Updated |
|---|---|---|---|---|---|
| api-keys | [ ] | [ ] | [ ] | [ ] | [ ] |
| authz-core | [ ] | [ ] | [ ] | [ ] | [ ] |
| identity-login-service | [ ] | [ ] | [ ] | [ ] | [ ] |
| identity-user-mgmt-service | [ ] | [ ] | [ ] | [ ] | [ ] |
| identity-session-service | [ ] | [ ] | [ ] | [ ] | [ ] |
| org-mgmt | [ ] | [ ] | [ ] | [ ] | [ ] |
| **Post-migration** | — | [ ] | [ ] | [ ] | [ ] |

---

## Current State: Service-by-Service Analysis (Reference)

### 1. identity-login-service (Port 8101) — 20 endpoints

| # | Current Path | Methods | Problem |
|---|---|---|---|
| 1 | `/login` | POST | Bare entity at root — no version, no product prefix |
| 2 | `/login/dual-otp` | POST | Same problem |
| 3 | `/login/email-otp` | POST | Same problem |
| 4 | `/login/phone-otp` | POST | Same problem |
| 5 | `/login/magic-link` | POST | Same problem |
| 6 | `/login/magic-link/verify` | POST | Same problem |
| 7 | `/login/phone-magic-link` | POST | Same problem |
| 8 | `/login/phone-magic-link/verify` | POST | Same problem |
| 9 | `/register` | POST | Bare entity at root |
| 10 | `/logout` | POST | Bare entity at root |
| 11 | `/forgot-password` | POST | Bare entity at root |
| 12 | `/reset-password` | POST | Bare entity at root |
| 13 | `/token` | POST | Bare entity at root — conflicts with any `/token` on other services |
| 14 | `/oauth/authorize` | GET | Bare `/oauth` — could conflict with other OAuth providers |
| 15 | `/signup/validate` | GET | Bare `/signup` — no version |
| 16 | `/verify/email-otp` | POST | Bare `/verify` — could conflict |
| 17 | `/verify/phone-otp` | POST | Bare `/verify` — could conflict |
| 18 | `/verify/dual-otp` | POST | Bare `/verify` — could conflict |
| 19 | `/social/{provider}/login` | GET | Bare `/social` — could conflict |
| 20 | `/social/{provider}/callback` | POST | Bare `/social` — could conflict |

**Issues:**
- All paths are bare-root (no `/api/v1/` or `/auth/v1/` prefix)
- 7 paths share `/verify/*` — no namespace isolation
- `/token` is an OAuth2 generic name — conflicts with any service that might also have a token endpoint
- No API versioning (`v1` not present anywhere)
- No product prefix (`/idam` or `/auth` not present)

### 2. identity-session-service (Port 8105) — 13 endpoints

| # | Current Path | Methods | Problem |
|---|---|---|---|
| 1 | `/.well-known/jwks.json` | GET | OIDC standard path — acceptable as-is, but bare |
| 2 | `/.well-known/openid-configuration` | GET | Same — OIDC standard |
| 3 | `/admin/impersonate` | POST | Bare `/admin` — could conflict |
| 4 | `/admin/impersonate/restore` | POST | Same |
| 5 | `/api/v1/identity/users/me` | GET, PATCH | Has `/api/v1/` but "identity" is internal service naming leaking out |
| 6 | `/api/v1/identity/users/me/token` | POST | Same — leaking service name |
| 7 | `/api/v1/identity/users/me/userinfo` | GET | Same — OIDC userinfo should be standard path |
| 8 | `/api/v1/platform/mcp/agents` | GET, POST | `/api/v1/platform/` — "platform" is not a product concept |
| 9 | `/api/v1/platform/mcp/agents/{agent_id}` | GET, DELETE | Same |
| 10 | `/mcp/token` | POST | Bare `/mcp` — no version, no product prefix |
| 11 | `/mcp/token/validate` | POST | Same |
| 12 | `/refresh` | POST | Bare endpoint — most ambiguous name in entire API |
| 13 | `/verify/step-up` | POST | Bare `/verify` — conflicts with login-service /verify/* |

**Issues:**
- `/refresh` is the worst: no namespace, no version, no product prefix. A 4-letter word that means nothing without context
- Mix of `/api/v1/identity/` and `/api/v1/platform/` internal naming leaking externally
- `/admin/` at root — could conflict with any admin API on any service
- `/mcp/` at root — could conflict
- `/verify/step-up` conflicts with login-service `/verify/*` paths
- `/api/v1/identity/users/me/userinfo` should be `/userinfo` (standard OIDC)

### 3. identity-user-mgmt-service (Port 8106) — 28 endpoints

| # | Current Path | Methods | Problem |
|---|---|---|---|
| 1 | `/users` | POST | Bare `/users` — highest conflict risk |
| 2 | `/users/{user_id}` | DELETE | Same |
| 3 | `/users/email` | GET | Bare `/users/email` — could conflict |
| 4 | `/users/username` | GET | Same |
| 5 | `/users/query` | GET | Bare `/users/query` — ambiguous |
| 6 | `/users/migrate` | POST | Bare `/users/migrate` |
| 7 | `/users/migrate-password` | POST | Same |
| 8 | `/users/{user_id}/disable` | POST | Same |
| 9 | `/users/{user_id}/enable` | POST | Same |
| 10 | `/users/{user_id}/email` | PUT | Same |
| 11 | `/users/{user_id}/email/verify` | POST | Same |
| 12 | `/users/{user_id}/phone` | POST | Same |
| 13 | `/users/{user_id}/phone/verify` | POST | Same |
| 14 | `/users/{user_id}/mfa/disable` | POST | Same |
| 15 | `/users/{user_id}/mfa/setup` | POST | Same |
| 16 | `/users/{user_id}/mfa/verify` | POST | Same |
| 17 | `/users/{user_id}/password` | DELETE | Same |
| 18 | `/users/{user_id}/social/link` | POST | Same |
| 19 | `/users/{user_id}/social/tokens` | GET | Same |
| 20 | `/users/{user_id}/social/tokens/{provider}/refresh` | GET | Same |
| 21 | `/users/{user_id}/resend-email-confirmation` | POST | Same |
| 22 | `/users/{user_id}/magiclink` | POST | Same |
| 23 | `/users/{user_id}/employee` | GET | Same |
| 24 | `/users/{user_id}/logout-all-sessions` | POST | Same |
| 25 | `/audit/user/events` | POST | Bare `/audit` — could conflict |
| 26 | `/audit/user/{user_id}/events/count` | GET | Same |
| 27 | `/audit/user/{user_id}/events/compliance-export` | POST | Same |
| 28 | `/oauth/logout` | POST | Bare `/oauth` — conflicts with login-service |

**Issues:**
- 20+ paths under `/users/{user_id}/` — the largest flat subtree in the entire API
- No versioning, no product prefix
- `/audit/*` at root — could conflict with authz-core audit paths
- `/oauth/logout` — duplicates `/oauth/authorize` style, needs consistency
- No semantic grouping: account security, MFA, social, audit are all mixed under `/users/`
- This is an admin/management service — all paths should signal "admin" or "management"

### 4. authz-core (Port 8102) — 11 paths

| # | Current Path | Methods | Problem |
|---|---|---|---|
| 1 | `/authorize` | POST | Bare endpoint — most ambiguous name |
| 2 | `/principal/effective` | POST | Bare entity at root |
| 3 | `/principals/roles` | POST, DELETE | Same |
| 4 | `/principals/attributes` | POST | Same |
| 5 | `/audit/events` | POST | Bare `/audit` — conflicts with user-mgmt |
| 6 | `/audit/events/stats` | POST | Same |
| 7 | `/audit/events/{id}` | GET | Same |
| 8 | `/audit/export` | POST | Same |
| 9 | `/audit/export/{export_id}` | GET | Same |
| 10 | `/audit/retention` | GET, POST | Same |
| 11 | `/audit/retention/{id}` | PATCH, DELETE | Same |

**Issues:**
- `/authorize` is extremely bare — "authorize what?"
- No versioning, no product prefix
- `/audit/*` shared with user-mgmt — collision risk
- All paths are flat — no logical grouping of authz concepts
- 7 paths under `/audit/` — could be a lot of admin noise

### 5. api-keys (Port 8103) — 10 endpoints

| # | Current Path | Methods | Problem |
|---|---|---|---|
| 1 | `/` | POST | Bare root — `POST /` — creates confusion |
| 2 | `/{key_id}` | DELETE | Bare root with path param |
| 3 | `/current` | GET | Bare `/current` — ambiguous |
| 4 | `/archived` | GET | Bare `/archived` — ambiguous |
| 5 | `/archived/{key_id}` | GET | Bare |
| 6 | `/usage` | GET | Bare `/usage` — could conflict |
| 7 | `/import` | POST | Bare `/import` — could conflict |
| 8 | `/validate` | POST | Bare `/validate` — could conflict with login-service /verify |
| 9 | `/validate/personal` | POST | Same |
| 10 | `/validate/org` | POST | Same |

**Issues:**
- 4 bare root paths (`/`, `/{key_id}`, `/current`, `/archived`) — relies entirely on service routing
- `/validate` at root — conflicts with any other service that validates
- No versioning, no product prefix
- No semantic grouping — CRUD and validation are mixed together
- `POST /` is especially bad — REST convention uses it but it provides no discoverability

### 6. org-mgmt (Port 8104) — 33 paths

| # | Current Path | Methods | Problem |
|---|---|---|---|
| 1 | `/` | GET | Bare root |
| 2 | `/{org_id}` | GET, PUT, DELETE | Bare root with path param |
| 3 | `/{org_id}/users` | GET, POST, DELETE | Same |
| 4 | `/{org_id}/users/{user_id}/role` | PATCH | Same |
| 5 | `/{org_id}/domains` | PUT | Same |
| 6 | `/{org_id}/invite-user` | POST | Same |
| 7 | `/{org_id}/invite-user-by-id` | POST | Same |
| 8 | `/{org_id}/migrate-to-isolated` | POST | Same |
| 9 | `/{org_id}/oidc-metadata` | POST | Same |
| 10 | `/{org_id}/pending-invites` | DELETE | Same |
| 11 | `/{org_id}/allow-saml` | POST | Same |
| 12 | `/{org_id}/enable-saml` | POST | Same |
| 13 | `/{org_id}/create-saml-link` | POST | Same |
| 14 | `/{org_id}/disallow-saml` | POST | Same |
| 15 | `/{org_id}/saml` | DELETE | Same |
| 16 | `/{org_id}/saml-metadata` | PUT | Same |
| 17 | `/{org_id}/users` | GET, POST | Same (duplicate listing) |
| 18 | `/{org_id}/users/{user_id}` | DELETE | Same |
| 19 | `/{org_id}/role-mappings` | GET | Same |
| 20 | `/{org_id}/subscribe-role-mapping` | PUT | Same |
| 21 | `/{org_id}/scim/users` | GET, POST | Same |
| 22 | `/{org_id}/scim/users/{user_id}` | PUT, DELETE | Same |
| 23 | `/{org_id}/scim/groups` | GET | Same |
| 24 | `/{org_id}/scim/groups/{group_id}` | GET | Same |
| 25 | `/{org_id}/webhooks` | GET | Same |
| 26 | `/{org_id}/webhooks/{subscription_id}` | DELETE | Same |
| 27 | `/{org_id}/webhooks/{subscription_id}/test` | POST | Same |
| 28 | `/{org_id}/users/{user_id}` | PATCH | Same |
| 29 | `/api/v1/am/applications` | GET, POST | Has `/api/v1/` but "am" = "access management" internal naming |
| 30 | `/api/v1/am/applications/{app_id}` | GET | Same |
| 31 | `/api/v1/am/applications/{app_id}/roles` | GET, POST | Same |
| 32 | `/api/v1/am/applications/{app_id}/roles/{role_id}` | GET | Same |
| 33 | `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | GET, POST, DELETE | Same |
| 34 | `/{org_id}/admin/users/{user_id}/invalidate-all-keys` | POST | Same |

**Issues:**
- 28 paths with `/{org_id}/` at root — deepest nesting in the API
- Two different path styles in same spec: `/{org_id}/*` AND `/api/v1/am/*`
- `/api/v1/am/` — "am" is opaque, no consumer knows what "am" means
- No product prefix anywhere
- `/` at root — no discoverability
- SAML/SSO paths scattered — should be grouped under `/sso/` or `/saml/`
- SCIM paths under `/{org_id}/scim/` — consistent but not under versioned namespace

---

## Cross-Service Conflict Map

The following paths **collide** across services in the current spec (same last segment at the same depth):

| Path Segment | Services | Conflict? |
|---|---|---|
| `/verify/*` | login-service (3), session-service (1) | YES — 4 paths total |
| `/validate` (root) | api-keys, login-service (/verify) | PARTIAL — different methods, same intent |
| `/oauth/*` | login-service (1), user-mgmt (1) | YES — `/oauth/authorize` vs `/oauth/logout` |
| `/users/{id}/` | login-service, session-service, user-mgmt | YES — 3 services touch users |
| `/social/*` | login-service, user-mgmt | YES — OAuth social flows |
| `/admin/*` | session-service, org-mgmt | YES — impersonation vs key invalidation |
| `/audit/*` | authz-core (7), user-mgmt (3) | YES — 10 audit paths total |
| `/token` | login-service (POST /token), session-service (/mcp/token, /api/v1/identity/users/me/token) | YES — multiple token paths |
| `/mcp/*` | session-service only (2) | NO — unique to session-service |
| `/.well-known/*` | session-service only (2) | NO — OIDC standard |
| `/{org_id}/` | org-mgmt only (28) | NO — unique to org-mgmt |
| `/{key_id}/` | api-keys only (2) | NO — unique to api-keys |
| `/principals/*` | authz-core only (3) | NO — unique to authz-core |
| `/refresh` | session-service only (1) | NO — unique, but ambiguous name |

**Total collision groups:** 8 out of 11 conflict categories would be resolved by `/idam/v1/` prefix + proper namespacing.

---

## Summary Table: Current vs Proposed by Service

| Service | Current Paths | Proposed Paths | Path Style Change |
|---|---|---|---|
| identity-login-service | 20 bare-root paths (`/login/*`, `/register`, `/token`, etc.) | 20 under `/idam/v1/auth/*` | All gain `/idam/v1/auth/` prefix |
| identity-session-service | 13 mixed styles (`/.well-known/*`, `/api/v1/identity/*`, `/mcp/*`, `/refresh`, `/admin/*`) | 13 under `/idam/v1/session/*` + `/idam/v1/.well-known/*` + `/idam/v1/admin/*` | Unified to `/idam/v1/` + logical group |
| identity-user-mgmt-service | 28 bare-root paths (`/users/{id}/*`, `/audit/*`, `/oauth/*`) | 28 under `/idam/v1/admin/users/*` + `/idam/v1/admin/audit/*` | All gain `/idam/v1/admin/` prefix |
| authz-core | 11 bare-root paths (`/authorize`, `/principals/*`, `/audit/*`) | 11 under `/idam/v1/authz/*` | All gain `/idam/v1/authz/` prefix |
| api-keys | 10 bare-root paths (`/`, `/{key_id}`, `/validate`, `/current`) | 10 under `/idam/v1/api-keys/*` | All gain `/idam/v1/api-keys/` prefix |
| org-mgmt | 33 mixed styles (`/{org_id}/*`, `/api/v1/am/*`) | 33 under `/idam/v1/organizations/*` + `/idam/v1/applications/*` + `/idam/v1/sso/*` | Unified to `/idam/v1/` + logical group |

**Total:** 119 endpoints, 119 proposed paths. All gain `/idam/v1/` product+version prefix.

---

## Design Rationale

### Why `/idam` prefix?

1. **Product identification:** In a microscaler environment with multiple suites (hauliage, rerp, accounting, etc.), the gateway needs to route by product. `/idam/v1/` tells the router "this is an identity service request."

2. **Future-proofing:** If Sesame-IDAM is ever exposed as a standalone product (beyond microscaler), the `/idam/` prefix becomes the product name without requiring path changes.

3. **No collision with hauliage:** Hauliage uses `/api/v1/` prefix for its own paths. `/idam/v1/` is distinct and clearly belongs to the identity service, not hauliage.

### Why `/v1/` version?

1. **Breaking change isolation:** Future IDAM changes (new auth flows, new token types) can version without breaking consumers.

2. **Consistency:** Other microscaler suites use `/api/v1/`. IDAM uses `/idam/v1/` — same version concept, product-specific namespace.

3. **OIDC exception:** `/.well-known/` paths are OIDC standard and don't carry versions. They stay as `/.well-known/` under the `/idam/v1/` product prefix.

### Why service-specific sub-namespaces?

The `/idam/v1/` prefix is followed by a logical group that reflects the endpoint's purpose, not the service it lives on:

- `/idam/v1/auth/*` — all authentication (login, register, token, verify)
- `/idam/v1/session/*` — session management (refresh, userinfo, impersonation)
- `/idam/v1/admin/*` — admin operations (user CRUD, audit, account security)
- `/idam/v1/authz/*` — authorization (authorize, principals, audit)
- `/idam/v1/api-keys/*` — API key lifecycle
- `/idam/v1/organizations/*` — org management
- `/idam/v1/applications/*` — application management
- `/idam/v1/sso/*` — SSO/SAML/SCIM
- `/idam/v1/mcp/*` — MCP authentication
- `/idam/v1/oauth/*` — OAuth2 standard endpoints

### Paths that change and why

| Change | Reason |
|---|---|
| `/login` → `/idam/v1/auth/login` | Group auth flows together |
| `/refresh` → `/idam/v1/session/refresh` | Disambiguate — "session refresh" |
| `/token` → `/idam/v1/auth/token` | Disambiguate from MCP token |
| `/authorize` → `/idam/v1/authz/authorize` | "authorize" in authz context |
| `/users/{id}` → `/idam/v1/admin/users/{id}` | Admin service → admin namespace |
| `/api/v1/identity/users/me` → `/idam/v1/identity/me` | Drop internal "identity" leak, use standard `/me` |
| `/api/v1/am/applications` → `/idam/v1/applications` | Drop opaque "am" |
| `/{org_id}/*` → `/idam/v1/organizations/{id}/*` | Full path, no bare root |
| `/api/v1/platform/mcp/*` → `/idam/v1/mcp/*` | Drop "platform" |
| `/verify/*` on login + `/verify/step-up` on session → `/idam/v1/auth/verify/*` | Unified verify namespace |

---

## Migration Impact Assessment

### What changes (cosmetic only, no logic change):

1. **OpenAPI specs** — 119 path definitions change (all specs affected)
2. **Generated code** — `gen/` code regenerates from new specs (handler names may shift)
3. **Tilt/gateway routing** — port routing stays the same (each service still on its own port), but the gateway/proxy needs updated path-to-service mapping
4. **Consumer apps** — all HTTP clients need path prefix added (`/idam/v1/`)
5. **Tests** — all BDD/integration tests reference paths, all need update

### What stays the same:

1. **Port assignments** — each service stays on its current port
2. **Service boundaries** — no re-architecting needed
3. **Business logic** — handlers receive the same request shapes
4. **Tenancy model** — X-Tenant-ID header unchanged
5. **JWT claims** — no token format changes

### Risk assessment:

| Risk | Severity | Mitigation |
|---|---|---|
| Consumer app breakage | HIGH | Gateway proxy can provide dual-path support during transition |
| Generated code mismatch | LOW | `just gen` handles it, just verify handlers still wire correctly |
| OpenAPI lint failures | LOW | Path changes don't affect schema validation |
| Test suite breakage | MEDIUM | All test paths need update — mechanical change |
| Documentation drift | MEDIUM | All docs reference old paths — bulk update needed |

---

## Alternative Approaches Considered

### A: Keep bare paths, just use gateway routing
- **Pros:** Zero spec changes, zero migration cost
- **Cons:** No product identification, future collisions, no versioning, no discoverability
- **Verdict:** Reject — this is the current state and it's the problem we're solving

### B: Use `/api/v1/idam/*` (product inside version)
- **Pros:** Consistent with other microscaler suites
- **Cons:** `/api/v1/` implies "public API" — IDAM is infrastructure, not a public API
- **Verdict:** Considered but `/idam/v1/` is more semantically correct

### C: Use `/auth/*` instead of `/idam/v1/auth/*`
- **Pros:** Shorter paths
- **Cons:** No versioning, no product prefix, future collision if IDAM adds non-auth capabilities
- **Verdict:** Reject — no versioning is a debt trap

### D: Version-less `/idam/*`
- **Pros:** Simpler, no v1 suffix
- **Cons:** No breaking-change isolation, future incompatible changes break everything
- **Verdict:** Acceptable short-term, but `/idam/v1/` is safer for a platform service

---

## Recommended Path Structure

```
/idam/
  v1/
    auth/                  # Login, register, token, OTP, magic-link, social
    session/               # Refresh, userinfo, impersonation
    admin/                 # User CRUD, audit, account security
    authz/                 # Authorization, principals, roles, permissions
    api-keys/              # API key lifecycle, validation
    organizations/         # Org CRUD, users, domains, invitations
    applications/          # Application lifecycle, roles, permissions
    sso/                   # SAML, OIDC, SCIM
    mcp/                   # MCP token, agents
    oauth/                 # OAuth2 authorize, logout
    .well-known/           # JWKS, OIDC discovery (standard paths preserved)
```

This structure:
- Every path starts with `/idam/v1/`
- Second segment is a logical group (not a service name)
- Resources use plural nouns (`users`, `organizations`, `api-keys`)
- Actions use verbs or noun phrases (`login`, `refresh`, `authorize`)
- Parameters use `{snake_case_id}` format
- No bare root paths (`/`, `/{id}`)
- No internal naming leaks (`/am/`, `/platform/`, `/identity/`)
