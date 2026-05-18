---
title: Tenant Concept
status: verified
updated: 2026-05-16
sources: [openapi/*/openapi.yaml, microservices/*/impl/src/models/]
---

# Entity: Tenant (Conceptual)

Owned by: **ALL services** (as a boundary concept, not a table)

## Description

**Tenant = Logical isolation boundary.** A tenant represents an entire customer organization. Tenants are completely isolated from each other — zero bleed between tenants. All users, orgs, keys, roles, and permissions within a tenant are shared across all applications in that tenant.

**CRITICAL:** There is NO `tenants` table in the database. Tenants are identified purely by the `tenant_id` column on every other table. The `X-Tenant-ID` request header maps to this column. This is a logical boundary, not a database entity.

**Examples:**
- `hauliage` = one tenant (includes hauliage-web, hauliage-api, hauliage-admin, hauliage-mobile)
- `rerp` = one tenant (includes rerp-openapi, rerp-admin, rerp-mobile)

## How Tenant Isolation Works

Tenant isolation is enforced at three layers:

1. **Application layer:** BRRTRouter middleware extracts `tenant_id` from `X-Tenant-ID` header, appends `WHERE tenant_id = ?` to all queries.
2. **Database layer:** `SesameExecutor` runs `SET LOCAL current_tenant_id = ?` per transaction.
3. **RLS policies:** PostgreSQL policies enforce `WHERE tenant_id = current_tenant_id` as a failsafe.

## tenant_id Column Across All Services

|| Service | Tables with tenant_id |
|---------|----------------------|
| identity-login-service | `users` |
| identity-user-mgmt-service | `users` |
| org-mgmt | `organizations` |
| api-keys | `api_keys`, `api_key_usage` |
| authz-core | `audit_events`, `role_assignments`, `principal_attributes`, `authorizations` |

## Key Design Decisions

1. **Tenant = isolation boundary only.** There is NO `tenants` table. Every table uses `tenant_id` (varchar(255)) as a partitioning column.
2. **Applications are children of tenant.** Each tenant can have multiple applications that share the same data.
3. **X-Tenant-ID maps to tenant_id column.** Every API request header resolves to the tenant boundary.
4. **tenant_id is varchar(255), NOT uuid.** This is consistent across all services.
5. **No soft delete for tenants.** Tenants are deactivated by removing the `X-Tenant-ID` header; there is no `deleted_at` on a non-existent table.

## Drift Found (verified 2026-05-16)

|| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|--------|
| `tenants` table exists | NO TABLE EXISTS — tenant is purely logical | Critical — schema section was fabricated |
| `POST /tenants` | Stale paths; tenant management is conceptual | Medium — no tenant CRUD endpoints |
| `name`, `domain`, `is_active`, `deleted_at` columns | NOT in any table (no tenants table) | Medium — schema was fabricated |
| `tenant_id` is uuid | `tenant_id` is varchar(255) in ALL services | Low — type mismatch |
