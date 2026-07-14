# Topic: Platform tenant registry (SaaS-of-SaaS)

> **Status:** verified (2026-07-14)  
> **ADR:** [ADR-004](../../ADR-004-platform-tenant-provisioning.md)  
> **Design (canonical):** [design-saas-of-saas-multi-tenancy.md](../../design-saas-of-saas-multi-tenancy.md)  
> **Epic:** [10-platform-tenancy](../../Epics/10-platform-tenancy/README.md)  
> **PRD (P1):** [PRD-P1-platform-tenant-admin.md](../../PRD-P1-platform-tenant-admin.md)

## What it is

Sesame runs as **SaaS-of-SaaS**: each product (`hauliage`, `pricewhisperer`) or future buyer partition is a **platform tenant** in `sesame_idam.tenants`. The slug matches `X-Tenant-ID`. Tenants must be **provisioned** before any auth endpoint accepts traffic — no magic slugs.

**Not a tenant:** shipper vs transport org inside hauliage — those are **organizations** under tenant `hauliage`.

## Tables

| Table | Purpose |
|-------|---------|
| `tenants` | Registry: slug, display_name, status, provisioning_mode |
| `tenant_oauth_providers` | Per-tenant Google/Microsoft **metadata** (secrets in K8s/env) |

Migrations: `migrations/identity-login-service/20260714102157_{tenants,tenant_oauth_providers}.sql`

## Tenant status

| Status | Auth |
|--------|------|
| `provisioning` | Blocked (`403 tenant_not_active`) |
| `active` | Allowed |
| `suspended` | Blocked |
| `deprovisioned` | Blocked |
| `failed` | Blocked |

## Auth gate

`TenantService::require_active` before credential/OAuth work:

| Condition | HTTP | `error` |
|-----------|------|---------|
| Unknown slug | 404 | `tenant_unknown` |
| Non-active | 403 | `tenant_not_active` |

**Wired:** `auth_login`, `auth_register`, `signup_validate`, `social_login`, `social_callback`.

## OAuth resolution

`TenantOAuthService::resolve(tenant, provider)` — caller must gate tenant first:

1. Load enabled row from `tenant_oauth_providers`
2. Read `client_secret` from env via `secret_env_key`
3. Optional `client_id` override via `client_id_env_key`

Distinct from org-mgmt `Application` (org-scoped B2B OIDC).

## Minting paths

| Mode | `provisioning_mode` | Today | Target (P1) |
|------|---------------------|-------|-------------|
| Platform ops | `platform` | SQL seed + `ensure_active_tenant` in BDD | `POST /platform/tenants` + CLI |
| Self-service | `self_service` | — | P2 worker + store |

## Platform API (P1 — in progress)

Tag: `PlatformAdmin` on identity-login-service OpenAPI. Auth: `X-Platform-Admin-Key` (`PlatformServiceAuth`).

| Method | Path | Story |
|--------|------|-------|
| `POST` | `/platform/tenants` | 10.2 |
| `GET` | `/platform/tenants/{slug}` | 10.2 |
| `PATCH` | `/platform/tenants/{slug}/status` | 10.3 |
| `PUT` | `/platform/tenants/{slug}/oauth/{provider}` | 10.4 |
| `POST` | `/platform/tenants/{slug}/oauth/{provider}/rotate` | 10.5 |

## Dev seed

`identity-login-service/impl/seeds/20260714000000_platform_tenants.sql` — hauliage + pricewhisperer tenants and OAuth metadata (idempotent).

## Code anchors

| Area | Path |
|------|------|
| Models | `impl/src/models/tenant.rs`, `tenant_oauth_provider.rs` |
| Services | `tenant_service.rs`, `tenant_oauth_service.rs`, `tenant_gate.rs` |
| OAuth | `services/oauth/{config,state,providers}.rs` |
| Seed | `impl/seeds/20260714000000_platform_tenants.sql` |
| BDD helper | `impl/tests/common/mod.rs` → `ensure_active_tenant` |
| BDD gate | `impl/tests/bdd/auth_flow.rs` → `unknown_tenant_rejected` |

## Build order (Epic 10 P1)

`10.1` OpenAPI → `10.7` platform auth → `10.2` create/get → `10.3` status → `10.4` oauth PUT → `10.5` rotate → `10.6` CLI → `10.8` BDD

## Gaps / drift

> **Open:** Platform REST not yet implemented (story 10.1+). Rotation audit event type may need `AuditEventType` extension (10.5).
>
> **Open:** K8s secrets still manual per tenant; self-service dynamic secrets deferred to P4.
