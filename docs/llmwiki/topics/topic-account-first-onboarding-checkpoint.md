# Account-first onboarding — Sesame implementation checkpoint (2026-07-08)

- **Status**: `partially-verified` — org-admin handlers implemented; `fetch_users_in_org` impl on disk but **deployed image may still serve gen stub** until Tilt rebuild picks up `impl_registry.rs`
- **Source docs**: [`../ADR-002-tenant-consumer-idam-api-boundary.md`](../../ADR-002-tenant-consumer-idam-api-boundary.md), [`openapi/idam/tenant-consumer/openapi.yaml`](../../openapi/idam/tenant-consumer/openapi.yaml), Hauliage [`PRD_account-first-onboarding.md`](../../../hauliage/docs/PRD_account-first-onboarding.md)
- **Last updated**: 2026-07-15

## What it is

Sesame owns **users, orgs, memberships, invites**, and JWT **`org_id`**. Tenant consumers (Hauliage BFF) call org-mgmt on **8104** and identity-login on **8101** with `X-Tenant-ID: hauliage`.

## Resume here (next session)

1. **Deploy org-mgmt with full consumer + org-admin registry** — `brrtrouter-gen regen-impl-registry --apply` wired `fetch_users_in_org`, `remove_user_from_org`, `revoke_pending_invite`, `change_user_role_in_org`; **Tilt rebuild required** so Hauliage BFF stops using company fallback for team list.
2. **Invite token in HTTP response** — unblock Hauliage `real_accept_invite_onboarding.spec.ts`.
3. **Hauliage BFF remove/revoke** — expose org-admin mutations on `POST/DELETE` team routes.

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
| `fetch_users_in_org.rs` | `GET /organizations/{org_id}/users` — **org-admin list** (impl on disk) |
| `change_user_role_in_org.rs` | PATCH role (org admin) |
| `remove_user_from_org.rs` | Remove member |
| `revoke_pending_invite.rs` | Revoke pending invite |

Service: `org_lifecycle.rs` — create, invite, accept, list memberships, **list_org_members**, remove, revoke.

**Registry gotcha:** `main.rs` only `mod`s a subset of controllers; `impl_registry.rs` must list every wired handler. Gen stub for `fetch_users_in_org` returns `page:42` — symptom of missing registry entry or stale image.

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

- **org-mgmt image parity** — ensure deployed pod serves impl `fetch_users_in_org` (not gen `page:42` stub)
- Invite HTTP response should include opaque `invite_token` for accept-invite E2E
- Hauliage BFF remove/revoke team member wiring
- Full OpenAPI regen for all org-mgmt admin stubs (only consumer + org-admin handlers wired in `main.rs`)
- ADR-002 S2+ consumer paths beyond org lifecycle

### identity-session-service (2026-07-14)

- `tenant_db.rs` — profile reads/patches wrapped in `with_pre_auth_tenant` under forced users RLS

## Protected impl files

Line 1 on all controllers above: `// BRRTRouter: user-owned`. Required before any `generate-stubs --force`.

## Cross-references

- Hauliage checkpoint: [`../../../hauliage/docs/llmwiki/topics/account-first-onboarding-checkpoint.md`](../../../hauliage/docs/llmwiki/topics/account-first-onboarding-checkpoint.md)
- Tenancy: [`topic-tenancy-model.md`](./topic-tenancy-model.md)
- Login flow: [`topic-login-flow.md`](./topic-login-flow.md)
- Codegen: [`topic-brrtrouter-codegen.md`](./topic-brrtrouter-codegen.md)
- BRRTRouter lifecycle PRD: [`../../../BRRTRouter/docs/PRD_IMPL_CONTROLLER_LIFECYCLE.md`](../../../BRRTRouter/docs/PRD_IMPL_CONTROLLER_LIFECYCLE.md)
