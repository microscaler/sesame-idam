---
title: Tenancy Model
status: active
last_updated: 2026-01-22
tags:
  - architecture
  - multi-tenancy
  - isolation
---

# Tenancy Model

Sesame-IDAM uses a **Hard-Segment (partitioned) multi-tenant architecture**. Each subscribing software product ("Tenant") is a completely isolated ecosystem with zero data bleed.

## Core Principles

### No Shared Users Across Tenants

A user is strictly scoped to a single `tenant_id`. The same email address can exist on multiple tenants but represents entirely unrelated identities:

```
Tenant A: user_id=usr_1, email=alice@corp.com, application_id=app_A
Tenant B: user_id=usr_2, email=alice@corp.com, application_id=app_B
```

There is **no cross-tenant identity**. These are two different people who happen to share an email address.

### `X-Tenant-ID` Header

Every API request must identify which tenant it belongs to via:
- **Header:** `X-Tenant-ID: <uuid>` — used for public endpoints (login, register, social)
- **API Key:** Tenant-scoped API keys implicitly carry the tenant identity
- **JWT Claim:** The `tenant_id` claim in every JWT ensures downstream services operate in the correct context

### Single PostgreSQL Schema (SaaS)

In Microscaler's SaaS deployment:
- **One PostgreSQL instance** serves all tenants
- **One schema** (e.g., `public` or `sesame_idam`) contains all tenant data
- **`application_id` column** on every major table is the partition key
- **RLS policies** enforce row-level isolation as a safety net

### Dual Schema (Self-Hosted)

In self-hosted deployments:
- **Two schemas** on the same Postgres instance:
  - `app` — Tenant's business logic (orders, products, etc.)
  - `sesame_idam` — Sesame-managed identity tables
- **Schema separation** prevents table name collisions
- Sesame only operates on the `sesame_idam` schema

## Database Schema

Every major entity includes an `application_id` (UUID) column — the Application entity IS the tenant boundary:

### `users`
| Column | Type | Constraint |
|--------|------|------------|
| `id` | UUID | PK |
| `email` | String | `UNIQUE(application_id, email)` |
| `application_id` | UUID | `NOT NULL` — partitions data (Application FK) |
| `created_at` | Timestamp | |

### `organizations`
| Column | Type | Constraint |
|--------|------|------------|
| `id` | UUID | PK |
| `name` | String | `UNIQUE(application_id, name)` |
| `application_id` | UUID | `NOT NULL` — Application FK |

### `api_keys`
| Column | Type | Constraint |
|--------|------|------------|
| `id` | UUID | PK |
| `key_value` | String | `UNIQUE(application_id, key_value)` |
| `application_id` | UUID | `NOT NULL` — Application FK |
| `scope_type` | String | `user` or `org` |

## Isolation Mechanisms (Defense in Depth)

### Layer 1: Application Context (Primary)

BRRTRouter middleware extracts `tenant_id` from the `X-Tenant-ID` header or JWT and stores it in the request context. All database queries automatically include `WHERE tenant_id = ?`.

### Layer 2: Lifeguard Executor (Transparent)

The `SesameExecutor` wrapper on the database connection injects `SET LOCAL current_tenant_id = ?` at the start of every transaction.

### Layer 3: Row-Level Security (Safety Net)

PostgreSQL RLS policies on every table enforce `WHERE tenant_id = current_tenant_id`. Even if application-layer filtering is missed, the database silently strips cross-tenant data.

## Deployment Models

| Feature | SaaS (Microscaler) | Self-Hosted (Customer) |
|---------|-------------------|----------------------|
| DB Host | Microscaler's cluster | Customer's GCP/VPC |
| Schema | Single shared (`public`) | Dual (`app` + `sesame_idam`) |
| Partition Key | `application_id` column | Schema boundary + `tenant_id` |
| Bleed Protection | RLS + App Layer | Schema boundaries |
| Backups | Single DB dump | Per-database dump |

## Enterprise Escape Hatch (Future)

Schema-per-tenant can be offered as an opt-in "Enterprise" tier for clients requiring:
- **GDPR Data Dumps:** `pg_dump tenant_x_schema` for instant export
- **Massive Scale:** 5M+ users per tenant crushing shared schema performance
- **Compliance:** Physical data isolation requirements

This is **not** the default and requires:
- Schema routing middleware per tenant
- Per-schema migration orchestration
- Separate connection pool per schema

## OpenAPI Implications

Every OpenAPI spec must be updated to:

1. Accept `X-Tenant-ID` header on public endpoints (`/auth/login`, `/auth/register`, `/social/*`)
2. Return `tenant_id` in all responses (`LoginResponse`, `ApiKeyValidationResponse`)
3. Include `tenant_id` in JWT claims (`access_token` payload)
4. Scope all resource endpoints (`/orgs/*`, `/api/keys/*`) to the authenticated tenant

## Pitfalls

- **Do not** add global uniqueness constraints on `email` — they must be scoped to `tenant_id`
- **Do not** assume users exist across tenants — there is no cross-tenant user lookup
- **Do not** skip RLS — application-layer filtering alone is insufficient for "Zero Bleed"
- **Do not** use `tenant_id` in user-facing URLs — it's an internal partition key

## Related

- [RLS Architecture](../reference/ref-rls-architecture.md) — How RLS enforces tenant isolation
- [SesameExecutor Pattern](../reference/ref-rls-architecture.md) — Transparent context injection
- [OpenAPI Multi-Spec Architecture](../topics/topic-openapi-multi-spec-architecture.md) — How specs are structured
