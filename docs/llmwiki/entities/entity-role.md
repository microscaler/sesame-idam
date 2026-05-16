---
title: Role Entity
status: verified
updated: 2026-05-16
sources: [openapi/org-mgmt/openapi.yaml]
---

# Entity: Role

Owned by: **org-mgmt** (evaluated by authz-core for permission checks)

## Description

Role model. Roles are scoped to organizations (org-scoped). No inheritance or hierarchy support.

**Note:** The wiki previously documented `parent_role_id` inheritance, `tenant_id` scoping, `display_name`, and `is_system` — none of these exist in the impl model.

## Schema (from impl/ crate — org-mgmt)

|| Column | Type | Notes |
||--------|------|-------|
|| id | uuid (PK) | |
|| org_id | uuid (FK -> orgs) | Role is org-scoped |
|| name | varchar(255) | Internal name |
|| description | text (nullable) | |
|| created_at | timestamptz | |
|| updated_at | timestamptz | |

## RolePermission Association

|| Column | Type | Notes |
||--------|------|-------|
|| role_id | uuid (FK -> roles, PK composite) | |
|| permission_id | uuid (FK -> permissions, PK composite) | |

## Key Design Decisions

1. **Org-scoped roles.** Every role has a non-null `org_id`. There is no concept of platform-level roles (no `organization_id: NULL`).
2. **No inheritance.** The `parent_role_id` column does NOT exist in the impl. Roles are flat — no hierarchy.
3. **No system roles.** The `is_system` flag does NOT exist. All roles are user-managed.
4. **Simple name field.** Only `name` (varchar(255)) — no separate `display_name`.
5. **Role-permission is many-to-many.** Resolved via RolePermission association table.

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition
- `openapi/org-mgmt/openapi.yaml` — Role/permission API

## Drift Found (verified 2026-05-16)

| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|
| Role inheritance via `parent_role_id` | Column does NOT exist — roles are flat, no hierarchy | Critical — inheritance feature is missing |
| `tenant_id` column | NOT in impl — roles are org-scoped only (`org_id`) | High — wiki got scoping wrong |
| `organization_id: NULL` for platform roles | NOT in impl — `org_id` is non-null FK | High — platform roles don't exist |
| `display_name` column | NOT in impl (only `name`) | Medium |
| `is_system` column | NOT in impl (all roles user-managed) | Medium |
| `description` is text (required) | `description` is text (nullable) | Low — nullable vs required |
