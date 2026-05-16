---
title: Organization Entity
status: verified
updated: 2026-05-16
sources: [openapi/org-mgmt/openapi.yaml]
---

# Entity: Organization

Owned by: **org-mgmt** (consumed by api-keys for org data in validation)

## Description

Multi-tenant organization model. Organizations are scoped to a `tenant_id` so the same org name can exist in different tenants without conflict.

**Note:** The impl model is significantly simplified compared to the design spec. Most features from the design doc (SAML, SCIM, domain controls, password rotation, seat management) are NOT yet in the impl model.

## Schema (from impl/ crate — org-mgmt)

| Column | Type | Notes |
||--------|------|-------|
| id | uuid (PK) | |
| name | varchar(255) | Organization name |
| tenant_id | varchar(255) | **REQUIRED** — orgs belong to one tenant |
| status | varchar(32) | Active, suspended, etc. |
| created_at | timestamptz | |
| updated_at | timestamptz | |

## Key Design Decisions

1. **Per-tenant scoping.** The `tenant_id` column means orgs with the same name can exist across different tenants.
2. **Impl is simplified.** The actual impl model has only 6 columns (id, name, tenant_id, status, created_at, updated_at). Most design features (SAML, SCIM, domains, password rotation, seat limits, metadata) are NOT implemented yet.
3. **Single status field.** `status` (varchar(32)) replaces feature flags like `is_saml_configured`, `isolated`, etc.
4. **No soft delete.** `deleted_at` column does not exist in the impl.
5. **tenant_id is varchar(255), not uuid.** Type mismatch from wiki (wiki said uuid).

## New Features (from PropelAuth gap closure)

> **NOTE:** The API endpoints below reference endpoints from the OpenAPI spec, but most are NOT yet implemented against the simplified impl model. The impl org model lacks the data structures to support SAML, SCIM, domain controls, etc.

| Feature | Status |
||---------|--------|
| **SCIM User Provisioning** | NOT implemented — impl org model has no SCIM data fields |
| **Application RBAC** | Implemented as part of org-mgmt (roles/permissions are org-scoped) |
| **SAML SSO** | NOT implemented — no SAML fields in impl org model |
| **Domain Controls** | NOT implemented — no domain fields in impl org model |

## API Endpoints

> **NOTE:** Org CRUD endpoints (`/`, `/{org_id}` GET/PUT/DELETE) are implemented against the simplified impl model. Most other endpoints (SCIM, SAML, webhooks, domain controls, role mappings) reference the OpenAPI spec — implementations may use the impl model as-is or extend it.

| Endpoint | Method | Purpose |
| Service | Endpoint | Purpose |
|---------|----------|---------|
| org-mgmt | `GET /applications` | List applications |
| org-mgmt | `POST /applications` | Register application |
| org-mgmt | `GET /applications/{app_id}` | Get application by id |
| org-mgmt | `GET /applications/{app_id}/permissions` | List permissions for application |
| org-mgmt | `POST /applications/{app_id}/permissions` | Create permission for application |
| org-mgmt | `GET /applications/{app_id}/roles` | List roles for application |
| org-mgmt | `POST /applications/{app_id}/roles` | Create role for application |
| org-mgmt | `GET /applications/{app_id}/roles/{role_id}` | Get role by id |
| org-mgmt | `GET /applications/{app_id}/roles/{role_id}/permissions` | Get permissions for role |
| org-mgmt | `POST /applications/{app_id}/roles/{role_id}/permissions` | Assign permission to role |
| org-mgmt | `DELETE /applications/{app_id}/roles/{role_id}/permissions` | Revoke permission from role |
| org-mgmt | `GET /organizations` | Query for organisations |
| org-mgmt | `POST /organizations/admin/users/{user_id}/invalidate-all-keys` | Invalidate all API keys for user |
| org-mgmt | `GET /organizations/{org_id}` | Fetch organisation by ID |
| org-mgmt | `PUT /organizations/{org_id}` | Update organisation |
| org-mgmt | `DELETE /organizations/{org_id}` | Delete organisation |
| org-mgmt | `PUT /organizations/{org_id}/domains` | Update organisation domain settings |
| org-mgmt | `POST /organizations/{org_id}/invitations` | Invite user to organisation by email |
| org-mgmt | `POST /organizations/{org_id}/invitations/by-id` | Invite existing user to organisation |
| org-mgmt | `POST /organizations/{org_id}/migrate-to-isolated` | Migrate organisation to isolated SAML mode |
| org-mgmt | `POST /organizations/{org_id}/oidc-metadata` | Set OIDC IdP metadata for organisation |
| org-mgmt | `DELETE /organizations/{org_id}/pending-invitations` | Revoke pending organisation invite |
| org-mgmt | `GET /organizations/{org_id}/role-mappings` | Fetch custom role mappings for organisation |
| org-mgmt | `PUT /organizations/{org_id}/role-mappings/subscribe` | Subscribe organisation to a role mapping |
| org-mgmt | `GET /organizations/{org_id}/scim/groups` | Fetch SCIM groups for organisation |
| org-mgmt | `GET /organizations/{org_id}/scim/groups/{group_id}` | Fetch a specific SCIM group |
| org-mgmt | `GET /organizations/{org_id}/scim/users` | List SCIM users in org |
| org-mgmt | `POST /organizations/{org_id}/scim/users` | Create SCIM user in org |
| org-mgmt | `PUT /organizations/{org_id}/scim/users/{user_id}` | Update SCIM user in org |
| org-mgmt | `DELETE /organizations/{org_id}/scim/users/{user_id}` | Delete SCIM user from org |
| org-mgmt | `GET /organizations/{org_id}/users` | Fetch users in organisation |
| org-mgmt | `POST /organizations/{org_id}/users` | Add user to organisation |
| org-mgmt | `DELETE /organizations/{org_id}/users/{user_id}` | Remove user from organisation |
| org-mgmt | `PATCH /organizations/{org_id}/users/{user_id}/role` | Change user role in organisation |
| org-mgmt | `GET /organizations/{org_id}/webhooks` | Fetch organisation webhook subscriptions |
| org-mgmt | `DELETE /organizations/{org_id}/webhooks/{subscription_id}` | Delete webhook subscription |
| org-mgmt | `POST /organizations/{org_id}/webhooks/{subscription_id}/test` | Test webhook delivery |
| org-mgmt | `DELETE /sso/saml` | Delete SAML connection |
| org-mgmt | `POST /sso/saml/allow` | Allow organisation to set up SAML SSO |
| org-mgmt | `POST /sso/saml/disable` | Disallow organisation from using SAML SSO |
| org-mgmt | `POST /sso/saml/enable` | Enable SAML connection for organisation |
| org-mgmt | `POST /sso/saml/link` | Create SAML connection setup link |
| org-mgmt | `PUT /sso/saml/metadata` | Set SAML IdP metadata for organisation |

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition
- `openapi/org-mgmt/openapi.yaml` — Org CRUD API

## Drift Found (verified 2026-05-16)

| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|
| 30+ org columns (slug, logo_url, domains, SAML fields, etc.) | Only 6 columns: id, name, tenant_id, status, created_at, updated_at | Critical — impl is dramatically simplified |
| `slug`, `logo_url`, `domain`, `domains` | NOT in impl | High — domain-based features not supported |
| `domain_auto_join`, `domain_restrict` | NOT in impl | High — auto-join not implemented |
| `password_rotation_*` fields | NOT in impl | Medium |
| `max_users` seat management | NOT in impl | Medium |
| `metadata` jsonb | NOT in impl | Medium |
| `is_saml_*`, `can_setup_saml`, `isolated`, `sso_trust_level` | NOT in impl | High — SAML/isolation not implemented |
| `legacy_org_id` | NOT in impl | Low |
| `deleted_at` soft delete | NOT in impl | Medium |
| `status` column | EXISTS but was NOT in wiki | High — wiki was completely missing this field |
| `tenant_id` is uuid | `tenant_id` is varchar(255), not uuid | Low — type mismatch |
