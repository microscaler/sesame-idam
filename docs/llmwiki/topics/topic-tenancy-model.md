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

Sesame-IDAM uses a **two-level hierarchy: Tenant (isolation boundary) > Application (logical grouping)**.

## The Two Levels

### Tenant — The Isolation Boundary

A **Tenant** represents an entire customer organization. Tenants are completely isolated — **zero bleed** between tenants.

**Examples:**
- `hauliage` = one tenant (the entire hauliage business)
- `rerp` = one tenant (the entire rerp business)

Within a tenant, all users, orgs, and keys share the same data space. `alice@corp.com` on hauliage is a single identity that hauliage-web, hauliage-api, hauliage-admin, and hauliage-mobile all share.

### Application — Logical Grouping Within Tenant

An **Application** is a logical grouping within a tenant. Applications do NOT provide isolation — they are purely organizational.

**Examples (within hauliage tenant):**
- `hauliage-web` = the web frontend application
- `hauliage-api` = the backend API application
- `hauliage-admin` = the admin dashboard
- `hauliage-mobile` = the mobile app

All hauliage applications share the same tenant data (same users, orgs, keys). Applications are just a way to organize and label parts of a tenant's deployment.

**Examples (within rerp tenant):**
- `rerp-openapi` = openapi service
- `rerp-admin` = admin panel
- `rerp-mobile` = mobile app

## Core Rules

1. **`X-Tenant-ID` maps to `tenant_id`** — Every API request includes `X-Tenant-ID` header, which resolves to a `tenant_id` in the database.
2. **Same email on different tenants = unrelated users** — `alice@corp.com` on hauliage and `alice@corp.com` on rerp are completely different users.
3. **UNIQUE(tenant_id, email)** — Prevents duplicate emails within a tenant, but allows the same email across tenants.
4. **No cross-tenant identity** — Users, orgs, keys, sessions, roles, and permissions NEVER cross tenant boundaries.
5. **Applications share tenant data** — All applications within a tenant share the same user base and org structure.

## Database Schema

Every major entity includes a string `tenant_id` column containing the platform tenant slug:

### `users`
| Column | Type | Constraint |
|--------|------|------------|
| `id` | UUID | PK |
| `email` | String | `UNIQUE(tenant_id, email)` |
| `tenant_id` | VARCHAR(255) | `NOT NULL` — partitions data |
| `created_at` | Timestamp | |

### `organizations`
| Column | Type | Constraint |
|--------|------|------------|
| `id` | UUID | PK |
| `name` | String | `UNIQUE(tenant_id, name)` |
| `tenant_id` | VARCHAR(255) | `NOT NULL` |

### `api_keys`
| Column | Type | Constraint |
|--------|------|------------|
| `id` | UUID | PK |
| `key_value` | String | `UNIQUE(tenant_id, key_value)` |
| `tenant_id` | VARCHAR(255) | `NOT NULL` |
| `scope_type` | String | `user` or `org` |

## Isolation Mechanisms (Defense in Depth)

### Layer 1: BRRTRouter Middleware
BRRTRouter cryptographically validates the credential and exposes its `tenant_id` claim. `X-Tenant-ID`, when present, is only a consistency check and cannot establish identity.

### Layer 2: Lifeguard base executors
Protected work uses the existing Lifeguard pool/executor capability to pin a connection, begin a transaction, inject the validated `SessionContext` through the versioned helper, and commit or roll back before pool release. There is no Sesame-specific executor hierarchy.

### Layer 3: PostgreSQL RLS
PostgreSQL RLS policies enforce the appropriate tenant and active-organization accessors. Even if application-layer filtering is missed, the database strips unauthorized rows.

## Deployment Scenarios

### SaaS (Microscaler-hosted)
- **One PostgreSQL instance** serves all tenants
- **One schema** contains all tenant data
- **`tenant_id` column** on every major table is the partition key
- **RLS policies** enforce row-level isolation as a safety net

### Self-Hosted
- **Two schemas** on the same Postgres instance:
  - `app` — Tenant's business logic (orders, products, etc.)
  - `sesame_idam` — Sesame-managed identity tables
- **Schema separation** prevents table name collisions
- Sesame only operates on the `sesame_idam` schema
- The tenant's `X-Tenant-ID` is their own platform ID

## API Key Validation

When `api-keys` validates a key, it returns the `tenant_id` to confirm the key belongs to the correct tenant:

```json
{
  "valid": true,
  "tenant_id": "tenant_hauliage_uuid",
  "org_id": "org_xyz",
  "scope_type": "org",
  "permissions": ["...", "..."]
}
```

The consuming platform validates that the `tenant_id` matches their expected tenant before trusting the response.

## Design Constraints

- **Do** scope all database queries by `tenant_id`
- **Do** return `tenant_id` in all responses (`LoginResponse`, `ApiKeyValidationResponse`)
- **Do** include `tenant_id` in JWT claims (`access_token` payload)
- **Do not** add global uniqueness constraints on `email` — they must be scoped to `tenant_id`
- **Do not** use `tenant_id` in user-facing URLs — it's an internal partition key
- **Do not** allow applications from one tenant to query data from another tenant
