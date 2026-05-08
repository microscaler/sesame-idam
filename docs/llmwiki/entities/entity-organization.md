---
title: Organization Entity
status: partially-verified
updated: 2026-01-22
sources: [openapi/org-mgmt/openapi.yaml]
---

# Entity: Organization

Owned by: **org-mgmt** (consumed by api-keys for org data in validation)

## Description

Multi-tenant organization model. Organizations are scoped per-platform (via `platform` column) so the same org name can exist in different applications without conflict.

Each org supports: SAML SSO, OIDC, SCIM user provisioning, webhooks, application/role/permission RBAC, and domain-based auto-join.

## Schema (from design-doc.md + OpenAPI)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| name | text | |
| slug | text (UK per tenant) | |
| logo_url | text (nullable) | |
| domain | text (nullable) | Single domain for auto-join |
| domains | text[] | Multiple domains |
| domain_auto_join | boolean | Auto-join on domain match |
| domain_restrict | boolean | Restrict signups to domain |
| password_rotation_enabled | boolean | Password rotation policy |
| password_rotation_history_size | integer | |
| password_rotation_period | integer | |
| max_users | integer (nullable) | NULL = unlimited seats |
| metadata | jsonb | Custom org metadata |
| is_saml_configured | boolean | |
| is_saml_in_test_mode | boolean | |
| can_setup_saml | boolean | |
| isolated | boolean | Org isolation flag |
| sso_trust_level | text | SSO trust level |
| legacy_org_id | text (nullable) | Migration from legacy system |
| tenant_id | uuid (FK) | **REQUIRED** — orgs belong to one consuming platform |
| created_at | timestamptz | |
| updated_at | timestamptz | |
| deleted_at | timestamptz | Soft delete |

## Key Design Decisions

1. **Per-platform scoping.** The `platform` column means orgs with the same name can exist across different applications.
2. **Seat management.** `max_users` is nullable — NULL means unlimited.
3. **Domain controls.** `domain_auto_join` and `domain_restrict` control email-based org access.
4. **SSO settings.** SAML configuration stored per-org in the same table.
5. **Org personas.** Three org types: platform (SaaS operator), provider (service deliverer), consumer (service recipient). `org_type` in JWT determines access rules.
6. **SCIM provisioning.** Full SCIM 2.0 user provisioning for enterprise SSO integration.

## New Features (from PropelAuth gap closure)

| Feature | Description |
|---------|-------------|
| **SCIM User Provisioning** | `POST/PUT/DELETE /{org_id}/scim/users/{id}` — Enterprise user provisioning |
| **API Key Invalidation** | `POST /admin/users/{user_id}/invalidate-all-keys` — Invalidate keys on block/delete |
| **Application RBAC** | Full application/role/permission management under `/{org_id}/applications` |

## API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/` | GET | List organizations |
| `/{org_id}` | GET | Get organization |
| `/{org_id}` | PUT | Update organization |
| `/{org_id}` | DELETE | Delete organization |
| `/{org_id}/users` | GET | List users in org |
| `/{org_id}/add-user` | POST | Add user to org |
| `/{org_id}/invite-user` | POST | Invite user by email |
| `/{org_id}/invite-user-by-id` | POST | Invite existing user |
| `/{org_id}/remove-user` | POST | Remove user from org |
| `/{org_id}/change-role` | POST | Change user role in org |
| `/{org_id}/role-mappings` | GET | Get role mappings |
| `/{org_id}/pending-invites` | DELETE | Revoke pending invites |
| `/{org_id}/subscribe-role-mapping` | PUT | Subscribe to role mapping |
| `/{org_id}/domains` | PUT | Update org domains |
| `/{org_id}/saml` | DELETE | Remove SAML config |
| `/{org_id}/saml-metadata` | PUT | Update SAML metadata |
| `/{org_id}/allow-saml` | POST | Enable SAML |
| `/{org_id}/disallow-saml` | POST | Disable SAML |
| `/{org_id}/enable-saml` | POST | Enable SAML SSO |
| `/{org_id}/create-saml-link` | POST | Create SAML link |
| `/{org_id}/oidc-metadata` | POST | Configure OIDC metadata |
| `/{org_id}/migrate-to-isolated` | POST | Migrate to isolated mode |
| `/{org_id}/scim/groups` | GET | List SCIM groups |
| `/{org_id}/scim/groups/{group_id}` | GET | Get SCIM group |
| `/{org_id}/scim/users` | GET | List SCIM users |
| `/{org_id}/scim/users` | POST | Create SCIM user |
| `/{org_id}/scim/users/{user_id}` | PUT | Update SCIM user |
| `/{org_id}/scim/users/{user_id}` | DELETE | Delete SCIM user |
| `/{org_id}/webhooks` | GET | List webhooks |
| `/{org_id}/webhooks/{subscription_id}` | DELETE | Delete webhook |
| `/{org_id}/webhooks/{subscription_id}/test` | POST | Test webhook |
| `/api/v1/am/applications` | GET | List applications |
| `/api/v1/am/applications` | POST | Create application |
| `/api/v1/am/applications/{app_id}` | GET | Get application |
| `/api/v1/am/applications/{app_id}/roles` | GET | List roles |
| `/api/v1/am/applications/{app_id}/roles` | POST | Create role |
| `/api/v1/am/applications/{app_id}/roles/{role_id}` | GET | Get role |
| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | GET | List role permissions |
| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | POST | Assign permission |
| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | DELETE | Revoke permission |
| `/api/v1/am/applications/{app_id}/permissions` | GET | List permissions |
| `/api/v1/am/applications/{app_id}/permissions` | POST | Create permission |
| `/admin/users/{user_id}/invalidate-all-keys` | POST | Invalidate all user API keys |

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition
- `openapi/org-mgmt/openapi.yaml` — Org CRUD API

## Gaps / Drift

> **Open:** Verify actual Lifeguard model. SCIM user endpoints and API key invalidation are newly added to specs.
