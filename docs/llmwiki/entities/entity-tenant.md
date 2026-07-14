---
title: Platform Tenant
status: verified
updated: 2026-07-14
sources: [microservices/idam/identity-login-service/impl/src/models/tenant.rs, migrations/identity-login-service/20260714102157_tenants.sql]
---

# Entity: Platform Tenant

Owned by: **identity-login-service**; consumed as an isolation boundary by all services

## Description

**Tenant = SaaS product isolation boundary.** A tenant represents a product ecosystem such as Hauliage or RERP, not an end-customer organization. Tenants are completely isolated from each other. Users, customer organizations, keys, roles, and permissions belong to exactly one tenant.

The canonical `sesame_idam.tenants` registry must contain an active row before authentication traffic is accepted. Its UUID `id` is internal registry identity; its unique string `slug` (`hauliage`, `rerp`) is the value carried by `X-Tenant-ID`, JWT `tenant_id`, the RLS context, and dependent tables' string `tenant_id` columns. Headers are cross-checks after authentication and never create identity.

**Examples:**
- `hauliage` = one tenant (includes hauliage-web, hauliage-api, hauliage-admin, hauliage-mobile)
- `rerp` = one tenant (includes rerp-openapi, rerp-admin, rerp-mobile)

## How Tenant Isolation Works

Tenant isolation is enforced at three layers:

1. **Application layer:** BRRTRouter validates the credential and cross-checks any `X-Tenant-ID` header against the validated string tenant claim.
2. **Database layer:** Lifeguard's base pool/executors inject the validated `SessionContext` through the versioned helper on a pinned transaction.
3. **RLS policies:** PostgreSQL policies use the typed Sesame accessors as a failsafe when an application predicate is absent.

## tenant_id Column Across All Services

| Service | Tables with tenant_id |
|---------|----------------------|
| identity-login-service | `users` |
| identity-user-mgmt-service | `users` |
| org-mgmt | `organizations` |
| api-keys | `api_keys`, `api_key_usage` |
| authz-core | `audit_events`, `role_assignments`, `principal_attributes`, `authorizations` |

## Key Design Decisions

1. **Tenant = registered isolation boundary.** `sesame_idam.tenants` is the provisioned source of valid platform tenant slugs; dependent tables use the slug as `tenant_id`.
2. **Applications are children of tenant.** Each tenant can have multiple applications that share the same data.
3. **X-Tenant-ID is a claimed tenant selector.** Authentication verifies the registered slug and subsequent requests cross-check it against the credential.
4. **External `tenant_id` is a string, not the registry UUID.** This is the RLS v1 contract and the dependent-table representation.
5. **Tenant lifecycle is explicit.** Registry status is `active`, `suspended`, or `provisioning`; there is no implicit tenant creation.

## Historical drift resolved

| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|--------|
| Earlier wiki said no `tenants` table | The platform registry was delivered in `20260714102157_tenants.sql` | Resolved; tenant provisioning is now explicit |
| Earlier designs conflated tenant and customer organization | Tenant is the SaaS product partition; `org_id` is the customer workspace | Resolved in claims and RLS v1 |
| Earlier designs typed external tenant identity as UUID | External tenant identity is the registered string slug; subject and organization remain UUIDs | Resolved in code, SQL, and this documentation |
