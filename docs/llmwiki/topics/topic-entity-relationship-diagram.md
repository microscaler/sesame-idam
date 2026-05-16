---
title: Entity Relationship Diagram
status: partially-verified
updated: 2026-05-16
sources: [openapi/idam/*/openapi.yaml, microservices/*/impl/src/models/*.rs]
---

# Entity Relationship Diagram

## Overview

This document presents the authoritative entity relationship diagram for Sesame-IDAM, reconciled from the OpenAPI specifications across all 6 microservices and the actual Lifeguard entity models in the `impl/` crates.

**Key changes from the previous ERD (topic-data-model.md):**

- **Added** `mfa_setup` entity (from identity-session-service + identity-user-mgmt-service)
- **Added** `social_credentials` entity (from identity-login-service)
- **Added** `otp_tokens` entity (from identity-login-service)
- **Added** `magic_link_tokens` entity (from identity-login-service)
- **Added** `impersonations` entity (from identity-session-service)
- **Added** `tokens` entity (from identity-session-service)
- **Added** `user_profiles` entity (from identity-session-service)
- **Added** `mcp_agents` entity (from identity-session-service)
- **Added** `role_assignments` entity (from authz-core)
- **Added** `principal_attributes` entity (from authz-core)
- **Added** `authorizations` entity (from authz-core)
- **Added** `audit_retention_policies` entity (from authz-core)
- **Added** `api_key_usage` entity (from api-keys)
- **Added** `archived_api_keys` entity (from api-keys)
- **Added** `email_verifications` entity (from identity-user-mgmt-service)
- **Added** `social_accounts` entity (from identity-user-mgmt-service)
- **Added** `employees` entity (from identity-user-mgmt-service)
- **Added** `org_domains` entity (from org-mgmt)
- **Added** `org_invites` entity (from org-mgmt)
- **Added** `org_memberships` entity (from org-mgmt) — was missing from original ERD
- **Added** `saml_connections` entity (from org-mgmt)
- **Added** `scim_users` entity (from org-mgmt)
- **Added** `webhook_subscriptions` entity (from org-mgmt)
- **Added** `applications` entity (from org-mgmt)
- **Added** `role_permissions` bridge table (from org-mgmt)
- **Removed** `tenant` entity — tenants are implicit via `tenant_id` column on all tables, there is no `tenants` table
- **Removed** `audit_log` as separate table — audit is distributed across `audit_events` (authz-core + identity-user-mgmt-service)

---

## Complete ERD

### Identity Layer (identity-login-service)

```
users (id PK, email, password_hash, tenant_id, email_verified, phone, phone_verified, status, created_at, updated_at)
  |
  |── 1:N ──> social_credentials (id PK, user_id FK, provider, provider_user_id, access_token, refresh_token)
  |── 1:N ──> otp_tokens (id PK, user_id FK, type_field, code, expires_at, attempts, max_attempts)
  |── 1:N ──> magic_link_tokens (id PK, user_id FK, link, expires_at, used)
  |── 1:N ──> sessions (id PK, user_id FK, token, refresh_token, expires_at, ip, user_agent)
  |
  |── 1:N ──> email_verifications (id PK, user_id FK, token, expires_at)
  |── 1:N ──> social_accounts (id PK, user_id FK, provider, provider_user_id, access_token, refresh_token)
  |── 1:N ──> employees (id PK, user_id FK, employee_id, department, title, manager_id FK->users)
  |── 1:N ──> user_profiles (id PK, user_id FK, first_name, last_name, avatar_url)
  |── 1:N ──> mfa_setup (id PK, user_id FK, factor_type, secret, enabled)
  |── 1:N ──> tokens (id PK, user_id FK, session_id FK->sessions, type_field, token)
```

### Session Layer (identity-session-service)

```
sessions (id PK, user_id FK, token, refresh_token, expires_at, ip, user_agent, mfa_verified, impersonated_by FK->users)
  |
  |── 1:N ──> impersonations (id PK, user_id FK, impersonator_id FK->users, session_id FK, created_at, restored_at)
  |── 1:N ──> tokens (id PK, user_id FK, session_id FK, type_field, token)
  |
  |── 1:N ──> mcp_agents (id PK, user_id FK, name, description, config)
```

### Access Management Layer (org-mgmt)

```
organizations (id PK, name, tenant_id, status)
  |
  |── 1:N ──> org_memberships (id PK, org_id FK, user_id FK->users, role, status)
  |── 1:N ──> org_invites (id PK, org_id FK, email, role, token, expires_at, created_at, accepted_at)
  |── 1:N ──> org_domains (id PK, org_id FK, domain, verified)
  |── 1:N ──> roles (id PK, org_id FK, name, description)
  |── 1:N ──> permissions (id PK, org_id FK, name, description, resource, action)
  |── 1:N ──> applications (id PK, org_id FK, name, client_id, client_secret, redirect_uris)
  |── 1:N ──> saml_connections (id PK, org_id FK, issuer, metadata_url, sso_url, signing_cert)
  |── 1:N ──> scim_users (id PK, org_id FK, external_id, username, email)
  |── 1:N ──> webhook_subscriptions (id PK, org_id FK, url, events, secret, active)
  |
  |── 1:N ──> roles
         |
         |── 1:N ──> role_permissions (id PK, role_id FK->roles, permission_id FK->permissions)
```

### Authorization Layer (authz-core)

```
users (id PK)
  |
  |── 1:N ──> role_assignments (id PK, principal_id FK->users, role_name, resource_type, resource_id FK->orgs, tenant_id)
  |── 1:N ──> principal_attributes (id PK, principal_id FK->users, key, value, tenant_id)
  |── 1:N ──> authorizations (id PK, principal_id FK->users, action, resource, effect, tenant_id)
  |── 1:N ──> audit_events (id PK, tenant_id, user_id FK->users, event_type, severity, actor, data, ip, created_at)
```

### API Key Layer (api-keys)

```
users (id PK) ──┐
                ├──> api_keys (id PK, key_hash, key_prefix, name, tenant_id, user_id FK->users, org_id FK->orgs, permissions, expires_at, active, created_at, updated_at)
orgs (id PK) ───┘
                |
                |── 1:N ──> api_key_usage (id PK, key_id FK->api_keys, endpoint, method, tenant_id, ip, created_at)
                |
                |── 1:N ──> archived_api_keys (id PK, key_hash, key_prefix, name, reason, archived_at)
```

---

## Full Diagram (ASCII)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             SESAME-IDAM ENTITIES                            │
└─────────────────────────────────────────────────────────────────────────────┘

┌──────────────────┐     1:N      ┌─────────────────────┐
│   USERS           │────────────>│ SOCIAL_CREDENTIALS   │ (identity-login)
│   id              │             │ id                   │
│   email           │             │ user_id FK->users    │
│   password_hash   │             │ provider             │
│   tenant_id       │             │ provider_user_id     │
│   email_verified  │             │ access_token         │
│   phone           │             │ refresh_token        │
│   phone_verified  │             │ created_at           │
│   status          │             └─────────────────────┘
│   created_at      │
│   updated_at      │             ┌─────────────────────┐
└────────┬─────────┘             │ OTP_TOKENS            │
         │ 1:N                   │ id                    │
         ├───────────────────────>│ user_id FK->users     │
         │                       │ type_field            │
         │                       │ code                  │
         │                       │ expires_at            │
         │                       │ attempts              │
         │                       │ max_attempts          │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ MAGIC_LINK_TOKENS     │
         │                       │ id                    │
         │                       │ user_id FK->users     │
         │                       │ link                  │
         │                       │ expires_at            │
         │                       │ used                  │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ SESSIONS              │
         │                       │ id                    │
         │                       │ user_id FK->users     │
         │                       │ token                 │
         │                       │ refresh_token         │
         │                       │ expires_at            │
         │                       │ ip                    │
         │                       │ user_agent            │
         │                       │ mfa_verified          │
         │                       │ impersonated_by FK->  │
         │                       │ created_at            │
         │                       └─────────────────────┘
         │                       │                       │
         │ 1:N                   │                       │
         ├───────────────────────>│ IMPERSONATIONS        │ (identity-session)
         │                       │ id                    │
         │                       │ user_id FK->users     │
         │                       │ impersonator_id FK->  │
         │                       │ session_id FK->       │
         │                       │ created_at            │
         │                       │ restored_at           │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ USER_PROFILES         │ (identity-session)
         │                       │ id                    │
         │                       │ user_id FK->users     │
         │                       │ first_name            │
         │                       │ last_name             │
         │                       │ avatar_url            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ TOKENS                │ (identity-session)
         │                       │ id                    │
         │                       │ user_id FK->users     │
         │                       │ session_id FK->       │
         │                       │ type_field            │
         │                       │ token                 │
         │                       │ expires_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ MCP_AGENTS            │ (identity-session)
         │                       │ id                    │
         │                       │ user_id FK->users     │
         │                       │ name                  │
         │                       │ description           │
         │                       │ config                │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ EMAIL_VERIFICATIONS   │ (identity-user-mgmt)
         │                       │ id                    │
         │                       │ user_id FK->users     │
         │                       │ token                 │
         │                       │ expires_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ SOCIAL_ACCOUNTS       │ (identity-user-mgmt)
         │                       │ id                    │
         │                       │ user_id FK->users     │
         │                       │ provider              │
         │                       │ provider_user_id      │
         │                       │ access_token          │
         │                       │ refresh_token         │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ EMPLOYEES             │ (identity-user-mgmt)
         │                       │ id                    │
         │                       │ user_id FK->users     │
         │                       │ employee_id           │
         │                       │ department            │
         │                       │ title                 │
         │                       │ manager_id FK->users  │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ MFA_SETUP             │ (user-mgmt + session)
         │                       │ id                    │
         │                       │ user_id FK->users     │
         │                       │ factor_type           │
         │                       │ secret                │
         │                       │ enabled               │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ AUDIT_EVENTS          │ (authz-core + user-mgmt)
         │                       │ id                    │
         │                       │ tenant_id             │
         │                       │ user_id FK->users     │
         │                       │ event_type            │
         │                       │ severity              │
         │                       │ actor                 │
         │                       │ data                  │
         │                       │ ip                    │
         │                       │ created_at            │
         │                       └─────────────────────┘

┌──────────────────┐     1:N      ┌─────────────────────┐
│  ORGANIZATIONS    │────────────>│ ORG_MEMBERSHIPS       │ (org-mgmt)
│  id              │             │ id                    │
│  name            │             │ org_id FK->orgs       │
│  tenant_id       │             │ user_id FK->users     │
│  status          │             │ role                  │
└────────┬─────────┘             │ status                │
         │ 1:N                   └─────────────────────┘
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ ORG_INVITES           │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK->orgs       │
         │                       │ email                 │
         │                       │ role                  │
         │                       │ token                 │
         │                       │ expires_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ ORG_DOMAINS           │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK->orgs       │
         │                       │ domain                │
         │                       │ verified              │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ ROLES                 │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK->orgs       │
         │                       │ name                  │
         │                       │ description           │
         │                       └─────────────────────┘
         │                       │                       │
         │                       │ 1:N                 ┌─>│ ROLE_PERMISSIONS    │
         │                       │                     │  │ id                  │
         │                       │                     │  │ role_id FK->roles   │
         │                       │                     │  │ permission_id FK    │
         │                       │                     │  └───────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ PERMISSIONS           │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK->orgs       │
         │                       │ name                  │
         │                       │ description           │
         │                       │ resource              │
         │                       │ action                │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ APPLICATIONS          │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK->orgs       │
         │                       │ name                  │
         │                       │ client_id             │
         │                       │ client_secret         │
         │                       │ redirect_uris         │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ SAML_CONNECTIONS      │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK->orgs       │
         │                       │ issuer                │
         │                       │ metadata_url          │
         │                       │ sso_url               │
         │                       │ signing_cert          │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ SCIM_USERS            │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK->orgs       │
         │                       │ external_id           │
         │                       │ username              │
         │                       │ email                 │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ WEBHOOK_SUBSCRIPTIONS  │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK->orgs       │
         │                       │ url                   │
         │                       │ events                │
         │                       │ secret                │
         │                       │ active                │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ ROLE_ASSIGNMENTS      │ (authz-core)
         │                       │ id                    │
         │                       │ principal_id FK->users│
         │                       │ role_name             │
         │                       │ resource_type         │
         │                       │ resource_id FK->orgs  │
         │                       │ tenant_id             │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ PRINCIPAL_ATTRIBUTES  │ (authz-core)
         │                       │ id                    │
         │                       │ principal_id FK->users│
         │                       │ key                   │
         │                       │ value                 │
         │                       │ tenant_id             │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ AUTHORIZATIONS        │ (authz-core)
         │                       │ id                    │
         │                       │ principal_id FK->users│
         │                       │ action                │
         │                       │ resource              │
         │                       │ effect                │
         │                       │ tenant_id             │
         │                       └─────────────────────┘

┌──────────────────┐     1:N      ┌─────────────────────┐
│  API_KEYS         │────────────>│ API_KEY_USAGE         │ (api-keys)
│  id              │             │ id                    │
│  key_hash        │             │ key_id FK->api_keys   │
│  key_prefix      │             │ endpoint              │
│  name            │             │ method                │
│  tenant_id       │             │ tenant_id             │
│  user_id FK->users│            │ ip                    │
│  org_id FK->orgs │             │ created_at            │
│  permissions     │             └─────────────────────┘
│  expires_at      │
│  active          │             ┌─────────────────────┐
│  created_at      │             │ ARCHIVED_API_KEYS     │ (api-keys)
│  updated_at      │             │ id                    │
└──────────────────┘             │ key_hash              │
                                 │ key_prefix            │
                                 │ name                  │
                                 │ reason                │
                                 │ archived_at           │
                                 └─────────────────────┘
```

---

## Multi-Tenancy

All entities are partitioned by `tenant_id`:
- `users.tenant_id` — user belongs to one tenant
- `organizations.tenant_id` — org belongs to one tenant
- `api_keys.tenant_id` — key belongs to one tenant
- `role_assignments.tenant_id` — role assignment belongs to one tenant
- `principal_attributes.tenant_id` — attribute belongs to one tenant
- `authorizations.tenant_id` — authorization belongs to one tenant
- `api_key_usage.tenant_id` — usage log belongs to one tenant
- `audit_events.tenant_id` — audit event belongs to one tenant

**No `tenants` table exists.** Tenants are identified by the `X-Tenant-ID` header, and all data is partitioned by the `tenant_id` column on each table. A tenant is a logical boundary, not a database entity.

---

## Cross-Service Entity Ownership

| Entity | Primary Service | Shared By |
|--------|----------------|-----------|
| `users` | identity-login-service | identity-session-service, identity-user-mgmt-service |
| `sessions` | identity-login-service, identity-session-service | (duplicated per service) |
| `social_credentials` | identity-login-service | |
| `otp_tokens` | identity-login-service | |
| `magic_link_tokens` | identity-login-service | |
| `mfa_setup` | identity-user-mgmt-service, identity-session-service | (duplicated) |
| `user_profiles` | identity-session-service | |
| `tokens` | identity-session-service | |
| `impersonations` | identity-session-service | |
| `mcp_agents` | identity-session-service | |
| `email_verifications` | identity-user-mgmt-service | |
| `social_accounts` | identity-user-mgmt-service | |
| `employees` | identity-user-mgmt-service | |
| `organizations` | org-mgmt | |
| `org_memberships` | org-mgmt | |
| `org_invites` | org-mgmt | |
| `org_domains` | org-mgmt | |
| `roles` | org-mgmt | |
| `permissions` | org-mgmt | |
| `role_permissions` | org-mgmt | |
| `applications` | org-mgmt | |
| `saml_connections` | org-mgmt | |
| `scim_users` | org-mgmt | |
| `webhook_subscriptions` | org-mgmt | |
| `role_assignments` | authz-core | |
| `principal_attributes` | authz-core | |
| `authorizations` | authz-core | |
| `audit_events` | authz-core, identity-user-mgmt-service | |
| `audit_retention_policies` | authz-core | |
| `api_keys` | api-keys | |
| `api_key_usage` | api-keys | |
| `archived_api_keys` | api-keys | |

---

## Key Design Decisions

1. **No `tenants` table.** Tenants are identified by `tenant_id` column on every table. The `X-Tenant-ID` header maps to this column. This avoids a separate table and makes tenant isolation purely column-based.
2. **Sessions duplicated across services.** Both `identity-login-service` and `identity-session-service` have their own `sessions` table (same schema). This is intentional — each service manages its own session lifecycle.
3. **MFA duplicated.** `mfa_setup` exists in both `identity-session-service` and `identity-user-mgmt-service` with identical schema.
4. **Soft deletes via status.** Users and orgs use a `status` column (active/disabled/deleted) rather than `deleted_at`. Some entity wiki pages reference `deleted_at` which is outdated.
5. **Org-centric RBAC.** Roles, permissions, and role assignments are scoped to organizations, not applications. `applications` are linked to orgs.
6. **Two audit event tables.** `audit_events` exists in both `authz-core` and `identity-user-mgmt-service` with slightly different schemas.

---

## Gaps / Drift

> **Open:** The `users` entity wiki page references fields like `has_password`, `username`, `first_name`, `last_name`, `picture_url`, `extra_properties` that do NOT exist in the actual Lifeguard `User` model. These fields are in the OpenAPI request/response schemas but the impl model is simplified. The wiki pages need updating to reflect the actual database model.

> **Open:** `tenant` entity page should be removed or replaced with a cross-cutting concept page, since there is no `tenants` table.

> **Open:** The `role` entity wiki page references `parent_role_id` (self-referential inheritance) and `is_system` flag — these do NOT exist in the actual `Role` model. The impl `Role` has only `id, org_id, name, description`.

> **Open:** The `permission` entity wiki page references a simpler schema than the actual `Permission` model, which has additional `resource` and `action` columns.

> **Open:** `api_key` wiki page references `metadata` field which is NOT in the impl model.

> **Open:** `organization` wiki page references many fields (slug, logo_url, domain, domains, domain_auto_join, domain_restrict, password_rotation_*, metadata, is_saml_*, isolated, sso_trust_level, legacy_org_id) that do NOT exist in the actual `Org` model. The impl `Org` only has `id, name, tenant_id, status`.

> **Open:** `session` wiki page references `tenant_id`, `revoked`, `last_used_at`, `step_up_verified`, `step_up_verified_at` that are NOT in the actual impl models. The actual sessions have `mfa_verified` and `impersonated_by` instead.

> **Open:** `mfa_device` entity wiki page references `label` field which is NOT in the actual impl model. The actual MFA entity is named `mfa_setup` with `enabled` instead of `is_active`.
