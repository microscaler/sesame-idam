---
title: Data Model
status: verified
updated: 2026-05-16
sources: [openapi/idam/*/openapi.yaml, microservices/*/impl/src/models/, topic-entity-relationship-diagram.md]
---

# Data Model

## Overview

This data model reflects the ACTUAL Lifeguard entity models in the `impl/` crates, verified against all 6 OpenAPI specs on 2026-05-16. For the complete entity relationship diagram with all foreign keys, see [`topic-entity-relationship-diagram.md`](./topic-entity-relationship-diagram.md).

## All Tables (40+ across 6 services)

### Identity Layer (identity-login-service)

| Table | Service | Purpose |
|-------|---------|---------|
| `users` | identity-login-service, identity-user-mgmt-service | Registered users |
| `sessions` | identity-login-service, identity-session-service | Auth sessions (two models) |
| `social_credentials` | identity-login-service | OAuth provider linking |
| `otp_tokens` | identity-login-service | Email/phone OTP verification |
| `magic_link_tokens` | identity-login-service | Passwordless magic links |

### Session Layer (identity-session-service)

| Table | Service | Purpose |
|-------|---------|---------|
| `sessions` | identity-session-service | Full session model (with mfa_verified, impersonated_by) |
| `tokens` | identity-session-service | Access/refresh token tracking |
| `impersonations` | identity-session-service | Admin impersonation records |
| `mfa_setup` | identity-session-service, identity-user-mgmt-service | TOTP/SMS MFA |
| `user_profiles` | identity-session-service | Extended user metadata |
| `mcp_agents` | identity-session-service | MCP agent configuration |

### User Management (identity-user-mgmt-service)

| Table | Service | Purpose |
|-------|---------|---------|
| `users` | identity-user-mgmt-service | Same as identity-login-service |
| `email_verifications` | identity-user-mgmt-service | Email verification tokens |
| `social_accounts` | identity-user-mgmt-service | Social account linking |
| `employees` | identity-user-mgmt-service | Employee metadata |
| `audit_events` | identity-user-mgmt-service | Rich audit logging |

### Access Management (org-mgmt)

| Table | Service | Purpose |
|-------|---------|---------|
| `organizations` | org-mgmt | Multi-tenant orgs |
| `org_memberships` | org-mgmt | User-org relationships |
| `org_invites` | org-mgmt | Pending invitations |
| `org_domains` | org-mgmt | Verified domains |
| `roles` | org-mgmt | Flat roles (no inheritance) |
| `permissions` | org-mgmt | Org-scoped permissions |
| `role_permissions` | org-mgmt | Role-permission bridge |
| `applications` | org-mgmt | OIDC clients within org |
| `saml_connections` | org-mgmt | SAML IdP configuration |
| `scim_users` | org-mgmt | SCIM provisioned users |
| `webhook_subscriptions` | org-mgmt | Webhook endpoints |

### Authorization (authz-core)

| Table | Service | Purpose |
|-------|---------|---------|
| `audit_events` | authz-core | Lightweight audit |
| `audit_retention_policies` | authz-core | Audit log retention |
| `authorizations` | authz-core | ABAC-style records |
| `role_assignments` | authz-core | Principal role assignments |
| `principal_attributes` | authz-core | Custom user attributes |

### API Keys (api-keys)

| Table | Service | Purpose |
|-------|---------|---------|
| `api_keys` | api-keys | API keys with permissions |
| `api_key_usage` | api-keys | Usage tracking |
| `archived_api_keys` | api-keys | Revoked keys |

## Key Design Decisions (Verified)

1. **Single user table.** User type is distinguished by JWT claim, NOT a DB column. The `user_type` column does NOT exist in the impl.
2. **No soft deletes.** `deleted_at` columns do NOT exist on any table. Status is managed via `status` varchar columns.
3. **Two session models.** Both `identity-login-service` and `identity-session-service` have `sessions` tables — identical schema with minor differences.
4. **Flat roles.** The `parent_role_id` column does NOT exist. Roles are flat — no hierarchy/inheritance.
5. **No `tenants` table.** Tenants are a logical boundary enforced via `tenant_id` varchar(255) columns on all tables.
6. **Org-scoped RBAC.** Roles, permissions, applications, and memberships are scoped to `org_id`, NOT tenant or application.
7. **Permissions as org-scoped.** Permissions have `org_id` FK, not tenant_id or application_id.
8. **MFA duplicated.** The exact same `mfa_setup` model exists in TWO services (identity-session-service + identity-user-mgmt-service).
9. **Two audit event models.** `authz-core` uses a lightweight model; `identity-user-mgmt-service` has a richer model with user_id FK.
10. **`tenant_id` is varchar(255).** Consistent across all services that use it (NOT uuid).

## ERD (simplified — see `topic-entity-relationship-diagram.md` for full diagram)

```
users (1:N)──> sessions (1:N)──> impersonations
                │
                ├──> tokens
                │
                ├──> user_profiles
                │
                ├──> mfa_setup (identity-session + identity-user-mgmt)
                │
                ├──> social_credentials
                ├──> otp_tokens
                ├──> magic_link_tokens
                │
                ├──> email_verifications
                ├──> social_accounts
                ├──> employees
                │
                ├──> role_assignments (authz-core)
                ├──> principal_attributes (authz-core)
                ├──> authorizations (authz-core)
                ├──> audit_events (authz-core + identity-user-mgmt)
                │
                └──> api_keys (via user_id)
                      │
                      └──> api_key_usage
                            └──> archived_api_keys

organizations (1:N)──> org_memberships
                      ├──> org_invites
                      ├──> org_domains
                      ├──> roles (1:N)──> role_permissions (1:N)──> permissions
                      ├──> applications
                      ├──> saml_connections
                      ├──> scim_users
                      └──> webhook_subscriptions
```

## Code Anchors

- `docs/llmwiki/topics/topic-entity-relationship-diagram.md` — **Full ERD** with all foreign keys and service ownership
- `microservices/idam/*/impl/src/models/*.rs` — All 40+ Lifeguard entity definitions
- `openapi/idam/*/openapi.yaml` — All 6 OpenAPI specs

## Gaps / Drift (Resolved 2026-05-16)

All gaps identified by the comprehensive entity audit have been resolved. See individual entity pages for detailed drift tables.

> **Resolved:** `topic-entity-relationship-diagram.md` created with full ERD reconciled from OpenAPI specs + impl models.
