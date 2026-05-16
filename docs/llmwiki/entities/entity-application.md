---
title: Application Entity
status: verified
updated: 2026-05-16
sources: [openapi/org-mgmt/openapi.yaml]
---

# Entity: Application

Owned by: **org-mgmt**

## Description

**Application = OIDC client within an organization.** Applications are org-scoped (linked to `org_id`). NOT a tenant boundary.

An application represents an OIDC client with a `client_id`, `client_secret`, and `redirect_uris`. Roles and permissions under an application are org-scoped (the Application entity links to orgs, not tenants).

## Schema (from impl/ crate — org-mgmt)

| Column | Type | Notes |
||--------|------|-------|
| id | uuid (PK) | |
| org_id | uuid (FK -> orgs) | Application is org-scoped |
| name | varchar(255) | Application name |
| client_id | varchar(64) | OIDC client identifier |
| client_secret | text (nullable) | OIDC client secret |
| redirect_uris | text (nullable) | Redirect URIs (stored as text, not array) |
| created_at | timestamptz | |
| updated_at | timestamptz | |

## Key Design Decisions

1. **Application is OIDC client, NOT tenant.** The Application entity links to `org_id` — it is org-scoped. The wiki previously claimed "Application = Tenant boundary" which is wrong.
2. **No lifecycle management.** No `is_active`, `slug`, `platform`, `deleted_at`, or `tenant_id` columns exist.
3. **OIDC configuration stored in DB.** `client_id`, `client_secret`, and `redirect_uris` are stored directly on the application record.
4. **`redirect_uris` is text.** Stored as a single text field, not as a proper array or JSON.

## API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/applications` | GET | List applications |
| `/applications` | POST | Register application |
| `/applications/{app_id}` | GET | Get application by id |
| `/applications/{app_id}/roles` | GET | List roles for application |
| `/applications/{app_id}/roles` | POST | Create role for application |
| `/applications/{app_id}/roles/{role_id}` | GET | Get role by id |
| `/applications/{app_id}/roles/{role_id}/permissions` | GET | Get permissions for role |
| `/applications/{app_id}/roles/{role_id}/permissions` | POST | Assign permission to role |
| `/applications/{app_id}/roles/{role_id}/permissions` | DELETE | Revoke permission from role |
| `/applications/{app_id}/permissions` | GET | List permissions for application |
| `/applications/{app_id}/permissions` | POST | Create permission for application |

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition
- `openapi/org-mgmt/openapi.yaml` — Application CRUD API

## Drift Found (verified 2026-05-16)

| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|
| Application = Tenant boundary | Application is org-scoped (`org_id` FK) | Critical — fundamental misunderstanding of entity scope |
| `slug`, `platform`, `is_active` columns | NOT in impl | High — lifecycle/status management missing |
| `tenant_id` column | NOT in impl — org-scoped only | High — tenant boundary claim was wrong |
| `deleted_at` soft delete | NOT in impl | Medium |
| `client_id`, `client_secret`, `redirect_uris` | EXISTS in impl but were MISSING from wiki | Critical — wiki missed OIDC-specific fields entirely |
| `created_at`/`updated_at` only | EXISTS in impl (correct) | Low — correct |
