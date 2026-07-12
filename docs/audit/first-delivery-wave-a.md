# First Delivery — Wave A Staging

> **Target:** D1 + D2 + D3 for the Hauliage integration path (see [`epic-delivery-audit-2026-07-10.md`](./epic-delivery-audit-2026-07-10.md) §11).
>
> **Staged:** 2026-07-10. Update the **Status** column as tasks land.

---

## Biggest missing pieces (staged)

| ID | Piece | Status | Owner | Blocks |
|----|-------|--------|-------|--------|
| **A1** | JWT `aud` policy — document intentional `sesame-idam` (issuer + all consumers aligned) | ✅ Documented | sesame | Single audience is intentional for MVP |
| **A2** | Typed `set_active_organization` + `auth_context` (no manual JWT decode) | ✅ Done | sesame | D2 account-first JWT re-issue |
| **A3** | `picture_url` ↔ `avatar_url` OpenAPI ↔ gen ↔ `users_me_patch` | ✅ Done | sesame | OpenAPI + regen; OIDC userinfo keeps `picture_url` |
| **A4** | **BR-3** multi-status typed handlers + **SI-4** `auth_refresh` 401 | ✅ Done | sesame | `auth_refresh` returns `HttpJson` 401 `invalid_grant` |
| **A5** | Account-first BDD: register → org membership → active-org → JWT `org_id` | ✅ Tests added (skip w/o DB) | sesame | D2 gate |
| **A6** | Hauliage BFF E2E: `POST /api/v1/organizations/me` after active-org | ✅ Done | hauliage | Playwright `real_account_first_onboarding.spec.ts` green (2026-07-12, 4.1s) |
| **A7** | Role-split demo seeds (`shipper@amecorp.dev`, `transport@transportservices.dev`) | ✅ Done | sesame + hauliage | Users/orgs/memberships + company `organization_profiles` seeds |
| **A8** | K8s `database-env.yaml` + `:8080` ClusterIP parity | 🟡 Helm ready | sesame | `k8s/microservices/database-env.yaml` + `_http-kubernetes.yaml`; verify when Kind up |
| **A9** | `api-keys` validate impl + contract test | ✅ Done | sesame | `validate_api_key.rs` + `tests/bdd/api_key_flow.rs` (skip w/o DB) |

---

## Execution order (this session → next)

```
Now     A2 typed set_active_organization + A5 BDD smoke
Next    A3 avatar field audit (quick)
Then    A4 BR-3/SI-4 (cross-repo — unblock refresh 401)
Parallel A7 demo seeds (org-mgmt seed already partial)
Later   A6 BFF E2E (hauliage), A8 platform PRD, A9 api-keys
```

---

## A1 — Audience policy (clarification)

**Finding:** `token_issuer.rs` sets `aud: ["sesame-idam"]` on all issued tokens. Helm + impl `config.yaml` use `aud: sesame-idam` for validation. The per-service audiences in `jwks_client.rs` (`identity-login.seasame-idam.microscaler.local`, etc.) are **reference presets**, not production config.

**Decision for first delivery:** Keep **single audience `sesame-idam`** — simpler for multi-service consumers (Hauliage fleet, company, BFF). Per-service `aud` is a **post-MVP hardening** item (Epic 8), not a Wave A blocker.

**Action:** Mark A1 ✅ after doc note in audit §8; optional follow-up to align `jwks_client.rs` comment block with reality.

---

## A2 — Typed active-organization (acceptance)

- [x] `auth_context.rs` in identity-login-service (mirror session SI-3)
- [x] `set_active_organization.rs` uses `#[handler]` + `TypedHandlerRequest` + `jwt_claims`
- [x] `main.rs` registers via `spawn_typed_with_stack_size_and_name` (not `spawn_untyped`)
- [x] No `base64` manual JWT payload decode in controller
- [x] 403 when user not in org; 401 when claims/header mismatch
- [x] `cargo check` + login-service BDD pass on ms02 (DB tests skip when Kind down)

---

## A5 — Account-first BDD (acceptance)

- [x] Test: register user → seed org + membership → login → `set_active_organization` → access token contains `org_id`
- [x] Test: non-member org → 403
- [x] Skips when Postgres/Redis unreachable (same pattern as `token_lifecycle.rs`)

---

## Cross-repo handoffs

| Handoff | From | To | When |
|---------|------|-----|------|
| Active-org JWT with `org_id` | sesame A2+A5 green | hauliage A6 BFF chain | After ms02 redeploy login |
| BR-3 merged | BRRTRouter | sesame SI-4 `auth_refresh` | Before SDK documents refresh errors |
| Demo org UUIDs | org-mgmt seeds `20260706000002` | hauliage `organization_profiles` | A7 |

---

## Verification commands (ms02)

```bash
source ~/.cargo/env
cd ~/Workspace/microscaler/seasame-idam/microservices

# Wave A gates
cargo check --workspace
cargo test -p sesame_idam_identity_login_service --test main_bdd account_first -- --nocapture
cargo test -p sesame_idam_identity_login_service --test main_bdd token_lifecycle -- --nocapture

# Redeploy (Tilt port 10351)
TILT_PORT=10351 tilt trigger build-identity-login-service
TILT_PORT=10351 tilt trigger docker-identity-login-service
TILT_PORT=10351 tilt trigger identity-login-service
# Same pattern for identity-session-service

# Hauliage E2E (A6 — needs live BFF + stack)
# PLAYWRIGHT_BASE_URL=http://hauliage.dev.microscaler.local REAL_LOGIN=1 \
#   yarn test:e2e e2e/specs/auth/real_account_first_onboarding.spec.ts --project=chromium

# Manual smoke (account-first)
# register → POST /idam/v1/organizations (8104) → POST /idam/v1/sessions/active-organization (8101)
```

**2026-07-12:** Tilt redeploy triggered for login + session. BDD gates green on ms02 with port-forward (`KUBECONFIG=../shared-k8s-cluster/kubeconfig/shared-k8s.yaml`; postgres/redis in namespace `data`). Default `kubectl` context unset — use shared-k8s kubeconfig for cluster ops.

---

*Update this file when each ID moves status. Append `docs/llmwiki/log.md` on wave completion.*
