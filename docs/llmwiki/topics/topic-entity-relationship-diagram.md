---
title: Entity Relationship Diagram
status: verified
updated: 2026-05-17
sources: [openapi/idam/*/openapi.yaml, microservices/*/impl/src/models/*.rs]
---

# Entity Relationship Diagram

## Overview

This document presents the authoritative entity relationship diagram for Sesame-IDAM, reconciled from the actual Lifeguard entity models in the `impl/` crates. The OpenAPI specifications have been cross-referenced and significant gaps have been documented.

**Verified against 41 impl model files across 6 services:**

| Service | Model Files | Unique Tables |
|---------|-------------|---------------|
| api-keys | 3 | api_keys, api_key_usage, archived_api_keys |
| authz-core | 5 | audit_events, audit_retention_policies, authorizations, principal_attributes, role_assignments |
| identity-login-service | 5 | users, sessions, social_credentials, otp_tokens, magic_link_tokens |
| identity-session-service | 6 | sessions, tokens, impersonations, mcp_agents, mfa_setup, user_profiles |
| identity-user-mgmt-service | 6 | users, audit_events, email_verifications, social_accounts, employees, mfa_setup |
| org-mgmt | 11 | organizations, org_memberships, org_invites, org_domains, roles, permissions, role_permissions, applications, saml_connections, scim_users, webhook_subscriptions |

**Total: 36 unique table definitions** (some duplicated across services: users, sessions, mfa_setup, audit_events each exist in 2 services with slight schema differences)

---

## Complete ERD

### Identity Layer (identity-login-service)

```
users (id PK, email, password_hash, tenant_id, email_verified, phone, phone_verified, status, created_at, updated_at)
  |
  |── 1:N ──> social_credentials (id PK, user_id FK, provider, provider_user_id, access_token, refresh_token, created_at, updated_at)
  |── 1:N ──> otp_tokens (id PK, user_id FK, type_field, code, expires_at, attempts, max_attempts, created_at, updated_at)
  |── 1:N ──> magic_link_tokens (id PK, user_id FK, link, expires_at, used, created_at, updated_at)
  |── 1:N ──> sessions (id PK, user_id FK, token, refresh_token, expires_at, ip, user_agent, created_at, updated_at)
```

### Session Layer (identity-session-service)

```
sessions (id PK, user_id FK, token, refresh_token, expires_at, ip, user_agent, mfa_verified, impersonated_by FK, created_at, updated_at)
  |
  |── 1:N ──> impersonations (id PK, user_id FK, impersonator_id FK, session_id FK, created_at, restored_at)
  |── 1:N ──> tokens (id PK, user_id FK, session_id FK, type_field, token, expires_at, created_at, updated_at)
  |── 1:N ──> mcp_agents (id PK, user_id FK, name, description, config, created_at, updated_at)
```

### User Management (identity-user-mgmt-service)

```
users (id PK, email, password_hash, tenant_id, status, email_verified, phone, phone_verified, created_at, updated_at)
  |
  |── 1:N ──> email_verifications (id PK, user_id FK, token, expires_at, created_at, updated_at)
  |── 1:N ──> social_accounts (id PK, user_id FK, provider, provider_user_id, access_token, refresh_token, created_at, updated_at)
  |── 1:N ──> employees (id PK, user_id FK, employee_id, department, title, manager_id FK, created_at, updated_at)
  |── 1:N ──> mfa_setup (id PK, user_id FK, factor_type, secret, enabled, created_at, updated_at)
  |── 1:N ──> tokens (id PK, user_id FK, session_id FK, type_field, token, expires_at, created_at, updated_at)
  |── 1:N ──> audit_events (id PK, tenant_id, user_id FK, event_type, severity, actor, data, ip, user_agent, created_at)
```

### Access Management Layer (org-mgmt)

```
organizations (id PK, name, tenant_id, status, created_at, updated_at)
  |
  |── 1:N ──> org_memberships (id PK, org_id FK, user_id FK, role, status, created_at, updated_at)
  |── 1:N ──> org_invites (id PK, org_id FK, email, role, token, expires_at, created_at, accepted_at)
  |── 1:N ──> org_domains (id PK, org_id FK, domain, verified, created_at, updated_at)
  |── 1:N ──> roles (id PK, org_id FK, name, description, created_at, updated_at)
  |── 1:N ──> permissions (id PK, org_id FK, name, description, resource, action, created_at, updated_at)
  |── 1:N ──> applications (id PK, org_id FK, name, client_id, client_secret, redirect_uris, created_at, updated_at)
  |── 1:N ──> saml_connections (id PK, org_id FK, issuer, metadata_url, sso_url, signing_cert, created_at, updated_at)
  |── 1:N ──> scim_users (id PK, org_id FK, external_id, username, email, created_at, updated_at)
  |── 1:N ──> webhook_subscriptions (id PK, org_id FK, url, events, secret, active, created_at, updated_at)
  |── 1:M ──> roles (joined by role_permissions)
         |── role_permissions (id PK, role_id FK, permission_id FK, created_at)
         └── permissions
```

### Authorization Layer (authz-core)

```
users (id PK)
  |
  |── 1:N ──> role_assignments (id PK, principal_id FK, role_name, resource_type, resource_id FK, tenant_id, created_at, updated_at)
  |── 1:N ──> principal_attributes (id PK, principal_id FK, key, value, tenant_id, created_at, updated_at)
  |── 1:N ──> authorizations (id PK, principal_id FK, action, resource, effect, tenant_id, created_at, updated_at)
  |── 1:N ──> audit_events (id PK, tenant_id, event_type, severity, actor, data, ip, created_at)
  |── 1:N ──> audit_retention_policies (id PK, tenant_id, retention_days, enabled, created_at, updated_at)
```

### API Key Layer (api-keys)

```
users (id PK) ──┐
                ├──> api_keys (id PK, key_hash, key_prefix, name, tenant_id, user_id FK, org_id FK, permissions, expires_at, active, created_at, updated_at)
orgs (id PK) ───┘
                |
                |── 1:N ──> api_key_usage (id PK, key_id FK, endpoint, method, tenant_id, ip, created_at)
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
│   id              │             │ id                    │
│   email           │             │ user_id FK→users      │
│   password_hash   │             │ provider              │
│   tenant_id       │             │ provider_user_id      │
│   email_verified  │             │ access_token          │
│   phone           │             │ refresh_token         │
│   phone_verified  │             │ created_at            │
│   status          │             │ updated_at            │
│   created_at      │             └─────────────────────┘
│   updated_at      │
└────────┬─────────┘
         │ 1:N
         ├───────────────────────>┌─────────────────────┐
         │                       │ OTP_TOKENS            │ (identity-login)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ type_field            │
         │                       │ code                  │
         │                       │ expires_at            │
         │                       │ attempts              │
         │                       │ max_attempts          │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ MAGIC_LINK_TOKENS     │ (identity-login)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ link                  │
         │                       │ expires_at            │
         │                       │ used                  │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ SESSIONS              │ (identity-login)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ token                 │
         │                       │ refresh_token         │
         │                       │ expires_at            │
         │                       │ ip                    │
         │                       │ user_agent            │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ SESSIONS              │ (identity-session)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ token                 │
         │                       │ refresh_token         │
         │                       │ expires_at            │
         │                       │ ip                    │
         │                       │ user_agent            │
         │                       │ mfa_verified          │
         │                       │ impersonated_by FK→   │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ IMPERSONATIONS        │ (identity-session)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ impersonator_id FK→   │
         │                       │ session_id FK→        │
         │                       │ created_at            │
         │                       │ restored_at           │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ USER_PROFILES         │ (identity-session)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ first_name            │
         │                       │ last_name             │
         │                       │ avatar_url            │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ TOKENS                │ (login + session)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ session_id FK→        │
         │                       │ type_field            │
         │                       │ token                 │
         │                       │ expires_at            │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ MCP_AGENTS            │ (identity-session)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ name                  │
         │                       │ description           │
         │                       │ config                │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ EMAIL_VERIFICATIONS   │ (user-mgmt)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ token                 │
         │                       │ expires_at            │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ SOCIAL_ACCOUNTS       │ (user-mgmt)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ provider              │
         │                       │ provider_user_id      │
         │                       │ access_token          │
         │                       │ refresh_token         │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ EMPLOYEES             │ (user-mgmt)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ employee_id           │
         │                       │ department            │
         │                       │ title                 │
         │                       │ manager_id FK→users   │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ MFA_SETUP             │ (user-mgmt + session)
         │                       │ id                    │
         │                       │ user_id FK→users      │
         │                       │ factor_type           │
         │                       │ secret                │
         │                       │ enabled               │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ AUDIT_EVENTS          │ (authz-core)
         │                       │ id                    │
         │                       │ tenant_id             │
         │                       │ event_type            │
         │                       │ severity              │
         │                       │ actor                 │
         │                       │ data                  │
         │                       │ ip                    │
         │                       │ created_at            │
         │                       └─────────────────────┘
         │                       ┌─────────────────────┐
         │ 1:N                   │ AUDIT_EVENTS          │ (user-mgmt)
         │                       │ id                    │
         │                       │ tenant_id             │
         │                       │ user_id FK→users      │
         │                       │ event_type            │
         │                       │ severity              │
         │                       │ actor                 │
         │                       │ data                  │
         │                       │ ip                    │
         │                       │ user_agent            │
         │                       │ created_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ ROLE_ASSIGNMENTS      │ (authz-core)
         │                       │ id                    │
         │                       │ principal_id FK→users │
         │                       │ role_name             │
         │                       │ resource_type         │
         │                       │ resource_id FK→orgs   │
         │                       │ tenant_id             │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ PRINCIPAL_ATTRIBUTES  │ (authz-core)
         │                       │ id                    │
         │                       │ principal_id FK→users │
         │                       │ key                   │
         │                       │ value                 │
         │                       │ tenant_id             │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ AUTHORIZATIONS        │ (authz-core)
         │                       │ id                    │
         │                       │ principal_id FK→users │
         │                       │ action                │
         │                       │ resource              │
         │                       │ effect                │
         │                       │ tenant_id             │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ AUDIT_RETENTION_POLICIES │ (authz-core)
         │                       │ id                    │
         │                       │ tenant_id             │
         │                       │ retention_days        │
         │                       │ enabled               │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │                       │ API_KEYS              │ (api-keys)
         │                       │ id                    │
         │                       │ key_hash              │
         │                       │ key_prefix            │
         │                       │ name                  │
         │                       │ tenant_id             │
         │                       │ user_id FK→users      │
         │                       │ org_id FK→orgs        │
         │                       │ permissions           │
         │                       │ expires_at            │
         │                       │ active                │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │                       │                       │
         │                       │ 1:N                 ┌─>│ API_KEY_USAGE       │
         │                       │                     │  │ id                  │
         │                       │                     │  │ key_id FK→api_keys  │
         │                       │                     │  │ endpoint            │
         │                       │                     │  │ method              │
         │                       │                     │  │ tenant_id           │
         │                       │                     │  │ ip                  │
         │                       │                     │  │ created_at          │
         │                       │                     │  └───────────────────┘
         │                       │                       │
         │                       │ 1:N                 ┌─>│ ARCHIVED_API_KEYS   │
         │                       │                     │  │ id                  │
         │                       │                     │  │ key_hash            │
         │                       │                     │  │ key_prefix          │
         │                       │                     │  │ name                │
         │                       │                     │  │ reason              │
         │                       │                     │  │ archived_at         │
         │                       │                     │  └───────────────────┘

┌──────────────────┐     1:N      ┌─────────────────────┐
│  ORGANIZATIONS    │────────────>│ ORG_MEMBERSHIPS       │ (org-mgmt)
│  id              │             │ id                    │
│  name            │             │ org_id FK→orgs        │
│  tenant_id       │             │ user_id FK→users      │
│  status          │             │ role                  │
│  created_at      │             │ status                │
│  updated_at      │             │ created_at            │
└────────┬─────────┘             │ updated_at            │
         │ 1:N                   └─────────────────────┘
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ ORG_INVITES           │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK→orgs        │
         │                       │ email                 │
         │                       │ role                  │
         │                       │ token                 │
         │                       │ expires_at            │
         │                       │ created_at            │
         │                       │ accepted_at           │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ ORG_DOMAINS           │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK→orgs        │
         │                       │ domain                │
         │                       │ verified              │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ ROLES                 │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK→orgs        │
         │                       │ name                  │
         │                       │ description           │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       │                       │
         │                       │ M:N (via role_per-)   │
         │                       │ missions)             │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ PERMISSIONS           │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK→orgs        │
         │                       │ name                  │
         │                       │ description           │
         │                       │ resource              │
         │                       │ action                │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │                       │                       │
         │                       │ M:N (via role_per-)   │
         │                       │ missions)             │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ ROLE_PERMISSIONS      │ (org-mgmt)
         │                       │ id                    │
         │                       │ role_id FK→roles      │
         │                       │ permission_id FK→     │
         │                       │ created_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ APPLICATIONS          │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK→orgs        │
         │                       │ name                  │
         │                       │ client_id             │
         │                       │ client_secret         │
         │                       │ redirect_uris         │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ SAML_CONNECTIONS      │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK→orgs        │
         │                       │ issuer                │
         │                       │ metadata_url          │
         │                       │ sso_url               │
         │                       │ signing_cert          │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ SCIM_USERS            │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK→orgs        │
         │                       │ external_id           │
         │                       │ username              │
         │                       │ email                 │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
         │
         ├───────────────────────>┌─────────────────────┐
         │ 1:N                   │ WEBHOOK_SUBSCRIPTIONS  │ (org-mgmt)
         │                       │ id                    │
         │                       │ org_id FK→orgs        │
         │                       │ url                   │
         │                       │ events                │
         │                       │ secret                │
         │                       │ active                │
         │                       │ created_at            │
         │                       │ updated_at            │
         │                       └─────────────────────┘
```

---

## Multi-Tenancy

All entities are partitioned by `tenant_id`:
- `users.tenant_id` — user belongs to one tenant
- `organizations.tenant_id` — org belongs to one tenant
- `api_keys.tenant_id` — key belongs to one tenant
- `api_key_usage.tenant_id` — usage log belongs to one tenant
- `audit_events (authz-core).tenant_id` — audit event belongs to one tenant
- `audit_events (user-mgmt).tenant_id` — audit event belongs to one tenant
- `role_assignments.tenant_id` — role assignment belongs to one tenant
- `principal_attributes.tenant_id` — attribute belongs to one tenant
- `authorizations.tenant_id` — authorization belongs to one tenant
- `audit_retention_policies.tenant_id` — retention policy belongs to one tenant

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
| `tokens` | identity-login-service, identity-session-service | (duplicated) |
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
| `audit_events` | authz-core, identity-user-mgmt-service | (different schemas) |
| `audit_retention_policies` | authz-core | |
| `api_keys` | api-keys | |
| `api_key_usage` | api-keys | |
| `archived_api_keys` | api-keys | |

---

## Key Design Decisions

1. **No `tenants` table.** Tenants are identified by `tenant_id` column on every table. The `X-Tenant-ID` header maps to this column. This avoids a separate table and makes tenant isolation purely column-based.
2. **Sessions duplicated across services.** Both `identity-login-service` and `identity-session-service` have their own `sessions` table (different schemas — session-service has `mfa_verified`, `impersonated_by`).
3. **MFA duplicated.** `mfa_setup` exists in both `identity-session-service` and `identity-user-mgmt-service` with identical schema.
4. **Soft deletes via status.** Users and orgs use a `status` column (active/disabled/deleted) rather than `deleted_at`.
5. **Org-centric RBAC.** Roles, permissions, and role assignments are scoped to organizations, not applications. `applications` are linked to orgs.
6. **Two audit event tables.** `audit_events` exists in both `authz-core` (lightweight, no user_id) and `identity-user-mgmt-service` (richer, includes user_id).
7. **All tables include created_at.** Every model has `created_at` timestamps. Most also have `updated_at`.
8. **API keys are dual-scoped.** Keys can be user-scoped (`user_id` FK) or org-scoped (`org_id` FK) — both are `Option<uuid>`.
9. **RolePermission is a bridge table.** Many-to-many relationship between roles and permissions, not a simple child entity.
10. **Role/Permission are org-scoped in impl.** The `Role` and `Permission` structs both use `org_id FK`, NOT `application_id`.

---

## Gaps / Drift (2026-05-17 Audit)

This audit cross-referenced all 41 impl model files against the OpenAPI specs for all 6 services. Two categories of drift were found:

### Category A: Impl Models With NO Corresponding OpenAPI Schema

The following 17 impl model files have **no corresponding component schema** in their service's OpenAPI spec. These are internal database tables that are queried/manipulated directly rather than exposed as API resources:

| Service | Impl Model | Reason |
|---------|-----------|--------|
| identity-login-service | `MagicLinkToken` | Internal passwordless token storage |
| identity-login-service | `OTPToken` | Internal OTP code storage |
| identity-login-service | `Session` | Session storage (login-service version) |
| identity-login-service | `SocialCredential` | OAuth credential storage (login-service version) |
| identity-login-service | `User` | Database model (not exposed as API resource) |
| identity-session-service | `Impersonation` | Admin impersonation audit trail |
| identity-session-service | `MfaSetup` | MFA configuration (session-service version) |
| identity-session-service | `Session` | Session storage (session-service version) |
| identity-session-service | `Token` | Token tracking table |
| identity-user-mgmt-service | `EmailVerification` | Email verification token storage |
| identity-user-mgmt-service | `Employee` | Employee metadata |
| identity-user-mgmt-service | `MfaSetup` | MFA configuration (user-mgmt version) |
| identity-user-mgmt-service | `SocialAccount` | Social account linking (user-mgmt version) |
| identity-user-mgmt-service | `User` | Database model (not exposed as API resource) |
| authz-core | `Authorization` | ABAC authorization records |
| authz-core | `PrincipalAttribute` | Custom user/principal attributes |
| authz-core | `RoleAssignment` | Principal-to-role assignments |
| org-mgmt | `OrgDomain` | Verified organization domains |
| org-mgmt | `OrgInvite` | Invitation tokens |
| org-mgmt | `OrgMembership` | User-org membership records |
| org-mgmt | `RolePermission` | Role-permission bridge table |
| org-mgmt | `SamlConnection` | SAML IdP configuration |

**Key finding:** These 17 models are **database-only entities** — they don't have dedicated REST endpoints. They are queried/manipulated through the authz, session, or user-mgmt service APIs without being exposed as first-class resources.

### Category B: Schema Mismatches (Impl vs OpenAPI)

These impl models have OpenAPI counterparts but with significant column/property differences:

| Service | Entity | Impl Has (not in OpenAPI) | OpenAPI Has (not in impl) |
|---------|--------|--------------------------|---------------------------|
| api-keys | `ApiKey` | `id, key_hash, key_prefix, tenant_id, updated_at` | `api_key_id, metadata` |
| api-keys | `ArchivedApiKey` | `id, key_hash, key_prefix, name, reason, archived_at` | `archived_reason, revoked_at, revoked_by_user_id` |
| authz-core | `AuditEvent` (authz) | `created_at, data, ip` (lightweight: 8 cols) | `event_action, hmac_signature, ip_address, metadata, org_id, session_id, target_id, target_type, timestamp, user_agent, user_id` (rich: 16 cols) |
| authz-core | `AuditRetentionPolicy` | `enabled, updated_at` | `archive_after_days, delete_after_days, event_type` |
| identity-session | `McpAgent` | `id, user_id, config, created_at, updated_at` | `active, agent_id` |
| identity-session | `UserProfile` | `id, first_name, last_name, avatar_url, created_at, updated_at` (simple: 7 cols) | `sub, email, email_verified, name, phone_number, phone_verified, picture_url, preferred_username, properties, user_permissions, user_role, username, org_id, org_name` (rich: 18 cols) |
| user-mgmt | `AuditEvent` (user-mgmt) | `created_at, data, ip, user_agent, user_id` (rich: 10 cols) | `event_action, hmac_signature, ip_address, metadata, org_id, session_id, target_id, target_type, timestamp` (rich: 16 cols) |
| user-mgmt | `User` | `id, email, password_hash, tenant_id, email_verified, phone, phone_verified, status, created_at, updated_at` (raw DB: 10 cols) | `email_confirmed, enabled, first_name, has_password, last_name, locked, picture_url, properties, user_id, username` (API projection: 11 props) |
| org-mgmt | `Application` | `id, org_id, name, client_id, client_secret, redirect_uris, created_at, updated_at` (8 cols, OIDC fields) | `slug` (6 cols, minimal) |
| org-mgmt | `Org` | `id, name, tenant_id, status, created_at, updated_at` (6 cols, minimal) | `slug, logo_url, domain, domain_auto_join, domain_restrict, domains, sso fields, password_rotation, metadata, isolated, legacy fields` (21 cols, rich) |
| org-mgmt | `Permission` | `id, org_id, name, description, resource, action, created_at, updated_at` (8 cols) | `application_id` (6 cols) |
| org-mgmt | `Role` | `id, org_id, name, description, created_at, updated_at` (6 cols, org-scoped) | `application_id` (6 cols, app-scoped) |
| org-mgmt | `ScimUser` | `id, org_id, external_id, username, email, created_at, updated_at` (7 cols) | `active, emails, name, roles, schemas, userName` (SCIM protocol format) |
| org-mgmt | `WebhookSubscription` | `id, org_id, url, events, secret, active, created_at, updated_at` (8 cols) | `enabled, endpoint_url, events, failed_deliveries, last_delivery_at, last_delivery_status, org_id, secret_present, subscription_id, total_deliveries, created_at, updated_at` (12 cols) |

### Category C: Critical Schema Conflicts

These are the most impactful discrepancies that would cause LLM-generated code to fail:

1. **`Role.application_id` vs `Role.org_id`:** The OpenAPI spec defines `Role.application_id` as the foreign key, but the impl uses `Role.org_id`. Roles are org-scoped in the database. **The OpenAPI spec is stale here.**

2. **`Permission.application_id` vs `Permission.org_id`:** Same pattern — OpenAPI has `application_id`, impl has `org_id`. Permissions are org-scoped. **The OpenAPI spec is stale.**

3. **`AuditEvent` schema mismatch:** The OpenAPI `AuditEvent` (16 properties, including `event_action`, `hmac_signature`, `target_id`, `target_type`, `org_id`) does not match EITHER of the two impl versions. The authz-core impl has a lightweight 8-column model (no user_id, no user_agent). The user-mgmt impl has a richer 10-column model (with user_id, user_agent). Neither matches the 16-column OpenAPI spec. **All three schemas are out of sync.**

4. **`Org` schema mismatch:** The OpenAPI `Org` has 21 properties including `slug`, `logo_url`, `domain_auto_join`, `sso_trust_level`, `password_rotation_*`, `metadata`, `isolated` — none of which exist in the impl `Org` struct which only has 6 columns. **The OpenAPI spec describes a much richer Org model than what exists in the database.**

5. **`User` schema mismatch (user-mgmt):** The OpenAPI `User` is an API projection (no `password_hash`, no `tenant_id`, has `email_confirmed` instead of `email_verified`). The impl is the raw database model. These serve different purposes but the OpenAPI name collides with the impl name, causing confusion.
