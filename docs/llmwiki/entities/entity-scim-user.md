---
title: SCIM User Entity
status: verified
updated: 2026-05-17
sources: [microservices/idam/org-mgmt/impl/src/models/scim_user.rs]
---

# Entity: SCIM User

Owned by: **org-mgmt**

## Description

Represents users provisioned into an organization via the SCIM 2.0 protocol. Each SCIM user record is scoped to an organization and stores the external ID from the identity provider (IdP), a username, and an email address. This entity is used by the SCIM endpoints to sync user identities from external IdPs (e.g., Okta, Azure AD) into the organization's user base. The model is intentionally minimal — it does not store SCIM groups, meta attributes, or user attributes beyond the core identity fields.

## Schema (from impl/ crate — org-mgmt)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| org_id | uuid (FK) | FK → `sesame_idam.organizations(id) ON DELETE CASCADE` |
| external_id | varchar(255) | IdP-assigned user identifier |
| username | varchar(255) | Username for the provisioned user |
| email | varchar(255) | Email address |
| created_at | timestamptz | Record creation time |
| updated_at | timestamptz | Last update time |

## Key Design Decisions

1. **Minimal SCIM model.** Only stores `external_id`, `username`, and `email` — SCIM 2.0 supports rich `schemas`, `meta`, `name`, `addresses`, `phones`, `emails` arrays, and custom attributes, but the impl captures only the essential identity fields.
2. **External ID as the IdP reference.** The `external_id` column stores the IdP's opaque user identifier, enabling SCIM update and delete operations to match users by external ID rather than by internal uuid.
3. **No direct user linkage.** This table does NOT reference the `users` table. SCIM users are provisioned identities that may or may not correspond to an existing `users` record. The actual linking to user accounts (if any) is handled at the application layer.
4. **Cascade delete on org deletion.** When an organization is deleted, all its SCIM user records are automatically removed.

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/scim_user.rs` — Lifeguard entity definition
- `microservices/idam/org-mgmt/impl/src/controllers/scim_list_users.rs` — List SCIM users
- `microservices/idam/org-mgmt/impl/src/controllers/scim_create_user.rs` — Create SCIM user
- `microservices/idam/org-mgmt/impl/src/controllers/scim_update_user.rs` — Update SCIM user
- `microservices/idam/org-mgmt/impl/src/controllers/scim_delete_user.rs` — Delete SCIM user

## Gaps / Drift

> None — this entity was just created and verified against the impl model on 2026-05-17.
