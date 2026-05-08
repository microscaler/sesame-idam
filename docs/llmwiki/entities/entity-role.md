---
title: Role Entity
status: partially-verified
updated: 2026-01-22
sources: [openapi/org-mgmt/openapi.yaml]
---

# Entity: Role

Owned by: **org-mgmt** (evaluated by authz-core for permission checks)

## Description

Role model with inheritance support. Roles are per-application, scoped to organizations. Platform-level roles have `organization_id: NULL`.

## Schema (from OpenAPI)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| tenant_id | uuid (FK) | |
| organization_id | uuid (FK, nullable) | NULL = platform role |
| name | text | Internal name |
| display_name | text | Human-readable name |
| description | text | |
| is_system | boolean | System roles cannot be modified/deleted |
| parent_role_id | uuid (FK, self-ref) | Role inheritance chain |
| created_at | timestamptz | |
| updated_at | timestamptz | |

## RolePermission Association

| Column | Type | Notes |
|--------|------|-------|
| role_id | uuid (FK, PK composite) | |
| permission_id | uuid (FK, PK composite) | |

## Key Design Decisions

1. **Per-application roles.** A role belongs to an application and optionally to an organization.
2. **Role inheritance.** `parent_role_id` creates a hierarchy. Effective permissions are resolved by walking the chain.
3. **System roles.** `is_system` flag prevents modification of built-in roles (admin, member, etc.).
4. **Platform vs organization roles.** Platform-level roles (admin, editor) have `organization_id: NULL`.

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition
- `openapi/org-mgmt/openapi.yaml` — Role/permission API

## Gaps / Drift

> **Open:** Verify actual Lifeguard model against OpenAPI spec.
