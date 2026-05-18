---
title: Data Model
status: verified
updated: 2026-05-17
sources: [openapi/idam/*/openapi.yaml, microservices/*/impl/src/models/, topic-entity-relationship-diagram.md]
---

# Data Model

## Overview

This data model reflects the ACTUAL Lifeguard entity models in the `impl/` crates, verified against all 6 OpenAPI specs on 2026-05-17. For the complete entity relationship diagram with all foreign keys, see [`topic-entity-relationship-diagram.md`](./topic-entity-relationship-diagram.md).

## All Tables (41 across 6 services)

### Identity Layer (identity-login-service)

| Table | Purpose | Columns |
|-------|---------|---------|
| `users` | Registered users | `id, email, password_hash, tenant_id, email_verified, phone, phone_verified, status, created_at, updated_at` |
| `sessions` | Auth sessions (login version) | `id, user_id, token, refresh_token, expires_at, ip, user_agent, created_at, updated_at` |
| `social_credentials` | OAuth provider linking | `id, user_id, provider, provider_user_id, access_token, refresh_token, created_at, updated_at` |
| `otp_tokens` | Email/phone OTP verification | `id, user_id, type_field, code, expires_at, attempts, max_attempts, created_at, updated_at` |
| `magic_link_tokens` | Passwordless magic links | `id, user_id, link, expires_at, used, created_at, updated_at` |

### Session Layer (identity-session-service)

| Table | Purpose | Columns |
|-------|---------|---------|
| `sessions` | Full session model (with mfa_verified, impersonated_by) | `id, user_id, token, refresh_token, expires_at, ip, user_agent, mfa_verified, impersonated_by, created_at, updated_at` |
| `tokens` | Access/refresh token tracking | `id, user_id, session_id, type_field, token, expires_at, created_at, updated_at` |
| `impersonations` | Admin impersonation records | `id, user_id, impersonator_id, session_id, created_at, restored_at` |
| `mfa_setup` | TOTP/SMS MFA | `id, user_id, factor_type, secret, enabled, created_at, updated_at` |
| `user_profiles` | Extended user metadata | `id, user_id, first_name, last_name, avatar_url, created_at, updated_at` |
| `mcp_agents` | MCP agent configuration | `id, user_id, name, description, config, created_at, updated_at` |

### User Management (identity-user-mgmt-service)

| Table | Purpose | Columns |
|-------|---------|---------|
| `users` | Same as identity-login-service | `id, email, password_hash, tenant_id, status, email_verified, phone, phone_verified, created_at, updated_at` |
| `email_verifications` | Email verification tokens | `id, user_id, token, expires_at, created_at, updated_at` |
| `social_accounts` | Social account linking | `id, user_id, provider, provider_user_id, access_token, refresh_token, created_at, updated_at` |
| `employees` | Employee metadata | `id, user_id, employee_id, department, title, manager_id, created_at, updated_at` |
| `mfa_setup` | TOTP/SMS MFA (duplicate of session-service) | `id, user_id, factor_type, secret, enabled, created_at, updated_at` |
| `audit_events` | Rich audit logging | `id, tenant_id, user_id, event_type, severity, actor, data, ip, user_agent, created_at` |

### Access Management (org-mgmt)

| Table | Purpose | Columns |
|-------|---------|---------|
| `organizations` | Multi-tenant orgs | `id, name, tenant_id, status, created_at, updated_at` |
| `org_memberships` | User-org relationships | `id, org_id, user_id, role, status, created_at, updated_at` |
| `org_invites` | Pending invitations | `id, org_id, email, role, token, expires_at, created_at, accepted_at` |
| `org_domains` | Verified domains | `id, org_id, domain, verified, created_at, updated_at` |
| `roles` | Flat roles (no inheritance) | `id, org_id, name, description, created_at, updated_at` |
| `permissions` | Org-scoped permissions | `id, org_id, name, description, resource, action, created_at, updated_at` |
| `role_permissions` | Role-permission bridge | `id, role_id, permission_id, created_at` |
| `applications` | OIDC clients within org | `id, org_id, name, client_id, client_secret, redirect_uris, created_at, updated_at` |
| `saml_connections` | SAML IdP configuration | `id, org_id, issuer, metadata_url, sso_url, signing_cert, created_at, updated_at` |
| `scim_users` | SCIM provisioned users | `id, org_id, external_id, username, email, created_at, updated_at` |
| `webhook_subscriptions` | Webhook endpoints | `id, org_id, url, events, secret, active, created_at, updated_at` |

### Authorization (authz-core)

| Table | Purpose | Columns |
|-------|---------|---------|
| `audit_events` | Lightweight audit | `id, tenant_id, event_type, severity, actor, data, ip, created_at` |
| `audit_retention_policies` | Audit log retention | `id, tenant_id, retention_days, enabled, created_at, updated_at` |
| `authorizations` | ABAC-style records | `id, principal_id, action, resource, effect, tenant_id, created_at, updated_at` |
| `role_assignments` | Principal role assignments | `id, principal_id, role_name, resource_type, resource_id, tenant_id, created_at, updated_at` |
| `principal_attributes` | Custom user attributes | `id, principal_id, key, value, tenant_id, created_at, updated_at` |

### API Keys (api-keys)

| Table | Purpose | Columns |
|-------|---------|---------|
| `api_keys` | API keys with permissions | `id, key_hash, key_prefix, name, tenant_id, user_id, org_id, permissions, expires_at, active, created_at, updated_at` |
| `api_key_usage` | Usage tracking | `id, key_id, endpoint, method, tenant_id, ip, created_at` |
| `archived_api_keys` | Revoked keys | `id, key_hash, key_prefix, name, reason, archived_at` |

## Key Design Decisions (Verified)

1. **Single user table.** User type is distinguished by JWT claim, NOT a DB column. The `user_type` column does NOT exist in the impl.
2. **No soft deletes.** `deleted_at` columns do NOT exist on any table. Status is managed via `status` varchar columns.
3. **Two session models.** Both `identity-login-service` and `identity-session-service` have `sessions` tables — different schemas with minor differences.
4. **Flat roles.** The `parent_role_id` column does NOT exist. Roles are flat — no hierarchy/inheritance.
5. **No `tenants` table.** Tenants are a logical boundary enforced via `tenant_id` varchar(255) columns on all tables.
6. **Org-scoped RBAC.** Roles, permissions, applications, and memberships are scoped to `org_id`, NOT tenant or application.
7. **Permissions as org-scoped.** Permissions have `org_id` FK, not tenant_id or application_id.
8. **MFA duplicated.** The exact same `mfa_setup` model exists in TWO services (identity-session-service + identity-user-mgmt-service).
9. **Two audit event models.** `authz-core` uses a lightweight model (8 columns); `identity-user-mgmt-service` has a richer model with user_id (10 columns).
10. **`tenant_id` is varchar(255).** Consistent across all services that use it (NOT uuid).
11. **All tables include created_at.** Every model has `created_at` timestamps. Most also have `updated_at`.
12. **API keys are dual-scoped.** Keys can be user-scoped (`user_id` FK) or org-scoped (`org_id` FK) — both are `Option<uuid>`.
13. **RolePermission is a bridge table.** Many-to-many relationship between roles and permissions, not a simple child entity.
14. **Role/Permission are org-scoped in impl.** The `Role` and `Permission` structs both use `org_id FK`, NOT `application_id`.

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
- `microservices/idam/*/impl/src/models/*.rs` — All 41 Lifeguard entity definitions
- `openapi/idam/*/openapi.yaml` — All 6 OpenAPI specs

## Gaps / Drift (2026-05-17 Audit)

A comprehensive audit was performed on 2026-05-17 comparing all 41 impl model files against the OpenAPI specs for all 6 services. The following gaps were identified:

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
