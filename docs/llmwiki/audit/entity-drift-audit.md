# Sesame-IDAM: Entity Relationship Audit

> **Date:** 2026-05-16
> **Status:** CRITICAL — Design-to-Implementation Drift Detected
> **Scope:** All 6 microservices, 42 model files, 11 wiki entity docs, 6 OpenAPI specs

## Executive Summary

The design document ERD (design-doc.md section 5.1) defines **15 canonical entities** with rich schemas (10-30+ columns each, full RLS, soft delete, tenant scoping). The actual Lifeguard models contain **42 model files** across 6 services, but with **extreme drift** — most models are skeletal (6-12 columns) and missing virtually every field from the design.

**This is not a minor gap.** The design describes a complete IDAM platform; the models represent a barebones skeleton. Every implementation story will need to bridge these gaps.

## Drift Classification

| Severity | Definition | Count |
|----------|-----------|-------|
| **P0 — Missing Core Entity** | Entity exists in design but has NO model file | 2 |
| **P1 — Massive Drift** | Entity has model but design specifies 50%+ fewer columns | 8 |
| **P2 — Moderate Drift** | Entity has model but design specifies 20-50% fewer columns | 5 |
| **P3 — Minor Drift** | Entity has model, columns within 20% of design | 4 |
| **OK — Aligned** | Model matches design closely | 5 |

## Canonical Entity-to-Model Mapping

### 1. User

**Wiki:** entity-user.md | **Models:** 2 (identity-login-service, identity-user-mgmt-service) | **Design ERD:** 21 columns | **Actual:** 10 columns

| Field | Design ERD | Design Doc | OpenAPI | Login Service Model | User-Mgmt Model | Drift |
|-------|-----------|------------|---------|-------------------|-----------------|-------|
| id | uuid PK | uuid PK | user_id string | uuid PK | uuid PK | OK |
| email | text | text | string | VARCHAR(255) | VARCHAR(255) | OK |
| tenant_id | uuid FK | uuid FK | N/A | VARCHAR(255) | VARCHAR(255) | P1 |
| email_confirmed | boolean | boolean | boolean (email_confirmed) | bool (email_verified) | bool (email_verified) | P2 |
| phone_number | text | text | string | VARCHAR(64) NULL | VARCHAR(64) NULL | OK |
| phone_confirmed | boolean | boolean | boolean (phone_verified) | bool (phone_verified) | bool (phone_verified) | OK |
| user_type | text | text | N/A | **MISSING** | **MISSING** | P0 |
| first_name | text | text | string | **MISSING** | **MISSING** | P0 |
| last_name | text | text | string | **MISSING** | **MISSING** | P0 |
| username | text | text | string | **MISSING** | **MISSING** | P0 |
| picture_url | text | text | string NULL | **MISSING** | **MISSING** | P0 |
| extra_properties | jsonb | jsonb | object | **MISSING** | **MISSING** | P0 |
| locked | boolean | boolean | boolean | **MISSING** | **MISSING** | P0 |
| enabled | boolean | boolean | boolean | **MISSING** | **MISSING** | P0 |
| has_password | boolean | boolean | boolean | **MISSING** | **MISSING** | P0 |
| password_hash | text | text | N/A | TEXT | TEXT | OK |
| created_at | timestamptz | timestamptz | string | timestamptz | timestamptz | OK |
| updated_at | timestamptz | timestamptz | string | timestamptz | timestamptz | OK |
| last_active_at | timestamptz | timestamptz | **MISSING** | **MISSING** | **MISSING** | P0 |
| deleted_at | timestamptz | timestamptz | **MISSING** | **MISSING** | **MISSING** | P0 |
| status | N/A | N/A | N/A | VARCHAR(32) | VARCHAR(32) | — |

**Verdict:** P0 — Both models have 10 columns vs 21 in design. Critical fields all missing: user_type, first_name, last_name, username, picture_url, extra_properties, locked, enabled, has_password, last_active_at, deleted_at. Two identical User models in 2 services (duplicate). The design doc describes a complete identity entity but both models are bare.

### 2. Session

**Wiki:** entity-session.md | **Models:** 2 (identity-login-service, identity-session-service) | **Design ERD:** 10 columns | **Actual:** 9-11 columns

| Field | Design ERD | Design Doc | OpenAPI | Login Service | Session Service | Drift |
|-------|-----------|------------|---------|--------------|-----------------|-------|
| id | uuid PK | uuid PK | N/A | uuid PK | uuid PK | OK |
| user_id | uuid FK | uuid FK | N/A | uuid FK | uuid FK | OK |
| tenant_id | uuid FK | uuid FK | N/A | **MISSING** | **MISSING** | P0 |
| session_token | text hashed | text hashed | N/A | TEXT (token) | TEXT (token) | OK |
| refresh_token | text hashed | text hashed | N/A | TEXT (refresh_token) | TEXT (refresh_token) | OK |
| ip_address | inet | inet | N/A | VARCHAR(64) NULL | **MISSING** | P2 |
| user_agent | text | text | N/A | **MISSING** | TEXT NULL | P2 |
| created_at | timestamptz | timestamptz | N/A | timestamptz | timestamptz | OK |
| expires_at | timestamptz | timestamptz | N/A | timestamptz | timestamptz | OK |
| revoked | boolean | boolean | N/A | **MISSING** | **MISSING** | P0 |
| last_used_at | timestamptz | timestamptz | N/A | **MISSING** | **MISSING** | P0 |
| impersonated_by | uuid FK | uuid FK | N/A | UUID NULL | **MISSING** | P1 |
| step_up_verified | boolean | boolean | N/A | **MISSING** | **MISSING** | P0 |
| step_up_verified_at | timestamptz | timestamptz | N/A | **MISSING** | **MISSING** | P0 |
| mfa_verified | N/A | N/A | N/A | BOOLEAN | BOOLEAN | — |

**Verdict:** P1 — login-service has 9 cols (minus tenant_id, ip, user_agent, revoked, last_used_at, step_up). session-service has 11 cols but includes impersonation and mfa fields the login-service lacks. Both lack tenant_id, revoked flag, last_used_at, and step-up MFA tracking. The session-service model is richer but still incomplete.

### 3. Organization

**Wiki:** entity-organization.md | **Models:** 1 (org-mgmt) | **Design ERD:** 23 columns | **Actual:** 6 columns

| Field | Design ERD | Design Doc | OpenAPI | Actual Model | Drift |
|-------|-----------|------------|---------|-------------|-------|
| id | uuid PK | uuid PK | string | uuid PK | OK |
| tenant_id | uuid FK | uuid FK | **MISSING** | VARCHAR(255) | P1 |
| name | text | text | string | VARCHAR(255) | OK |
| slug | text UK | text | string | **MISSING** | P0 |
| logo_url | text | text | string NULL | **MISSING** | P0 |
| domain | text | text | string NULL | **MISSING** | P0 |
| domains | text[] | text[] | array<string> | **MISSING** | P0 |
| domain_auto_join | boolean | boolean | boolean | **MISSING** | P0 |
| domain_restrict | boolean | boolean | boolean | **MISSING** | P0 |
| password_rotation_enabled | boolean | boolean | boolean | **MISSING** | P0 |
| password_rotation_history_size | integer | integer | integer | **MISSING** | P0 |
| password_rotation_period | integer | integer | integer | **MISSING** | P0 |
| max_users | integer | integer | integer NULL | **MISSING** | P0 |
| metadata | jsonb | jsonb | object NULL | **MISSING** | P0 |
| is_saml_configured | boolean | boolean | boolean | **MISSING** | P0 |
| is_saml_in_test_mode | boolean | boolean | boolean | **MISSING** | P0 |
| can_setup_saml | boolean | boolean | boolean | **MISSING** | P0 |
| isolated | boolean | boolean | boolean | **MISSING** | P0 |
| sso_trust_level | text | text | string NULL | **MISSING** | P0 |
| legacy_org_id | text | text | string NULL | **MISSING** | P0 |
| status | N/A | N/A | N/A | VARCHAR(32) | — |
| created_at | timestamptz | timestamptz | string | timestamptz | OK |
| updated_at | timestamptz | timestamptz | string | timestamptz | OK |
| deleted_at | timestamptz | timestamptz | **MISSING** | **MISSING** | P0 |

**Verdict:** P0 — **Massive drift. 6 actual columns vs 23 in design. 17 columns completely missing.** The org-mgmt model is a barebone id+name+tenant_id+status+timestamps. All SAML settings, domain controls, password rotation, seat management, metadata, slug, logo_url are absent.

### 4. Role

**Wiki:** entity-role.md | **Models:** 1 (org-mgmt) | **Design ERD:** 10 columns | **Actual:** 6 columns

| Field | Design ERD | OpenAPI | Actual Model | Drift |
|-------|-----------|---------|-------------|-------|
| id | uuid PK | string | uuid PK | OK |
| tenant_id | uuid FK | string | **MISSING** | P0 |
| organization_id | uuid FK | string | uuid FK | P1 |
| name | text | string | VARCHAR(255) | OK |
| display_name | text | **MISSING** | **MISSING** | P0 |
| description | text | string NULL | TEXT NULL | OK |
| is_system | boolean | **MISSING** | **MISSING** | P0 |
| parent_role_id | uuid FK | **MISSING** | **MISSING** | P0 |
| created_at | timestamptz | string | timestamptz | OK |
| updated_at | timestamptz | string | timestamptz | OK |
| application_id | N/A | string | **MISSING** | P0 |

**Verdict:** P1 — 6 actual columns vs 10 in design. Missing: tenant_id (critical!), display_name, is_system, parent_role_id (inheritance), application_id. Role inheritance is a core design concept — no model to support it.

### 5. Permission

**Wiki:** entity-permission.md | **Models:** 1 (org-mgmt) | **Design ERD:** 5 columns | **Actual:** 8 columns

| Field | Design ERD | OpenAPI | Actual Model | Drift |
|-------|-----------|---------|-------------|-------|
| id | uuid PK | string | uuid PK | OK |
| tenant_id | uuid FK | string | **MISSING** | P0 |
| name | text | string | VARCHAR(255) | OK |
| description | text | string NULL | TEXT NULL | OK |
| created_at | timestamptz | string | timestamptz | OK |
| application_id | N/A | string | uuid FK | — |
| org_id | N/A | **MISSING** | uuid FK | — |
| resource | N/A | **MISSING** | VARCHAR(255) | — |
| action | N/A | **MISSING** | VARCHAR(255) | — |

**Verdict:** P2 — Actually **more columns than design** but shifted to app/org-scoped with resource:action naming. Missing tenant_id (P0). Uses org_id FK but design says tenant_id. Different model shape.

### 6. RolePermission (junction table)

**Wiki:** (in entity-role.md) | **Models:** 1 (org-mgmt) | **Design ERD:** 2 columns | **Actual:** 4 columns

| Field | Design ERD | Actual Model | Drift |
|-------|-----------|-------------|-------|
| role_id | uuid FK PK | uuid PK | OK |
| permission_id | uuid FK PK | uuid FK | OK |
| id | N/A | uuid PK | — |
| application_id | N/A | uuid FK | — |

**Verdict:** P2 — Added id PK + application_id FK not in design. Design has pure composite PK junction table.

### 7. UserOrganization (org membership)

**Wiki:** (in entity-organization.md) | **Models:** 1 (org-mgmt) | **Design ERD:** 7 columns | **Actual:** 7 columns

| Field | Design ERD | OpenAPI | Actual Model | Drift |
|-------|-----------|---------|-------------|-------|
| user_id | uuid FK PK | N/A | uuid FK | OK |
| org_id | uuid FK PK | N/A | uuid FK | OK |
| role_id | uuid FK | N/A | **MISSING** | P1 |
| role | text | N/A | VARCHAR(255) | OK |
| additional_roles[] | text[] | N/A | **MISSING** | P0 |
| joined_at | timestamptz | N/A | timestamptz | OK |
| invited_at | timestamptz | N/A | **MISSING** | P0 |
| id | N/A | N/A | uuid PK | — |
| status | N/A | N/A | VARCHAR(32) | — |
| created_at | N/A | N/A | timestamptz | — |
| updated_at | N/A | N/A | timestamptz | — |

**Verdict:** P2 — Model matches design closely but adds id PK and timestamps. Missing role_id FK and additional_roles array. Uses single `role` string field instead.

### 8. APIKey

**Wiki:** entity-api-key.md | **Models:** 3 (api-keys) | **Design ERD:** 10 columns | **Actual:** 12 columns

| Field | Design ERD | OpenAPI | Actual Model | Drift |
|-------|-----------|---------|-------------|-------|
| id | uuid PK | string | uuid PK | OK |
| tenant_id | uuid FK | N/A | VARCHAR(255) | OK |
| user_id | uuid FK | string NULL | UUID NULL + FK | OK |
| org_id | uuid FK | string NULL | UUID NULL + FK | OK |
| key_hash | text | **MISSING** | TEXT | OK |
| key_prefix | text | **MISSING** | VARCHAR(16) | OK |
| display_name | text | string | VARCHAR(255) | OK |
| description | text NULL | **MISSING** | **MISSING** | P0 |
| metadata | jsonb | object NULL | **MISSING** | P0 |
| expires_at | timestamptz | integer NULL | timestamptz NULL | OK |
| revoked | boolean | boolean (active) | BOOLEAN (active) | P2 |
| created_at | timestamptz | integer | timestamptz | OK |
| updated_at | N/A | **MISSING** | timestamptz | — |
| last_used_at | timestamptz | **MISSING** | **MISSING** | P0 |

**Verdict:** P2 — 12 actual vs 10 in design. Has updated_at (extra). Missing description, metadata, last_used_at. Uses `active` boolean instead of `revoked`. `tenant_id` is VARCHAR not UUID. Two separate model files (ApiKey, ArchivedApiKey) where design uses one table with soft delete.

### 9. MFADevice / MfaSetup

**Wiki:** entity-mfa-device.md | **Models:** 2 (identity-user-mgmt-service, identity-session-service) | **Design ERD:** 8 columns | **Actual:** 7 columns

| Field | Design ERD | OpenAPI | MfaSetup (user-mgmt) | MfaSetup (session) | Drift |
|-------|-----------|---------|---------------------|-------------------|-------|
| id | uuid PK | N/A | uuid PK | uuid PK | OK |
| user_id | uuid FK | N/A | uuid FK | uuid FK | OK |
| type | text | N/A | VARCHAR(16) | VARCHAR(16) | OK |
| secret | text | N/A | TEXT NULL | TEXT | OK |
| is_active | boolean | N/A | BOOLEAN | BOOLEAN | OK |
| label | text | N/A | VARCHAR(255) NULL | **MISSING** | P2 |
| tenant_id | uuid FK | N/A | VARCHAR(255) | **MISSING** | P1 |
| created_at | timestamptz | N/A | timestamptz | timestamptz | OK |
| last_used_at | timestamptz | N/A | **MISSING** | **MISSING** | P0 |
| mfa_type | N/A | N/A | VARCHAR(32) | **MISSING** | — |
| name | N/A | N/A | VARCHAR(255) NULL | VARCHAR(255) | — |

**Verdict:** P1 — 7 columns vs 8 in design. Missing last_used_at, tenant_id (session-service), label (session-service). Two duplicate MfaSetup models across services.

### 10. AuditLog / AuditEvent

**Wiki:** entity-audit-log.md | **Models:** 2 (authz-core, identity-user-mgmt-service) | **Design ERD:** 10 columns | **Actual:** 8-10 columns

| Field | Design ERD | OpenAPI | authz-core | user-mgmt | Drift |
|-------|-----------|---------|-----------|-----------|-------|
| id | uuid PK | string | uuid PK | uuid PK | OK |
| user_id | uuid | string NULL | uuid FK | uuid FK | OK |
| org_id | uuid | string NULL | uuid NULL | uuid FK | OK |
| tenant_id | uuid FK | string | VARCHAR(16) | **MISSING** | P0 |
| action | text | string | text | VARCHAR(255) | OK |
| resource_type | text | string NULL | VARCHAR(255) NULL | VARCHAR(255) | OK |
| resource_id | text | string NULL | VARCHAR(255) NULL | **MISSING** | P1 |
| metadata | jsonb | object NULL | **MISSING** | jsonb NULL | — |
| ip_address | inet | string NULL | VARCHAR(64) NULL | VARCHAR(45) NULL | OK |
| user_agent | text | string NULL | TEXT NULL | TEXT NULL | OK |
| timestamp | timestamptz | string | timestamptz | timestamptz | OK |
| event_type | N/A | enum | VARCHAR(32) | **MISSING** | P1 |
| severity | N/A | enum | VARCHAR(16) NULL | **MISSING** | P1 |
| actor | N/A | enum | VARCHAR(32) | **MISSING** | P1 |

**Verdict:** P1 — authz-core has 8 cols, user-mgmt has 10. authz-core is missing resource_id, metadata, event_type, severity, actor. user-mgmt model adds more fields but missing tenant_id, resource_id. Design says tenant_id FK required — missing from user-mgmt model.

### 11. WebhookEndpoint / WebhookDelivery

**Wiki:** entity-webhook.md | **Models:** 1 (org-mgmt: WebhookSubscription) | **Design ERD:** 2 tables, 19 columns combined | **Actual:** 1 model, 8 columns

**WebhookSubscription (actual)** | **Design: WebhookEndpoint** | **Design: WebhookDelivery** | Drift
------|---------|---------|------
id uuid PK | id uuid PK | id uuid PK | OK
org_id uuid FK | org_id uuid FK | webhook_endpoint_id uuid FK | OK
endpoint_url | url | — | P2
events | events text[] | event_type | P1
enabled | is_active | — | P2
secret | secret | — | P2
metadata | — | — | —
created_at | created_at | created_at | OK
updated_at | updated_at | — | OK

**Verdict:** P1 — Only WebhookEndpoint has a model. WebhookDelivery (delivery tracking, retry logic, response recording) has NO model at all. The OpenAPI spec has webhook endpoints but no delivery tracking endpoints.

### 12. McpAgent

**Wiki:** (in entity-session.md) | **Models:** 1 (identity-session-service) | **Design ERD:** 12 columns | **Actual:** 7 columns

| Field | Design ERD | Actual Model | Drift |
|-------|-----------|-------------|-------|
| agent_id | uuid PK | uuid PK | OK |
| tenant_id | uuid FK | **MISSING** | P0 |
| name | text | VARCHAR(255) | OK |
| tool_namespace | text | VARCHAR(255) | OK |
| description | text | TEXT NULL | OK |
| api_key_prefix | text | VARCHAR(16) | OK |
| active | boolean | BOOLEAN | OK |
| max_tokens_per_minute | integer | **MISSING** | P0 |
| metadata | jsonb | **MISSING** | P0 |
| created_at | timestamptz | timestamptz | OK |
| last_used_at | timestamptz | **MISSING** | P0 |
| total_tokens_issued | integer | **MISSING** | P0 |

**Verdict:** P1 — 7 actual vs 12 in design. Missing: tenant_id, max_tokens_per_minute, metadata, last_used_at, total_tokens_issued.

### 13. Tenant

**Wiki:** entity-tenant.md | **Models:** **NONE** | **Design ERD:** 6 columns | **Actual:** 0 columns

**Verdict:** P0 — **Entity exists in design and wiki but has NO model file whatsoever.** The Tenant is the fundamental isolation boundary — it has zero implementation. All tenant_id fields in other tables are bare VARCHAR strings with no FK or entity.

### 14. Impersonation

**Wiki:** (in entity-session.md) | **Models:** 1 (identity-session-service) | **Design ERD:** 6 columns | **Actual:** 6 columns

| Field | Design ERD | OpenAPI | Actual Model | Drift |
|-------|-----------|---------|-------------|-------|
| id | uuid PK | N/A | uuid PK | OK |
| impersonator_id | uuid FK | N/A | uuid FK | OK |
| target_user_id | uuid FK | N/A | uuid FK | OK |
| started_at | timestamptz | N/A | timestamptz | OK |
| ended_at | timestamptz | N/A | **MISSING** | P1 |
| tenant_id | uuid FK | N/A | **MISSING** | P0 |

**Verdict:** P1 — 6 cols but missing tenant_id and ended_at. Impersonation sessions can't be tracked as time-bounded.

### 15. Token (refresh tokens)

**Wiki:** (in entity-session.md) | **Models:** 1 (identity-session-service) | **Design ERD:** 8 columns | **Actual:** 8 columns

| Field | Design ERD | Actual Model | Drift |
|-------|-----------|-------------|-------|
| id | uuid PK | uuid PK | OK |
| user_id | uuid FK | uuid FK | OK |
| tenant_id | uuid FK | **MISSING** | P0 |
| refresh_token_hash | text | TEXT | OK |
| session_id | uuid FK | uuid FK | OK |
| expires_at | timestamptz | timestamptz | OK |
| revoked | boolean | BOOLEAN | OK |
| created_at | timestamptz | timestamptz | OK |
| last_used_at | timestamptz | timestamptz | — |

**Verdict:** P1 — 8 columns but missing tenant_id (critical for multi-tenant isolation).

## Additional Models (Not in Design ERD)

These model files exist but are **not represented in the design document ERD at all**:

| Model | Table | Service | Cols | Notes |
|-------|-------|---------|------|-------|
| ApiKeyUsage | api_key_usage | api-keys | 7 | Usage tracking — no design |
| ArchivedApiKey | archived_api_keys | api-keys | 6 | Separate table for soft-deleted keys — design uses deleted_at |
| Authorization | authorizations | authz-core | 8 | ABAC rules — partially in design |
| AuditRetentionPolicy | audit_retention_policies | authz-core | 6 | Retention policies — no design |
| PrincipalAttribute | principal_attributes | authz-core | 7 | ABAC attributes — no design |
| RoleAssignment | role_assignments | authz-core | 8 | Role assignments — no design |
| MagicLinkToken | magic_link_tokens | login | 7 | Passwordless tokens — no design |
| OTPToken | otp_tokens | login | 9 | OTP codes — no design |
| SocialCredential | social_credentials | login | 8 | OAuth credentials — partially in design |
| Impersonation | impersonations | session | 6 | Admin impersonation — partially in design |
| EmailVerification | email_verifications | user-mgmt | 6 | Email verification tokens — no design |
| Employee | employees | user-mgmt | 8 | Employee mode — no design |
| SocialAccount | social_accounts | user-mgmt | 8 | Linked social accounts — partially in design |
| OrgDomain | org_domains | org-mgmt | 6 | Multi-domain orgs — no design |
| OrgInvite | org_invites | org-mgmt | 8 | Org invitations — no design |
| SamlConnection | saml_connections | org-mgmt | 8 | SAML IdP connections — no design |
| ScimUser | scim_users | org-mgmt | 7 | SCIM provisioning — no design |
| OrgMembership | org_memberships | org-mgmt | 7 | Org membership — partially in design |

**Verdict:** 18 new models that extend the design ERD but were never documented. The design ERD is incomplete.

## OpenAPI Schema Gaps

The OpenAPI specs define schemas that don't map to any model:

| Schema | Service | Type | Model Gap |
|--------|---------|------|-----------|
| UserProfile | session | entity | user_profiles model exists but has different shape |
| Token | session | response | tokens model exists |
| MfaSetup | session | entity | mfa_setup model exists in 2 services |
| McpAgent | session | entity | mcp_agents model exists but missing fields |
| AuditEventFilter | authz-core | request | No model |
| AuthorizeRequest | authz-core | request | authorizations model exists |
| EffectiveRequest/Response | authz-core | request | No model |

## Cross-Service Model Duplication

| Entity | Services with Model | Drift Between Services |
|--------|-------------------|----------------------|
| User | identity-login-service, identity-user-mgmt-service | **Identical** (both 10 cols) |
| Session | identity-login-service, identity-session-service | **Different** (9 vs 11 cols) |
| MFADevice | identity-user-mgmt-service, identity-session-service | **Different** (7 vs 7 cols) |
| AuditEvent | authz-core, identity-user-mgmt-service | **Different** (8 vs 10 cols) |

**Problem:** The same entity has different models in different services. The login-service User and user-mgmt-service User are identical but could diverge. Session models differ in columns but serve different purposes.

## Critical Structural Errors (Cross-Entity)

These are not mere column gaps — they are **architectural errors** in the entity relationships themselves.

### Error 1: Role and Permission Missing `tenant_id`

| Field | Design ERD | Actual Role Model | Actual Permission Model | Impact |
|-------|-----------|------------------|----------------------|--------|
| tenant_id | uuid FK (scoped to tenant) | **MISSING** | **MISSING** | **CRITICAL** — No tenant isolation enforcement. Without tenant_id on Role/Permission, all roles and permissions are globally shared across tenants. A role created by Tenant A is visible to Tenant B. |

**Evidence:**
- Role model (`org-mgmt/impl/src/models/role.rs`): `id`, `org_id`, `name`, `description`, `created_at`, `updated_at` — **NO tenant_id**
- Permission model (`org-mgmt/impl/src/models/permission.rs`): `id`, `org_id`, `name`, `description`, `resource`, `action`, `created_at`, `updated_at` — **NO tenant_id**

The OpenAPI specs also confirm this gap:
- `AssignPrincipalRoleRequest` has `tenant_id: string NOT NULL` — the spec requires it
- `EffectiveRequest` has `tenant_id: string NOT NULL` — the spec requires it
- But the Role model itself lacks `tenant_id` to store it

**Fix:** ADD `tenant_id: VARCHAR(255)` to both Role and Permission entities. The migrator will add the column.

### Error 2: Application is a Child of Organization (Wrong Hierarchy)

| Field | Design ERD | Actual Application Model | Impact |
|-------|-----------|------------------------|--------|
| tenant_id | uuid FK (belongs to one tenant) | **MISSING** | Application is not partitioned by tenant |
| FK relationship | Application → Tenant (child of Tenant) | Application → Organization (child of Org) | **INVERTED HIERARCHY** |

**Evidence:**
- Application model: `id`, `org_id` (FK→organizations), `name`, `client_id`, `client_secret`, `redirect_uris`, `created_at`, `updated_at`
- Design ERD shows Application as a child of Tenant, peer of Organization
- Current model makes Application a child of Organization

**Impact:** This inverts the entire hierarchy. In the design:
- Tenant → Application → (User, Organization, APIKey, Session)

Current model has:
- Organization → Application → ???

**Fix:** Change Application's FK from `org_id` to `tenant_id`. The Application should be a child of Tenant, not Organization. Also add missing fields: `slug`, `platform`, `is_active`.

### Error 3: Organization Model Missing `tenant_id`

| Field | Design ERD | Actual Org Model | Impact |
|-------|-----------|-----------------|--------|
| tenant_id | uuid FK (TENANT BOUNDARY) | **MISSING** | **CRITICAL** — Organizations not partitioned by tenant. |

**Evidence:**
- Org model (`org-mgmt/impl/src/models/org.rs`): `id`, `name`, `tenant_id` (as VARCHAR(255) but NOT as a proper FK with UUID type), `status`, `created_at`, `updated_at`
- Wait — actually the Org model DOES have `tenant_id: String` at position 3. Let me re-examine...

Actually, looking at the Org model again:
```rust
pub id: uuid::Uuid,           // PK
pub name: String,             // VARCHAR(255)
pub tenant_id: String,        // VARCHAR(255) — EXISTS but as string, not UUID FK
pub status: String,           // VARCHAR(32)
pub created_at: chrono::DateTime<chrono::Utc>,
pub updated_at: chrono::DateTime<chrono::Utc>,
```

The `tenant_id` field EXISTS in the Org model, but:
- It's `VARCHAR(255)` not `UUID` type
- It lacks a `#[foreign_key]` attribute
- The design ERD shows it as `uuid FK "TENANT BOUNDARY"`

So the Org model is **P2 drift** (moderate) for tenant_id, but P0 for all other fields (slug, logo_url, domain, domains, domain_auto_join, domain_restrict, password_rotation, max_users, metadata, SAML settings, legacy_org_id).

## Summary Matrix (Updated)

| Entity | Wiki | Models | Design Cols | Actual Cols | Drift | Priority | Structural Error |
|--------|------|--------|-------------|-------------|-------|----------|-----------------|
| Tenant | Yes | **0** | 6 | 0 | **P0** | BLOCKER | NO ENTITY EXISTS |
| User | Yes | 2 | 21 | 10 | P0 | BLOCKER | Missing 11 fields |
| Organization | Yes | 1 | 23 | 6 | P0 | BLOCKER | Missing 17 fields |
| Role | Yes | 1 | 10 | 6 | P1 | HIGH | **MISSING tenant_id** (structural) |
| Permission | Yes | 1 | 5 | 8 | P2 | MEDIUM | **MISSING tenant_id** (structural) |
| Application | Yes | 1 | 10 | 8 | P1 | HIGH | **WRONG FK** (org_id vs tenant_id) |
| Session | Yes | 2 | 10-11 | 9-11 | P1 | HIGH | Missing tenant_id, revoked |
| APIKey | Yes | 3 | 10 | 12 | P2 | MEDIUM | Good |
| MFA Device | Yes | 2 | 8 | 7 | P1 | MEDIUM | Missing tenant_id, last_used |
| McpAgent | Yes | 1 | 12 | 7 | P1 | MEDIUM | Missing tenant_id |
| AuditLog | Yes | 2 | 10 | 8-10 | P1 | HIGH | Missing tenant_id |
| RolePermission | Yes | 1 | 2 | 4 | P2 | MEDIUM | Extra columns |
| UserOrganization | Yes | 1 | 7 | 7 | P2 | MEDIUM | Good |
| Webhook | Yes | 1 | 19 | 8 | P1 | MEDIUM | Missing delivery model |
| Impersonation | Yes | 1 | 6 | 6 | P1 | LOW | Missing tenant_id |
| Token | No | 1 | 8 | 8 | P1 | LOW | Missing tenant_id |

**Plus 18 undocumented models** (see below) that exist in code with no wiki coverage.

---
| Organization | Yes | 1 | 23 | 6 | P0 | BLOCKER |
| Role | Yes | 1 | 10 | 6 | P1 | HIGH |
| Session | Yes | 2 | 10-11 | 9-11 | P1 | HIGH |
| APIKey | Yes | 3 | 10 | 12 | P2 | MEDIUM |
| AuditLog | Yes | 2 | 10 | 8-10 | P1 | HIGH |
| MFA Device | Yes | 2 | 8 | 7 | P1 | MEDIUM |
| McpAgent | Yes | 1 | 12 | 7 | P1 | MEDIUM |
| Permission | Yes | 1 | 5 | 8 | P2 | MEDIUM |
| RolePermission | Yes | 1 | 2 | 4 | P2 | MEDIUM |
| UserOrganization | Yes | 1 | 7 | 7 | P2 | MEDIUM |
| Webhook | Yes | 1 | 19 | 8 | P1 | MEDIUM |
| Impersonation | Yes | 1 | 6 | 6 | P1 | LOW |
| Token | No | 1 | 8 | 8 | P1 | LOW |

**Plus 18 undocumented models** that exist in code but have no wiki or design coverage.

## Implementation Implications

Every story that touches these entities will need:

1. **Schema migrations** — ADD COLUMN for every missing field
2. **Model updates** — Add fields to the Lifeguard entities
3. **Regenerate migrations** — Run lifeguard_migrator
4. **Controller updates** — Handlers may need new field access
5. **Wiki updates** — Entity pages need verification and drift notes

## Design Document Recommendations

### D1: Merge the 2 User models
Both identity-login-service and identity-user-mgmt-service have identical User models. Pick one canonical location.

### D2: Move tenant_id from VARCHAR to UUID
All tenant_id columns are VARCHAR(255) but should be UUID FK to the (missing) Tenant entity.

### D3: Soft delete vs archived table pattern
APIKey uses separate ArchivedApiKey table instead of deleted_at. Decide on a consistent pattern.

### D4: Audit logging — central vs per-service
Two separate AuditEvent models suggest audit events are logged per-service. Design the central audit service pattern.

### D5: Add the missing foundational entities
Tenant (P0 blocker), and ensure all FK relationships are properly expressed.

### D6: Document the 18 undocumented models
Wiki pages for every model file that has no entity doc.
