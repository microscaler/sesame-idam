# ADR-004: Platform Tenant Provisioning (SaaS-of-SaaS)

> **Status:** Accepted  
> **Date:** 2026-07-14  
> **Deciders:** Platform (Sesame-IDAM), Microscaler product teams  
> **Related:** [ADR-002](./ADR-002-tenant-consumer-idam-api-boundary.md), [topic-tenancy-model.md](./llmwiki/topics/topic-tenancy-model.md), [topic-platform-tenants.md](./llmwiki/topics/topic-platform-tenants.md), [design-saas-of-saas-multi-tenancy.md](./design-saas-of-saas-multi-tenancy.md)

---

## 1. Context

Sesame-IDAM is evolving into a **SaaS-of-SaaS** identity platform. Each downstream product (`hauliage`, `pricewhisperer`, …) is a **tenant** identified by `X-Tenant-ID`. Dogfooding begins with Microscaler-owned products; external self-service signup follows the same machinery.

Prior OAuth work used per-tenant env vars only. That works for one product but does not scale to:

- Rejecting unknown tenant slugs (security: no magic tenants)
- PriceWhisperer onboarding without hauliage migration
- OAuth credential rotation with audit trail
- Separating **platform tenant OAuth** (product signup) from **org-scoped OIDC `Application`** entities (B2B customers inside a tenant)

---

## 2. Decisions

### 2.1 Tenant minting — both platform admin and self-service

| Path | Who | `provisioning_mode` | Use case |
|------|-----|---------------------|----------|
| Platform admin | Ops / internal CLI | `platform` | Hauliage, PriceWhisperer, dogfood tenants |
| Self-service | SaaS signup API (future) | `self_service` | External customers spinning up their own tenant partition |

Both paths insert into `sesame_idam.tenants` before any auth traffic is accepted.

### 2.2 Unknown tenants — hard reject

Every auth entry point (`/auth/login`, `/auth/register`, `/auth/signup/validate`, `/auth/social/*`) calls `TenantService::require_active` **before** credential or OAuth work.

| Condition | HTTP | `error` |
|-----------|------|---------|
| Slug not in registry | 404 | `tenant_unknown` |
| Tenant suspended / provisioning | 403 | `tenant_not_active` |

No implicit tenant creation from `X-Tenant-ID`. This is a security boundary.

### 2.3 OAuth secrets — K8s/env now, DB holds metadata only

| Stored in DB (`tenant_oauth_providers`) | Stored in K8s secret → pod env |
|----------------------------------------|--------------------------------|
| `client_id` (or `client_id_env_key` override) | `client_secret` via `secret_env_key` |
| `redirect_uris` | — |
| `config_version`, `last_rotated_at`, `last_rotated_by` | — |

Env key pattern (unchanged): `SESAME_OAUTH__{TENANT}__{PROVIDER}_CLIENT_SECRET`.

Rotation flow (critical):

1. Ops updates K8s secret (new client secret).
2. Platform API / CLI calls `POST …/oauth/{provider}/rotate` → bumps `config_version`, records `last_rotated_by` + timestamp.
3. Audit event emitted (future: dedicated `oauth_credential_rotated` type).

### 2.4 Platform tenant OAuth vs org `Application`

| Layer | Scope | Owner service | Purpose |
|-------|-------|---------------|---------|
| `tenant_oauth_providers` | Platform tenant (`hauliage`) | identity-login-service | Google/Microsoft buttons on product signup |
| `Application` (org-mgmt) | Org inside tenant | org-mgmt | B2B OIDC clients, enterprise SSO (P3) |

These must not be conflated. Product social login is **not** an org `Application`.

### 2.5 PriceWhisperer readiness

Seed `hauliage` **and** `pricewhisperer` tenant rows + OAuth metadata now so hauliage launch does not require a later breaking migration when PW starts.

---

## 3. Implementation slice (2026-07-14)

| Component | Location |
|-----------|----------|
| `tenants` entity | `identity-login-service/impl/src/models/tenant.rs` |
| `tenant_oauth_providers` entity | `identity-login-service/impl/src/models/tenant_oauth_provider.rs` |
| Tenant gate | `TenantService::require_active`, `tenant_gate::tenant_http_error` |
| OAuth resolve | `TenantOAuthService::resolve` (DB metadata + env secret) |
| Dev seed | `impl/seeds/20260714000000_platform_tenants.sql` |

**Not yet in this slice:** OpenAPI platform-admin routes (`POST /platform/tenants`, OAuth rotate API), CLI commands, hauliage BFF wiring for social buttons.

---

## 4. Consequences

**Positive**

- Zero bleed: unknown slugs cannot authenticate.
- PW can onboard with its own Google/Microsoft apps without hauliage env bleed.
- Rotation audit fields exist before production cutover.
- Clear separation from org OIDC for enterprise features.

**Negative / follow-up**

- Every new tenant requires registry row (ops or self-service API).
- BDD tests must provision synthetic tenants (`ensure_active_tenant` helper).
- Platform admin API + rotation REST still required for ops ergonomics.

---

## 5. Open questions

> **Open:** Self-service tenant signup — which service owns `POST /platform/tenants/register`? Likely identity-login-service platform surface, not org-mgmt.

> **Open:** Vault integration for secrets — deferred; K8s secrets sufficient for Launch 1.0.
