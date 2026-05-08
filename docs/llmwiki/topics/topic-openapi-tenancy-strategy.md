---
title: OpenAPI Spec Strategy for Multi-Tenancy
status: active
last_updated: 2026-01-22
tags:
  - openapi
  - multi-tenancy
  - codegen
  - architecture
---

# OpenAPI Spec Strategy for Multi-Tenancy

## Decision: One Global Spec, Per-Tenant Data

Sesame-IDAM uses a **single OpenAPI specification per service**, shared across all tenants. The API surface (paths, methods, request/response shapes) is identical for every tenant. Tenant isolation is enforced at the **infrastructure layer** (middleware + database), not at the API contract level.

## Why Not One Spec Per Tenant?

| Factor | Global Spec | Per-Tenant Specs |
|--------|-------------|------------------|
| Codegen | One `just gen` for all tenants | N regenerations per tenant |
| Testing | Single spec, one tenant per test run | N specs to maintain |
| Documentation | One source of truth | N docs to sync |
| Complexity | Low | High (spec drift risk) |
| Standard | Used by Auth0, Clerk, Stripe | Rare in SaaS |

**The API contract is the same for everyone.** The data inside each tenant's context is what differs.

## How It Works

### The Spec (Code/Contract)

```yaml
# openapi/identity-login-service/openapi.yaml
paths:
  /login:
    post:
      summary: Authenticate a user
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                email: { type: string }
                password: { type: string }
      responses:
        '200':
          description: Successful login
```

**This spec is shared by Tenant A, Tenant B, and all future tenants.**

### The Runtime (Data/Isolation)

At runtime, the `X-Tenant-ID` header is added **outside the spec** by the infrastructure:

```bash
# Tenant A login
POST /login
X-Tenant-ID: tenant_a
Body: { "email": "alice@corp.com", "password": "secret" }

# Tenant B login
POST /login
X-Tenant-ID: tenant_b
Body: { "email": "alice@corp.com", "password": "secret" }
```

Same endpoint, same body, different `X-Tenant-ID`. The middleware intercepts the header, resolves the tenant context, and the database queries automatically scope to that tenant.

## Implementation Layers

### Layer 1: BRRTRouter Middleware

A global middleware runs on every request:
1. Extracts `X-Tenant-ID` from the header (or JWT/API key context)
2. Resolves it to a `tenant_id` UUID
3. Injects it into the request context (`BRRTRouterContext::tenant_id`)
4. All subsequent handlers have access to `current_tenant_id`

### Layer 2: Lifeguard / SesameExecutor

The database wrapper intercepts queries:
1. At transaction start: `SET LOCAL current_tenant_id = ?`
2. Every query automatically includes `WHERE tenant_id = ?`
3. No code changes needed in handlers — context is transparent

### Layer 3: PostgreSQL RLS (Safety Net)

Database policies enforce isolation:
```sql
CREATE POLICY tenant_isolation ON users
  FOR ALL
  USING (tenant_id = current_setting('app.tenant_id'));
```

If a handler accidentally forgets to scope by tenant, the database silently strips cross-tenant data.

## OpenAPI Spec Updates

While the specs are global, they **do need to reflect tenant-awareness** in a few places:

1. **`/login`, `/register`, `/social/*` endpoints** — Accept `X-Tenant-ID` header
2. **`TokenResponse` / `LoginResponse`** — Return `tenant_id` in the body
3. **`ApiKeyValidationResponse`** — Return `tenant_id` in the body
4. **JWT payload** — `tenant_id` claim included (not in OpenAPI, but documented in spec descriptions)

## When Would You Need Per-Tenant Specs?

Rare cases where Approach B is insufficient:
- A tenant needs a **completely different API contract** (e.g., they fork the service with custom endpoints)
- A tenant requires **on-premise deployment** with their own codegen and isolated infrastructure
- A tenant needs **custom SDK generation** tailored to their specific usage

In these cases, the spec can be forked, but this is an **Enterprise opt-in** feature, not the default.

## Pitfalls

- **Do not** try to generate tenant-specific OpenAPI specs — it's unnecessary complexity
- **Do not** embed tenant IDs in URL paths (`/{tenant_id}/login`) — the header pattern is cleaner and standard
- **Do not** assume the spec describes data — it describes **behavior and contracts**, not database contents
- **Do** document the `X-Tenant-ID` header in each relevant endpoint's spec description

## Related

- [Tenancy Model](./topic-tenancy-model.md) — How tenants are isolated
- [RLS Architecture](./ref-rls-architecture.md) — How RLS enforces tenant isolation
- [BRRTRouter Codegen](./topic-brrtrouter-codegen.md) — Spec-to-code workflow
