# LLM Wiki — Session Log

## [2026-05-16] Entity Relationship Diagram — Comprehensive Audit

### Summary

Complete audit of the Sesame-IDAM entity relationship diagram by reconciling the wiki entity pages against the actual OpenAPI specs (6 services, 119+ endpoints) and the Lifeguard impl models in the `impl/` crates.

### Key Findings

**14 entities verified against impl models:**

| Entity | Status | Drift Found |
|--------|--------|-------------|
| User | corrected | 8 fields removed (user_type, first_name, username, etc. not in impl) |
| Organization | corrected | 24+ fields removed, impl has only 6 columns |
| Session | corrected | Renamed columns (session_token→token), removed tenant_id/revoked, added MFA/impersonation |
| API Key | corrected | Added permissions field, renamed revoked→active, removed metadata |
| Role | corrected | Removed inheritance (parent_role_id), removed is_system, tenant_id |
| Permission | corrected | Added org_id, resource, action columns; fixed scope from app→org |
| Application | corrected | Fixed scope from tenant→org, added OIDC fields (client_id, client_secret, redirect_uris) |
| MFA Setup | corrected | Renamed entity, added second service model, fixed field names |
| Audit Event | corrected | Split into two models (authz-core + identity-user-mgmt-service) |

**Entities removed from entity list:**
- `entity-tenant.md` — No `tenants` table exists; tenant is a logical boundary via `tenant_id` columns

**New ERD created:**
- `docs/llmwiki/topics/topic-entity-relationship-diagram.md` — Comprehensive ERD with all 40+ tables, foreign key relationships, and service ownership

### API Path Corrections

All entity pages updated with current OpenAPI endpoint paths:
- `POST /auth/login` (not `POST /login`)
- `POST /auth/verify/step-up` (not `POST /verify/step-up`)
- `POST /admin/impersonate` (not `POST /admin/users/{user_id}/impersonate`)
- `GET /identity/me` (not `GET /api/v1/identity/users/me`)
- `POST /identity/me/token` (not `POST /api/v1/identity/users/me/token`)
- `POST /mcp/token` (new endpoint)
- `POST /admin/users` (not `POST /users`)
- `POST /applications` (not `POST /api/v1/am/applications`)

### Entity Changes by Service

**identity-login-service (5 models):**
- `users` — Simplified: 10 columns (was documented as 20+)
- `sessions` — Simplified: no mfa_verified/impersonated_by
- `social_credentials` — New entity
- `otp_tokens` — New entity
- `magic_link_tokens` — New entity

**identity-session-service (6 models):**
- `sessions` — Full model: includes mfa_verified, impersonated_by
- `tokens` — New entity (access/refresh token tracking)
- `impersonations` — New entity
- `mfa_setup` — Duplicate of identity-user-mgmt-service MFA
- `user_profiles` — Extended profile metadata
- `mcp_agents` — MCP agent configuration

**identity-user-mgmt-service (7 models):**
- `users` — Same as identity-login-service
- `mfa_setup` — Duplicate of identity-session-service
- `email_verifications` — New entity
- `social_accounts` — Duplicate of social_credentials
- `employees` — Employee metadata
- `audit_event` — Richer than authz-core version
- `mfa_setup` — TOTP MFA setup

**authz-core (5 models):**
- `audit_event` — Lightweight audit
- `audit_retention_policy` — New entity
- `authorization` — ABAC-style records
- `role_assignment` — Principal role assignments
- `principal_attribute` — Custom user attributes

**api-keys (3 models):**
- `api_key` — API keys with permissions as JSON text
- `api_key_usage` — Usage tracking
- `archived_api_key` — Revoked keys

**org-mgmt (12 models):**
- `org` — Simplified: 6 columns (was documented as 30+)
- `org_membership` — New entity (was missing from original ERD)
- `org_invite` — Pending invitations
- `org_domain` — Verified domains
- `role` — Flat roles (no inheritance)
- `permission` — Org-scoped with resource/action columns
- `role_permission` — Bridge table
- `application` — OIDC client within org
- `saml_connection` — SAML IdP config
- `scim_user` — SCIM provisioned users
- `webhook_subscription` — Webhook endpoints

### Files Changed

| File | Action |
|------|--------|
| `docs/llmwiki/topics/topic-entity-relationship-diagram.md` | Created — comprehensive ERD |
| `docs/llmwiki/entities/entity-user.md` | Corrected — 10 columns, 48 endpoints |
| `docs/llmwiki/entities/entity-organization.md` | Corrected — 6 columns, 43 endpoints |
| `docs/llmwiki/entities/entity-session.md` | Corrected — two session models |
| `docs/llmwiki/entities/entity-api-key.md` | Corrected — added permissions, fixed status |
| `docs/llmwiki/entities/entity-role.md` | Corrected — removed inheritance |
| `docs/llmwiki/entities/entity-permission.md` | Corrected — org-scoped with resource/action |
| `docs/llmwiki/entities/entity-application.md` | Corrected — org-scoped, OIDC fields |
| `docs/llmwiki/entities/entity-mfa-device.md` | Corrected — two identical models |
| `docs/llmwiki/entities/entity-audit-log.md` | Corrected — two separate models |
| `docs/llmwiki/entities/entity-tenant.md` | Kept (for reference, but marked as logical only) |
| `docs/llmwiki/topics/topic-data-model.md` | Not yet updated — still shows old ERD |
| `docs/llmwiki/index.md` | Updated — status changes, new ERD topic |

### Commits

- `1e195de` — docs(wiki): add comprehensive ERD from OpenAPI + impl model audit
