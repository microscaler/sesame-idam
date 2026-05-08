---
title: Permission Entity
status: partially-verified
updated: 2026-01-22
sources: [openapi/org-mgmt/openapi.yaml, openapi/authz-core/openapi.yaml]
---

# Entity: Permission

Owned by: **org-mgmt** (owned) / **authz-core** (evaluated)

## Description

Permission model. Permissions are per-application and referenced by roles via RolePermission table.

## Schema (from OpenAPI)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| tenant_id | uuid (FK) | |
| name | text | e.g., "invoices:write" |
| description | text | |
| created_at | timestamptz | |

## Key Design Decisions

1. **Named permissions.** Permission names follow `resource:action` convention (e.g., "invoices:write", "users:manage").
2. **Per-application.** Permissions are scoped to applications — different apps can have different permission sets.
3. **Coarse vs fine-grained.** Coarse checks use JWT claims directly (zero latency). Fine checks require authz-core `/authorize` endpoint with resource context.

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition (owned)
- `microservices/idam/authz-core/impl/src/` — Permission evaluation logic
- `openapi/authz-core/openapi.yaml` — Authorization API

## Gaps / Drift

> **Open:** Verify actual Lifeguard model against OpenAPI spec.
