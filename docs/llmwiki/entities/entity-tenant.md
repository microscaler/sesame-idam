---
title: Tenant Entity
status: active
updated: 2026-01-22
sources: [openapi/org-mgmt/openapi.yaml]
---

# Entity: Tenant

Owned by: **org-mgmt**

## Description

**Tenant = Isolation boundary.** A tenant represents an entire customer organization. Tenants are completely isolated from each other — zero bleed between tenants. All users, orgs, keys, roles, and permissions within a tenant are shared across all applications in that tenant.

**Examples:**
- `hauliage` = one tenant (includes hauliage-web, hauliage-api, hauliage-admin, hauliage-mobile)
- `rerp` = one tenant (includes rerp-openapi, rerp-admin, rerp-mobile)

## Schema (from OpenAPI)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| name | text | |
| domain | text | Primary domain |
| is_active | boolean | |
| created_at | timestamptz | |
| updated_at | timestamptz | |
| deleted_at | timestamptz | Soft delete |

## Key Design Decisions

1. **Tenant = isolation boundary.** All entities (users, orgs, keys) are partitioned by `tenant_id`.
2. **Applications are children of tenant.** Each tenant can have multiple applications that share the same data.
3. **X-Tenant-ID maps to tenant.id.** Every API request header resolves to the tenant boundary.
4. **Soft deletes for auditability.** Tenants can be deactivated without data loss.

## API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/am/tenants` | GET | List tenants |
| `/api/v1/am/tenants` | POST | Create tenant |
| `/api/v1/am/tenants/{tenant_id}` | GET | Get tenant |
| `/api/v1/am/tenants/{tenant_id}/applications` | GET | List tenant applications |
| `/api/v1/am/tenants/{tenant_id}/applications` | POST | Create application |
| `/api/v1/am/tenants/{tenant_id}/applications/{app_id}` | GET | Get application |

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition
- `openapi/org-mgmt/openapi.yaml` — Tenant CRUD API

## Gaps / Drift

> **Open:** Verify actual Lifeguard model. Implementations may not yet support all endpoints.
