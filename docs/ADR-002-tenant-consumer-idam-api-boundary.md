# ADR-002: Tenant Consumer IDAM API Boundary

> **Status:** Proposed  
> **Date:** 2026-07-08  
> **Deciders:** Platform (Sesame-IDAM), Hauliage, future SaaS tenants  
> **Related:** [topic-tenancy-model.md](./llmwiki/topics/topic-tenancy-model.md), [ADR-001-org-type-classification.md](./adr-001-org-type-classification.md), [cross-repo-auth-analysis.md](./cross-repo-auth-analysis.md)

---

## 1. Context

Sesame-IDAM is a **standalone identity platform** (PropelAuth-class): separate deployment, separate database (`sesame_idam`), reachable only over HTTPS. Dev may colocate Postgres with Hauliage; production may run Sesame in another region or cloud (e.g. Azure).

Each **SaaS product** is a Sesame **tenant**:

| Sesame `tenant_id` | Product database | Example orgs |
|--------------------|------------------|--------------|
| `hauliage` | `hauliage` schema / DB | Many shipper orgs, many transporter orgs |
| `pricewhisperer` | PriceWhisperer DB | Trader orgs, etc. |
| `rerp` | RERP DB | … |

Within one tenant:

- **Users** authenticate once (`UNIQUE(tenant_id, email)`).
- **Organizations** are B2B workspaces (customer companies), not the SaaS product itself.
- **Membership** links users to orgs; **invites** are sent **by Sesame** (email + magic link).

Hauliage today duplicates Sesame concerns in the company service (`team_members`, `principal_organization_map`, invite tokens). That cannot scale to PriceWhisperer or external deployment.

---

## 2. Decision

### 2.1 Single source of truth (Sesame DB only)

| Concern | Owner | Never duplicate in product DB |
|---------|--------|-------------------------------|
| User credentials & profile core | identity-login / user-mgmt | — |
| Organization id, name, tenant scope | org-mgmt | — |
| Membership (user ↔ org, role, pending/active) | org-mgmt | — |
| Invitations (token, email, expiry, **send email**) | org-mgmt | — |
| JWT claims: `sub`, `tenant_id`, **`org_id`**, roles | login + session | — |
| Product org profile (SHIPPER/HAULIER, compliance, fleet…) | Hauliage company | Keyed by **`sesame_org_id` FK** |
| Jobs, quotes, consignments | Hauliage domain services | Scoped by JWT `org_id` |

### 2.2 Integration contract (cross-cloud safe)

Products integrate with Sesame **only** via:

1. **HTTP APIs** under `/idam/v1/*` with mandatory `X-Tenant-ID: hauliage` (or product tenant slug).
2. **JWT validation** (JWKS from session-service) — no shared Postgres, no cross-DB joins.
3. **Optional webhooks** (`invite.accepted`, `org.created`) for async domain provisioning.

No product service reads `sesame_idam` tables directly in production.

### 2.3 OpenAPI layout

Split Sesame specs by **consumer**, not by adding Hauliage paths into Sesame:

| Spec | Audience | Services |
|------|----------|----------|
| `identity-login-service/openapi.yaml` | End user + BFF | register, login, refresh, signup/validate |
| **`tenant-consumer/openapi.yaml`** (new) | Product BFF + SPA (via BFF) | Self-service org lifecycle, memberships, invite accept |
| `org-mgmt/openapi.yaml` | Platform admin + enterprise | SCIM, SAML, webhooks, bulk admin (existing) |
| `authz-core/openapi.yaml` | Internal + login enrichment | roles, authorize |
| `identity-user-mgmt-service/openapi.yaml` | Admin / support | user CRUD, block, metadata |

**Do not** add Hauliage-specific routes to Sesame. Hauliage keeps `openapi/company/openapi.yaml` for **domain org profile** only.

---

## 3. Tenant consumer API (new spec)

Base: `/idam/v1` · Header: `X-Tenant-ID` · Auth: Bearer JWT unless noted.

### 3.1 End-user / BFF operations (implement in org-mgmt + login)

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/auth/register` | Account only (login-service, exists) |
| GET | `/users/me` | Profile + **active org summary** (user-mgmt) |
| GET | `/users/me/memberships` | Orgs user belongs to (pending + active) |
| POST | `/organizations` | **Create org**; caller becomes owner membership |
| GET | `/organizations/{org_id}` | Org metadata (member or admin) |
| POST | `/organizations/{org_id}/invitations` | **Invite by email; Sesame sends mail** (exists, wire email) |
| POST | `/invitations/accept` | Body `{ token }`; accept magic link (authenticated user) |
| GET | `/invitations/preview` | Query `token`; public org name + inviter for UX (optional) |
| POST | `/sessions/active-organization` | Set active org; **re-issue or refresh JWT with `org_id`** |

### 3.2 JWT requirements

After org create or invite accept, access token must include:

```json
{
  "sub": "<user_id>",
  "tenant_id": "hauliage",
  "org_id": "<uuid>",
  "email": "user@example.com",
  "https://sesame-idam.dev/claims": {
    "tenant": "hauliage",
    "roles": [],
    "permissions": []
  }
}
```

Login with **no membership** returns token **without** `org_id` (valid for onboarding only).

Login may accept optional `organization_id` when user has multiple memberships (already in login OpenAPI examples).

### 3.3 Org “type” for Hauliage (shipper vs transporter)

**Not** a Sesame platform enum (see ADR-001: `provider` / `consumer` / `platform`).

Hauliage-specific classification lives in **product metadata**:

- **Preferred:** `POST /organizations` body `metadata: { "hauliage_profile_type": "SHIPPER" | "HAULIER" }` (tenant-scoped convention documented in Hauliage PRD).
- **Also:** Hauliage company service stores `profile_type` on `organization_profiles.sesame_org_id` at provision time.

Other tenants use their own metadata keys; Sesame stores opaque JSON on org if/when impl model grows.

---

## 4. Hauliage: rip out vs keep

### Remove from Hauliage (move to Sesame consumer API)

| Hauliage artifact | Replacement |
|-------------------|-------------|
| `principal_organization_map` | Sesame membership + JWT `org_id` |
| `team_members` (identity rows) | Sesame `org_memberships` + invites |
| `invite_team_member` / `accept_organization_invite` (company) | BFF → Sesame `POST .../invitations`, `POST /invitations/accept` |
| `team_member` invite_token columns | Sesame `org_invites.token` |
| BFF/company `provision_organization` membership inserts | Sesame `POST /organizations` then Hauliage profile hook |

### Keep in Hauliage company service

| Artifact | Notes |
|----------|--------|
| `organization_profiles` | Add **`sesame_organization_id UUID UNIQUE NOT NULL`** |
| Addresses, compliance docs, preferences, fleet views | Domain data |
| `GET /organizations/me` | Resolve JWT `org_id` → load hauliage profile (404 if profile not provisioned yet) |
| `POST /organizations/me/profile` (rename from conflated create) | **Domain bootstrap** after Sesame org exists |

### Hauliage BFF orchestration (thin)

```
Register     → Sesame login-service
Create org   → Sesame POST /organizations
             → Hauliage POST /internal/org-profiles { sesame_org_id, profile_type, name }
Invite       → Sesame POST /organizations/{id}/invitations  (Sesame emails)
Accept       → Sesame POST /invitations/accept
             → optional: ensure Hauliage profile exists (idempotent)
Dashboard    → JWT org_id on all domain calls
```

---

## 5. Async provisioning (recommended)

Because Sesame and Hauliage DBs are separate:

1. **Sync path (MVP):** BFF chains Sesame create → Hauliage profile create; rollback/compensate on failure.
2. **Target path:** Sesame webhook `org.created` / `membership.created` → Hauliage worker creates profile row.

Both avoid cross-database transactions.

---

## 6. Implementation phases

| Phase | Sesame | Hauliage |
|-------|--------|----------|
| **S0** | Publish `tenant-consumer/openapi.yaml`; stub accept + create org | Stop new `principal_organization_map` features |
| **S1** | Implement create org, accept invite, send invite email | BFF proxies; bridge table read-only for demo seeds |
| **S2** | JWT `org_id` on login/refresh; `/users/me/memberships` | Company keyed by `sesame_org_id`; delete duplicate team/invite |
| **S3** | Webhooks | Worker-based profile provisioning |

---

## 7. Consequences

- PriceWhisperer (tenant `pricewhisperer`) reuses the **same** Sesame consumer API; only `X-Tenant-ID` and domain DB change.
- Hauliage company service shrinks to **domain org profile**, not identity.
- org-mgmt OpenAPI remains large; **tenant-consumer** is the doc product teams read (like PropelAuth “Frontend APIs”).
- Until S1 ships, Hauliage bridge tables are **explicitly temporary** (demo/ms02 only).

---

## 8. Open questions

1. Multi-org users: org switcher vs single active org in JWT (v1: single active).
2. Invite email templates: per-tenant branding in Sesame vs product-supplied template id.
3. Service account for BFF: API key vs user-delegated OAuth for server-side org admin actions.
