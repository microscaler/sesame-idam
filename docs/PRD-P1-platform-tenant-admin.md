# PRD-P1: Platform Tenant Admin API & CLI

**Date:** 2026-07-14  
**Status:** Draft — ready for implementation  
**Phase:** P1 (Platform admin — pre–online store)  
**Authors:** Platform (Sesame-IDAM)  
**Epic:** [10-platform-tenancy](./Epics/10-platform-tenancy/README.md)  
**Design source:** [design-saas-of-saas-multi-tenancy.md](./design-saas-of-saas-multi-tenancy.md) §8.1, §12, §17 P1  
**Depends on:** [ADR-004](./ADR-004-platform-tenant-provisioning.md) foundation (tenant registry, auth gate — **implemented**)  
**Blocks:** PRD-P2 (self-service provisioning worker), PriceWhisperer ops onboarding without SQL seeds

---

## 1. Executive Summary

Sesame-IDAM now has a **tenant registry** (`tenants`, `tenant_oauth_providers`) and an **auth gate** that rejects unknown slugs. Hauliage and PriceWhisperer are still provisioned via **manual SQL seeds** and per-tenant K8s env vars.

**PRD-P1** replaces manual ops with a **platform-admin REST API** and matching **`sesame-idam` CLI** so operators can:

- Mint tenants (`provisioning_mode = platform`)
- Suspend / reactivate tenants
- Configure per-tenant OAuth **metadata** (secrets remain in K8s/env)
- Record OAuth credential rotations with audit trail

This is the **dogfood path** required before the online store (PRD-P3) and the provisioning worker (PRD-P2). No Stripe, no store UI, no self-service signup in P1.

---

## 2. Problem Statement

| Today | Problem |
|-------|---------|
| `20260714000000_platform_tenants.sql` seed | Requires SQL access; not repeatable in CI; drift risk |
| No REST surface for tenant lifecycle | Ops cannot suspend a tenant without DB access |
| OAuth metadata only in seed file | Rotation/version bump not operable without code deploy |
| `TenantOAuthService::record_rotation` exists | No HTTP/CLI caller |
| PriceWhisperer onboarding | Would copy hauliage seed pattern — same manual tax |

### Why now

- Tenant registry and auth gate are **implemented and tested** (86 tests on ms02).
- PriceWhisperer launch needs a **second tenant** without a one-off migration for hauliage.
- Platform API is the **shared contract** the P2 provisioning worker will call later (`POST /platform/tenants` is a subset of provision).

---

## 3. Goals

| # | Goal | Evidence |
|---|------|----------|
| G1 | Mint tenant via API/CLI in &lt; 2 min, no SQL | `POST /platform/tenants` → `active` |
| G2 | Suspend tenant blocks all auth immediately | `PATCH status=suspended` → login returns `403 tenant_not_active` |
| G3 | OAuth metadata manageable without DB | `PUT …/oauth/{provider}` updates row; secret not in response body |
| G4 | Rotation bumps `config_version` + audit | `POST …/rotate` → version+1; audit event emitted |
| G5 | CLI parity with REST | `sesame-idam tenant create` matches API behaviour |
| G6 | Platform routes not callable by end-user JWT | Unauthenticated / wrong credential → `401` |
| G7 | Hauliage + PW migratable off seeds | Runbook: recreate tenants via CLI; seeds deprecated |

---

## 4. Non-Goals (P1)

| Item | Phase |
|------|-------|
| `POST /platform/tenants/provision` (Stripe idempotency) | P2 |
| Store UI, Stripe Checkout, KYC tables | P3 |
| Vault / dynamic secret fetch | P4 |
| Tenant admin portal (post-provision wizard) | P3 |
| Auto-create K8s Secret objects | P1 docs only — ops still apply secrets out-of-band |
| Hauliage BFF social proxy | Cross-repo consumer story C1 |
| Self-service `provisioning_mode = self_service` via public API | P2 |

---

## 5. Functional Requirements

### FR-P1-001 — Platform OpenAPI surface

Add tag **`PlatformAdmin`** to `openapi/idam/identity-login-service/openapi.yaml` (or sibling `platform-admin/openapi.yaml` merged at codegen — **prefer single spec** per repo convention).

Paths under `/platform/tenants/*` on base `/idam/v1`.

Codegen via `just gen-identity-login`; impl controllers in `impl/src/controllers/platform_*`.

### FR-P1-002 — Create tenant

**`POST /platform/tenants`**

Request:

```json
{
  "slug": "pricewhisperer",
  "display_name": "PriceWhisperer",
  "provisioning_mode": "platform",
  "activate": true
}
```

Behaviour:

1. Validate slug: `^[a-z][a-z0-9-]{2,63}$`, reserved slugs blocked (`admin`, `platform`, `www`, …).
2. Insert `tenants` with `status = provisioning` if `activate=false`, else `active`.
3. `provisioning_mode` defaults to `platform`; reject `self_service` on this endpoint (P2 only).
4. Duplicate slug → `409 slug_taken`.

Response `201`:

```json
{
  "id": "uuid",
  "slug": "pricewhisperer",
  "display_name": "PriceWhisperer",
  "status": "active",
  "provisioning_mode": "platform",
  "created_at": "…"
}
```

### FR-P1-003 — Get tenant

**`GET /platform/tenants/{slug}`**

Response `200`: tenant row + list of enabled `tenant_oauth_providers` (metadata only — no secret values).

`404 tenant_not_found` if slug absent.

### FR-P1-004 — Update tenant status

**`PATCH /platform/tenants/{slug}/status`**

Request:

```json
{ "status": "suspended" }
```

Allowed transitions (P1):

| From | To |
|------|-----|
| `active` | `suspended` |
| `suspended` | `active` |
| `active` | `deprovisioned` |
| `provisioning` | `active` |
| `provisioning` | `failed` |

Invalid transition → `409 invalid_status_transition`.

Side effect: status change emits audit event `tenant_status_changed`.

### FR-P1-005 — Upsert OAuth metadata

**`PUT /platform/tenants/{slug}/oauth/{provider}`**

`provider` ∈ `google`, `microsoft`.

Request:

```json
{
  "client_id": "…",
  "redirect_uris": ["http://pw.dev.microscaler.local/oauth/callback"],
  "secret_env_key": "SESAME_OAUTH__PRICEWHISPERER__GOOGLE_CLIENT_SECRET",
  "client_id_env_key": "SESAME_OAUTH__PRICEWHISPERER__GOOGLE_CLIENT_ID",
  "enabled": true
}
```

Behaviour:

- Tenant must exist and not be `deprovisioned`.
- Delegates to `TenantOAuthService::upsert_metadata`.
- Response excludes secret values; returns `config_version`.

Ops runbook: apply K8s Secret / env separately; API only registers metadata.

### FR-P1-006 — Record OAuth rotation

**`POST /platform/tenants/{slug}/oauth/{provider}/rotate`**

Request:

```json
{ "rotated_by": "ops@microscaler.dev" }
```

Behaviour:

- Assumes secret already updated in K8s/env out-of-band.
- Calls `TenantOAuthService::record_rotation`.
- Emits audit `oauth_credential_rotated` (extend `AuditEventType` if missing).
- Response: `{ "config_version": 2 }`.

### FR-P1-007 — Platform route authentication

Platform routes require **`PlatformServiceAuth`** security scheme:

| Mechanism (P1) | Detail |
|----------------|--------|
| API key header | `X-Platform-Admin-Key` matched against env `SESAME_PLATFORM_ADMIN_KEY` |
| Future | mTLS service identity (P4) |

- Missing/invalid key → `401 unauthorized`.
- End-user Bearer JWT → `403 forbidden` (even if valid).
- Public auth routes unchanged.

### FR-P1-008 — CLI commands

Extend `sesame-idam` tooling (`tooling/`) with subcommands:

```bash
sesame-idam tenant create --slug SLUG --display-name NAME [--no-activate]
sesame-idam tenant get --slug SLUG
sesame-idam tenant status set --slug SLUG --status active|suspended|deprovisioned
sesame-idam tenant oauth set --slug SLUG --provider google|microsoft \
  --client-id ID --redirect-uris URI[,URI...] --secret-env-key ENV_KEY \
  [--client-id-env-key ENV_KEY]
sesame-idam tenant oauth rotate --slug SLUG --provider PROVIDER --by EMAIL
```

CLI calls login-service HTTP API (not direct DB). Uses `SESAME_LOGIN_SERVICE_URL` + `SESAME_PLATFORM_ADMIN_KEY`.

---

## 6. API Summary

| Method | Path | Story |
|--------|------|-------|
| `POST` | `/platform/tenants` | 10.2 |
| `GET` | `/platform/tenants/{slug}` | 10.2 |
| `PATCH` | `/platform/tenants/{slug}/status` | 10.3 |
| `PUT` | `/platform/tenants/{slug}/oauth/{provider}` | 10.4 |
| `POST` | `/platform/tenants/{slug}/oauth/{provider}/rotate` | 10.5 |

---

## 7. Security Requirements

| ID | Requirement |
|----|-------------|
| SEC-P1-01 | Platform routes excluded from global Bearer default security |
| SEC-P1-02 | Slug enumeration on public auth paths unchanged (`tenant_unknown` generic) |
| SEC-P1-03 | Platform GET may return `slug_taken` on create only — not on public paths |
| SEC-P1-04 | OAuth PUT/rotate audit-logged with `tenant_slug`, `provider`, `rotated_by` |
| SEC-P1-05 | No secret value in any response body or audit payload |

---

## 8. Operational Runbook (target)

### Mint PriceWhisperer (replaces seed)

```bash
export SESAME_PLATFORM_ADMIN_KEY=…
export SESAME_LOGIN_SERVICE_URL=http://identity-login-service.sesame-idam:8080/idam/v1

sesame-idam tenant create --slug pricewhisperer --display-name "PriceWhisperer"
sesame-idam tenant oauth set --slug pricewhisperer --provider google \
  --client-id "$PW_GOOGLE_CLIENT_ID" \
  --redirect-uris "http://pricewhisperer.dev.microscaler.local/oauth/callback" \
  --secret-env-key SESAME_OAUTH__PRICEWHISPERER__GOOGLE_CLIENT_SECRET \
  --client-id-env-key SESAME_OAUTH__PRICEWHISPERER__GOOGLE_CLIENT_ID
# Apply K8s secret separately (Helm/Tilt)
```

### Suspend abusive tenant

```bash
sesame-idam tenant status set --slug bad-actor --status suspended
```

---

## 9. Test Plan

| Layer | Tests |
|-------|-------|
| Unit | Slug validation, status transition matrix, service delegation |
| BDD | 10.8: CLI/API mint → `POST /auth/register` → login succeeds |
| BDD | Suspend → login `403 tenant_not_active` |
| BDD | OAuth rotate → `config_version` increment |
| Security | Platform route rejects user JWT and missing API key |
| Lint | `just lint-openapi`, `just lint-rust`, `just nt` |

---

## 10. Story Map

| Story | Title | Points (est.) |
|-------|-------|---------------|
| [10.1](./Epics/10-platform-tenancy/stories/story-10.1.md) | Platform OpenAPI + codegen | M |
| [10.2](./Epics/10-platform-tenancy/stories/story-10.2.md) | Create + get tenant | M |
| [10.3](./Epics/10-platform-tenancy/stories/story-10.3.md) | Tenant status PATCH | S |
| [10.4](./Epics/10-platform-tenancy/stories/story-10.4.md) | OAuth metadata PUT | M |
| [10.5](./Epics/10-platform-tenancy/stories/story-10.5.md) | OAuth rotate + audit | S |
| [10.6](./Epics/10-platform-tenancy/stories/story-10.6.md) | CLI commands | M |
| [10.7](./Epics/10-platform-tenancy/stories/story-10.7.md) | Platform service auth | M |
| [10.8](./Epics/10-platform-tenancy/stories/story-10.8.md) | BDD e2e mint → auth | M |

**Suggested order:** 10.1 → 10.7 → 10.2 → 10.3 → 10.4 → 10.5 → 10.6 → 10.8

---

## 11. Acceptance Gate (P1 complete)

- [ ] All stories 10.1–10.8 marked done per [CONTRIBUTING.md](./CONTRIBUTING.md) gates
- [ ] Hauliage + PW tenants creatable via CLI without SQL seed (seed file deprecated in runbook)
- [ ] `just nt` passes on ms02
- [ ] Wiki `topic-platform-tenants.md` updated with API paths
- [ ] No regression: `tenant_unknown` still returned for unprovisioned slugs on `/auth/*`

---

## 12. Related Documents

| Document | Link |
|----------|------|
| Design (canonical) | [design-saas-of-saas-multi-tenancy.md](./design-saas-of-saas-multi-tenancy.md) |
| ADR-004 | [ADR-004-platform-tenant-provisioning.md](./ADR-004-platform-tenant-provisioning.md) |
| Next PRD | PRD-P2-self-service-provisioning (not yet written) |
| Consumer boundary | [ADR-002](./ADR-002-tenant-consumer-idam-api-boundary.md) |

---

*When implementation diverges from this PRD, update the PRD and design doc in the same change set.*
