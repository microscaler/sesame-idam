---
title: Application Entity
status: partially-verified
updated: 2026-01-22
sources: [openapi/org-mgmt/openapi.yaml]
---

# Entity: Application

Owned by: **org-mgmt**

## Description

**Application = Tenant boundary.** Each consuming platform (Software X, Software Y) is one application that acts as a tenant of Sesame-IDAM. An application has its own users, organizations, API keys, roles, and permissions. The `X-Tenant-ID` header maps to the application ID.

## Schema (from OpenAPI)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| name | text | |
| slug | text | |
| platform | text | Application domain (e.g., "myapp.com") |
| is_active | boolean | |
| tenant_id | uuid (FK) | **REQUIRED** — application IS the tenant (same as `application.id`, alias for `application_id` column in other tables) |
| created_at | timestamptz | |
| updated_at | timestamptz | |
| deleted_at | timestamptz | Soft delete |

## Key Design Decisions

1. **Application = Tenant boundary.** Each consuming platform (Software X, Software Y) is one application that acts as a tenant of Sesame-IDAM. An application has its own users, organizations, API keys, roles, and permissions. The `X-Tenant-ID` header maps to the application ID.
2. **Lifecycle management.** Applications can be activated/deactivated without data loss (soft delete pattern).

## API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/am/applications` | GET | List applications |
| `/api/v1/am/applications` | POST | Create application |
| `/api/v1/am/applications/{app_id}` | GET | Get application |
| `/api/v1/am/applications/{app_id}/roles` | GET | List application roles |
| `/api/v1/am/applications/{app_id}/roles` | POST | Create role |
| `/api/v1/am/applications/{app_id}/roles/{role_id}` | GET | Get role |
| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | GET | List role permissions |
| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | POST | Assign permission |
| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | DELETE | Revoke permission |
| `/api/v1/am/applications/{app_id}/permissions` | GET | List permissions |
| `/api/v1/am/applications/{app_id}/permissions` | POST | Create permission |

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition
- `openapi/org-mgmt/openapi.yaml` — Application CRUD API

## Gaps / Drift

> **Open:** Verify actual Lifeguard model. Implementations may not yet support all endpoints.
