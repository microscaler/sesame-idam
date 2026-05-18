---
title: Permission Entity
status: verified
updated: 2026-05-16
sources: [openapi/org-mgmt/openapi.yaml, openapi/authz-core/openapi.yaml]
---

# Entity: Permission

Owned by: **org-mgmt** (owned) / **authz-core** (evaluated)

## Description

Permission model. Permissions are per-application and referenced by roles via RolePermission table.

## Schema (from impl/ crate — org-mgmt)

|| Column | Type | Notes |
||--------|------|-------|
|| id | uuid (PK) | |
|| org_id | uuid (FK -> orgs) | Permission is org-scoped |
|| name | varchar(255) | e.g., "invoices:write" |
|| description | text (nullable) | |
|| resource | varchar(255) | Resource identifier |
|| action | varchar(255) | Action identifier |
|| created_at | timestamptz | |
|| updated_at | timestamptz | |

## Key Design Decisions

1. **Org-scoped permissions.** Permissions belong to an org (`org_id`), not a tenant or application. Named permissions follow `resource:action` convention.
2. **Separate resource/action columns.** The impl stores `resource` and `action` as separate varchar columns, not parsed from the name.
3. **Coarse vs fine-grained.** Coarse checks use JWT claims directly (zero latency). Fine checks require authz-core `/authorize` endpoint with resource context.
4. **No tenant boundary.** Permissions are org-scoped, not tenant-scoped.

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition (owned)
- `microservices/idam/authz-core/impl/src/` — Permission evaluation logic
- `openapi/authz-core/openapi.yaml` — Authorization API

## Drift Found (verified 2026-05-16)

| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|
| Permissions are per-application | Permissions are org-scoped (`org_id` FK) | High — scoping was wrong |
| `tenant_id` column | NOT in impl — org-scoped | High — wiki had wrong scope |
| `resource` column | EXISTS in impl but was MISSING from wiki | High — wiki missed 2 important columns |
| `action` column | EXISTS in impl but was MISSING from wiki | High — wiki missed 2 important columns |
| `description` is text (required) | `description` is text (nullable) | Low |
| No `updated_at` in wiki | EXISTS in impl | Low |
