# Account-first onboarding — Sesame implementation checkpoint (2026-07-08)

- **Status**: `partially-verified` — A2/A5 live BDD green; A6 Playwright E2E green (2026-07-12)
- **Source docs**: [`../ADR-002-tenant-consumer-idam-api-boundary.md`](../../ADR-002-tenant-consumer-idam-api-boundary.md), [`openapi/idam/tenant-consumer/openapi.yaml`](../../openapi/idam/tenant-consumer/openapi.yaml), Hauliage [`PRD_account-first-onboarding.md`](../../../hauliage/docs/PRD_account-first-onboarding.md)
- **Last updated**: 2026-07-12
- **Paused for**: Wave A commit + A8 in-cluster verify (optional)

## What it is

Sesame owns **users, orgs, memberships, invites**, and JWT **`org_id`**. Tenant consumers (Hauliage BFF) call org-mgmt on **8104** and identity-login on **8101** with `X-Tenant-ID: hauliage`.

## Resume here (next session)

1. **Port-forward (ms02):** `export KUBECONFIG=../shared-k8s-cluster/kubeconfig/shared-k8s.yaml` then forward `data/postgres:5432` and `data/redis:6379`
2. ~~Run account_first BDD~~ ✅ 2/2 pass (2026-07-12)
3. ~~Hauliage E2E (A6)~~ ✅ Playwright green 2026-07-12
4. Commit sesame-idam Wave A changes when ready

See [`docs/audit/first-delivery-wave-a.md`](../../audit/first-delivery-wave-a.md) for full staged backlog.

## Implemented

### JWT `org_id`

- `microservices/idam/common/src/jwt/types.rs`, `builders.rs` — optional top-level `org_id` on access claims
- `identity-login-service/impl/src/services/token_issuer.rs` — `issue_tokens(…, org_id: Option<&str>)`
- `identity-login-service/impl/src/services/org_context.rs` — `resolve_active_org_id` on login / set-active-org

### identity-login-service

| Controller | Path | Notes |
|------------|------|--------|
| `set_active_organization.rs` | `POST /sessions/active-organization` | Re-issue JWT after org create/accept (**typed handler + `auth_context`**, Wave A2) |
| `auth_login.rs` | `POST /auth/login` | Resolves active org when membership exists |
| `auth_register.rs` | `POST /auth/register` | Identity only |
| `auth_logout.rs` | `POST /auth/logout` | Refresh revoke |

OpenAPI: `openapi/idam/identity-login-service/openapi.yaml` — `/sessions/active-organization` added.

Wired in `impl/src/main.rs` via Register & Overwrite (typed dispatch for `set_active_organization`).

### org-mgmt (consumer API)

| Controller | Operation |
|------------|-----------|
| `create_organization.rs` | `POST /organizations` |
| `list_my_memberships.rs` | `GET /users/me/memberships` |
| `accept_invitation.rs` | `POST /invitations/accept` |
| `invite_user_to_org.rs` | Invite by email |
| `add_user_to_org.rs` | Add existing user |

Service: `org_lifecycle.rs` — create, invite, accept, list memberships.

OpenAPI: consumer paths in `openapi/idam/org-mgmt/openapi.yaml`; draft tenant-consumer spec at `openapi/idam/tenant-consumer/openapi.yaml`.

`org_invites.accepted_at` — nullable in migration + model.

### Ports (Tilt / ms02)

| Service | Port |
|---------|------|
| identity-login-service | 8101 |
| org-mgmt | 8104 |
| authz-core | 8102 |

Hauliage client default: login URL `:8101` → org-mgmt `:8104` when `SESAME_ORG_MGMT_URL` unset.

## Not done

- Full OpenAPI regen for all org-mgmt admin stubs (only consumer handlers wired in `main.rs`)
- Frontend / BFF onboarding UX (Hauliage repo) — Playwright spec exists; needs live stack (`REAL_LOGIN=1`)
- Live DB verification of account-first BDD ✅ (ms02 2026-07-12)
- ADR-002 S2+ consumer paths beyond org lifecycle

## Protected impl files

Line 1 on all controllers above: `// BRRTRouter: user-owned`. Required before any `generate-stubs --force`.

## Cross-references

- Hauliage checkpoint: [`../../../hauliage/docs/llmwiki/topics/account-first-onboarding-checkpoint.md`](../../../hauliage/docs/llmwiki/topics/account-first-onboarding-checkpoint.md)
- Tenancy: [`topic-tenancy-model.md`](./topic-tenancy-model.md)
- Login flow: [`topic-login-flow.md`](./topic-login-flow.md)
- Codegen: [`topic-brrtrouter-codegen.md`](./topic-brrtrouter-codegen.md)
- BRRTRouter lifecycle PRD: [`../../../BRRTRouter/docs/PRD_IMPL_CONTROLLER_LIFECYCLE.md`](../../../BRRTRouter/docs/PRD_IMPL_CONTROLLER_LIFECYCLE.md)
