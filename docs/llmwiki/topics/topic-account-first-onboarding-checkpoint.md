# Account-first onboarding — Sesame implementation checkpoint (2026-07-08)

- **Status**: `partially-verified` — compiles on ms02; consumer E2E with Hauliage BFF pending
- **Source docs**: [`../ADR-002-tenant-consumer-idam-api-boundary.md`](../../ADR-002-tenant-consumer-idam-api-boundary.md), [`openapi/idam/tenant-consumer/openapi.yaml`](../../openapi/idam/tenant-consumer/openapi.yaml), Hauliage [`PRD_account-first-onboarding.md`](../../../hauliage/docs/PRD_account-first-onboarding.md)
- **Last updated**: 2026-07-08
- **Paused for**: BRRTRouter [`PRD_IMPL_CONTROLLER_LIFECYCLE.md`](../../../BRRTRouter/docs/PRD_IMPL_CONTROLLER_LIFECYCLE.md)

## What it is

Sesame owns **users, orgs, memberships, invites**, and JWT **`org_id`**. Tenant consumers (Hauliage BFF) call org-mgmt on **8104** and identity-login on **8101** with `X-Tenant-ID: hauliage`.

## Resume here (next session)

1. Redeploy identity-login + org-mgmt after ms02 `cargo check` passes
2. Smoke: register → `POST /idam/v1/organizations` → `POST /idam/v1/sessions/active-organization` → verify JWT `org_id`
3. With Hauliage BFF: full chain through `POST /api/v1/organizations/me`
4. Align demo seed users to `org_memberships` + Hauliage `organization_profiles` (same org UUID)

## Implemented

### JWT `org_id`

- `microservices/idam/common/src/jwt/types.rs`, `builders.rs` — optional top-level `org_id` on access claims
- `identity-login-service/impl/src/services/token_issuer.rs` — `issue_tokens(…, org_id: Option<&str>)`
- `identity-login-service/impl/src/services/org_context.rs` — `resolve_active_org_id` on login / set-active-org

### identity-login-service

| Controller | Path | Notes |
|------------|------|--------|
| `set_active_organization.rs` | `POST /sessions/active-organization` | Re-issue JWT after org create/accept |
| `auth_login.rs` | `POST /auth/login` | Resolves active org when membership exists |
| `auth_register.rs` | `POST /auth/register` | Identity only |
| `auth_logout.rs` | `POST /auth/logout` | Refresh revoke |

OpenAPI: `openapi/idam/identity-login-service/openapi.yaml` — `/sessions/active-organization` added.

Wired in `impl/src/main.rs` via Register & Overwrite (untyped for `set_active_organization`).

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
- Frontend / BFF onboarding UX (Hauliage repo)
- Demo user re-seed for account-first model
- ADR-002 S2+ consumer paths beyond org lifecycle

## Protected impl files

Line 1 on all controllers above: `// BRRTRouter: user-owned`. Required before any `generate-stubs --force`.

## Cross-references

- Hauliage checkpoint: [`../../../hauliage/docs/llmwiki/topics/account-first-onboarding-checkpoint.md`](../../../hauliage/docs/llmwiki/topics/account-first-onboarding-checkpoint.md)
- Tenancy: [`topic-tenancy-model.md`](./topic-tenancy-model.md)
- Login flow: [`topic-login-flow.md`](./topic-login-flow.md)
- Codegen: [`topic-brrtrouter-codegen.md`](./topic-brrtrouter-codegen.md)
- BRRTRouter lifecycle PRD: [`../../../BRRTRouter/docs/PRD_IMPL_CONTROLLER_LIFECYCLE.md`](../../../BRRTRouter/docs/PRD_IMPL_CONTROLLER_LIFECYCLE.md)
