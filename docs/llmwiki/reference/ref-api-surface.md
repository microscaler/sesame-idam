---
title: API Surface
status: partially-verified
updated: 2026-01-22
sources: [openapi/*/openapi.yaml]
---

# API Surface — Complete Reference

Built from actual OpenAPI specs. Total: 119 endpoints across 6 services.

## api-keys (Port :8103)

M2M API key lifecycle: creation, validation (personal + org-scoped), usage tracking, archiving.

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/` | POST | Create API key (M2M key / service account) |
| `/archived` | GET | Fetch archived (revoked/expired) API keys |
| `/archived/{key_id}` | GET | Fetch archived API key details |
| `/current` | GET | Fetch active API keys |
| `/import` | POST | Import API keys from external system |
| `/usage` | GET | Fetch API key usage |
| `/validate` | POST | Validate API key |
| `/validate/org` | POST | Validate organisation API key |
| `/validate/personal` | POST | Validate personal API key |
| `/{key_id}` | DELETE | Delete API key |

## authz-core (Port :8102)

Centralized authorization engine. Evaluates principal permissions at request time via /principal/effective.

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/authorize` | POST | Check if principal is allowed to perform action on resource |
| `/principal/effective` | POST | Get effective roles and permissions for principal |
| `/principals/attributes` | POST | Set attribute for principal (ABAC) |
| `/principals/roles` | DELETE | Revoke role from principal |
| `/principals/roles` | POST | Assign role to principal |

## identity-login-service (Port :8101)

Handles all authentication entry points: login, register, MFA, social OAuth, OTP flows, passwordless magic links, and signup validation.

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/forgot-password` | POST | Request password reset email |
| `/login` | POST | Login with password |
| `/login/dual-otp` | POST | Send OTPs to both email and phone simultaneously |
| `/login/email-otp` | POST | Send email OTP |
| `/login/magic-link` | POST | Send magic link for passwordless login |
| `/login/magic-link/verify` | POST | Verify magic link token and complete login |
| `/login/phone-magic-link` | POST | Send SMS magic link for passwordless login |
| `/login/phone-magic-link/verify` | POST | Verify SMS magic link token and complete login |
| `/login/phone-otp` | POST | Send phone SMS OTP |
| `/logout` | POST | Logout (revoke refresh token) |
| `/oauth/authorize` | GET | OAuth2 authorization endpoint |
| `/register` | POST | Register new user with email and password |
| `/reset-password` | POST | Confirm password reset with token |
| `/signup/validate` | GET | Validate signup eligibility |
| `/social/{provider}/callback` | POST | Exchange OAuth provider callback for tokens |
| `/social/{provider}/login` | GET | Initiate OAuth login with provider |
| `/token` | POST | Token endpoint (refresh, client_credentials, token_exchange RFC 8693) |
| `/verify/dual-otp` | POST | Verify dual OTP codes and complete login |
| `/verify/email-otp` | POST | Verify email OTP and complete login |
| `/verify/phone-otp` | POST | Verify phone SMS OTP and complete login |

## identity-session-service (Port :8105)

Manages user sessions, token refresh, OIDC discovery, step-up MFA, user impersonation, direct token issuance, and MCP authentication.

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/.well-known/jwks.json` | GET | JWKS for JWT verification |
| `/.well-known/openid-configuration` | GET | OIDC discovery |
| `/admin/users/{user_id}/impersonate` | POST | Impersonate user |
| `/admin/users/{user_id}/impersonate/restore` | POST | Restore admin session |
| `/api/v1/identity/users/me` | GET | Current user profile |
| `/api/v1/identity/users/me` | PATCH | Update current user profile |
| `/api/v1/identity/users/me/token` | POST | Issue access token |
| `/api/v1/identity/users/me/userinfo` | GET | User Info endpoint |
| `/api/v1/platform/mcp/agents` | GET | List agents |
| `/api/v1/platform/mcp/agents` | POST | Create agent |
| `/api/v1/platform/mcp/agents/{agent_id}` | DELETE | Delete agent |
| `/api/v1/platform/mcp/agents/{agent_id}` | GET | Get agent |
| `/mcp/token` | POST | Issue MCP auth token |
| `/mcp/token/validate` | POST | Validate MCP token |
| `/refresh` | POST | Refresh access token |
| `/verify/step-up` | POST | Step-up MFA verification |

## identity-user-mgmt-service (Port :8106)

User administration: CRUD, email/phone management, MFA, password resets, social linking, migrations, and password clearing for SSO-only mode.

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/oauth/logout` | POST | OAuth2 logout endpoint |
| `/users` | POST | Create user (idempotent by email) |
| `/users/email` | GET | Fetch user by email |
| `/users/migrate` | POST | Migrate user from external auth system |
| `/users/migrate-password` | POST | Bulk migrate passwords (hash+salt) |
| `/users/query` | GET | Paginated query for users with filters |
| `/users/username` | GET | Fetch user by username |
| `/users/{user_id}` | DELETE | Delete user (irreversible) |
| `/users/{user_id}/disable` | POST | Disable/block user |
| `/users/{user_id}/email` | PUT | Change user email |
| `/users/{user_id}/email/verify` | POST | Verify user email |
| `/users/{user_id}/employee` | GET | Fetch user in employee mode |
| `/users/{user_id}/enable` | POST | Enable/unblock user |
| `/users/{user_id}/logout-all-sessions` | POST | Logout all user sessions |
| `/users/{user_id}/magiclink` | POST | Send magic link for login |
| `/users/{user_id}/mfa/disable` | POST | Disable user 2FA |
| `/users/{user_id}/mfa/setup` | POST | Set up TOTP MFA |
| `/users/{user_id}/mfa/verify` | POST | Verify MFA code |
| `/users/{user_id}/password` | DELETE | Clear password (convert to SSO-only) |
| `/users/{user_id}/phone` | POST | Add phone number for user |
| `/users/{user_id}/phone/verify` | POST | Verify phone number |
| `/users/{user_id}/resend-email-confirmation` | POST | Resend email confirmation |
| `/users/{user_id}/social/link` | POST | Link social account to user |
| `/users/{user_id}/social/tokens` | GET | Fetch user's OAuth tokens from providers |
| `/users/{user_id}/social/tokens/{provider}/refresh` | GET | Fetch fresh token from provider |

## org-mgmt (Port :8104)

Organization lifecycle, SAML/SCIM SSO, membership management, application/role/permission RBAC, webhooks, API key invalidation, and SCIM user provisioning.

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/` | GET | Query for organisations |
| `/admin/users/{user_id}/invalidate-all-keys` | POST | Invalidate all API keys for user |
| `/api/v1/am/applications` | GET | List applications |
| `/api/v1/am/applications` | POST | Register application |
| `/api/v1/am/applications/{app_id}` | GET | Get application by id |
| `/api/v1/am/applications/{app_id}/permissions` | GET | List permissions for application |
| `/api/v1/am/applications/{app_id}/permissions` | POST | Create permission for application |
| `/api/v1/am/applications/{app_id}/roles` | GET | List roles for application |
| `/api/v1/am/applications/{app_id}/roles` | POST | Create role for application |
| `/api/v1/am/applications/{app_id}/roles/{role_id}` | GET | Get role by id |
| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | DELETE | Revoke permission from role |
| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | GET | Get permissions for role |
| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | POST | Assign permission to role |
| `/{org_id}` | DELETE | Delete organisation |
| `/{org_id}` | GET | Fetch organisation by ID |
| `/{org_id}` | PUT | Update organisation |
| `/{org_id}/users` | POST | Add user to organisation |
| `/{org_id}/allow-saml` | POST | Allow organisation to set up SAML SSO |
| `/{org_id}/users/{user_id}/role` | PATCH | Change user role in organisation |
| `/{org_id}/create-saml-link` | POST | Create SAML connection setup link |
| `/{org_id}/disallow-saml` | POST | Disallow organisation from using SAML SSO |
| `/{org_id}/domains` | PUT | Update organisation domain settings |
| `/{org_id}/enable-saml` | POST | Enable SAML connection for organisation |
| `/{org_id}/invite-user` | POST | Invite user to organisation by email |
| `/{org_id}/invite-user-by-id` | POST | Invite existing user to organisation |
| `/{org_id}/migrate-to-isolated` | POST | Migrate organisation to isolated SAML mode |
| `/{org_id}/oidc-metadata` | POST | Set OIDC IdP metadata for organisation |
| `/{org_id}/pending-invites` | DELETE | Revoke pending organisation invite |
| `/{org_id}/users` | DELETE | Remove user from organisation |
| `/{org_id}/role-mappings` | GET | Fetch custom role mappings for organisation |
| `/{org_id}/saml` | DELETE | Delete SAML connection |
| `/{org_id}/saml-metadata` | PUT | Set SAML IdP metadata for organisation |
| `/{org_id}/scim/groups` | GET | Fetch SCIM groups for organisation |
| `/{org_id}/scim/groups/{group_id}` | GET | Fetch a specific SCIM group |
| `/{org_id}/scim/users` | GET | List SCIM users in org |
| `/{org_id}/scim/users` | POST | Create SCIM user in org |
| `/{org_id}/scim/users/{user_id}` | DELETE | Delete SCIM user from org |
| `/{org_id}/scim/users/{user_id}` | PUT | Update SCIM user in org |
| `/{org_id}/subscribe-role-mapping` | PUT | Subscribe organisation to a role mapping |
| `/{org_id}/users` | GET | Fetch users in organisation |
| `/{org_id}/webhooks` | GET | Fetch organisation webhook subscriptions |
| `/{org_id}/webhooks/{subscription_id}` | DELETE | Delete webhook subscription |
| `/{org_id}/webhooks/{subscription_id}/test` | POST | Test webhook delivery |

## Endpoints by Tag

### APIKeys (10 endpoints)

- `DELETE /{key_id}`
- `GET /archived`
- `GET /archived/{key_id}`
- `GET /current`
- `GET /usage`
- `POST /`
- `POST /import`
- `POST /validate`
- `POST /validate/org`
- `POST /validate/personal`

### AccountSecurity (1 endpoints)

- `POST /admin/users/{user_id}/invalidate-all-keys`

### Applications (3 endpoints)

- `GET /api/v1/am/applications`
- `GET /api/v1/am/applications/{app_id}`
- `POST /api/v1/am/applications`

### AuthFlows (9 endpoints)

- `POST /login`
- `POST /login/dual-otp`
- `POST /login/email-otp`
- `POST /login/phone-otp`
- `POST /register`
- `POST /users/{user_id}/magiclink`
- `POST /verify/dual-otp`
- `POST /verify/email-otp`
- `POST /verify/phone-otp`

### Discovery (2 endpoints)

- `GET /.well-known/jwks.json`
- `GET /.well-known/openid-configuration`

### Identity (3 endpoints)

- `GET /api/v1/identity/users/me`
- `PATCH /api/v1/identity/users/me`
- `PUT /users/{user_id}/email`

### Impersonation (2 endpoints)

- `POST /admin/users/{user_id}/impersonate`
- `POST /admin/users/{user_id}/impersonate/restore`

### MCP (6 endpoints)

- `DELETE /api/v1/platform/mcp/agents/{agent_id}`
- `GET /api/v1/platform/mcp/agents`
- `GET /api/v1/platform/mcp/agents/{agent_id}`
- `POST /api/v1/platform/mcp/agents`
- `POST /mcp/token`
- `POST /mcp/token/validate`

### Membership (9 endpoints)

- `DELETE /{org_id}/pending-invites`
- `GET /{org_id}/role-mappings`
- `GET /{org_id}/users`
- `POST /{org_id}/users`
- `POST /{org_id}/invite-user`
- `POST /{org_id}/invite-user-by-id`
- `DELETE /{org_id}/users`
- `PUT /{org_id}/subscribe-role-mapping`
- `PATCH /{org_id}/users/{user_id}/role`

### Organizations (5 endpoints)

- `DELETE /{org_id}`
- `GET /`
- `GET /{org_id}`
- `PUT /{org_id}`
- `PUT /{org_id}/domains`

### PasswordReset (2 endpoints)

- `POST /forgot-password`
- `POST /reset-password`

### PasswordSecurity (10 endpoints)

- `DELETE /users/{user_id}/password`
- `POST /users/{user_id}/disable`
- `POST /users/{user_id}/email/verify`
- `POST /users/{user_id}/enable`
- `POST /users/{user_id}/mfa/disable`
- `POST /users/{user_id}/mfa/setup`
- `POST /users/{user_id}/mfa/verify`
- `POST /users/{user_id}/phone`
- `POST /users/{user_id}/phone/verify`
- `POST /users/{user_id}/resend-email-confirmation`

### Passwordless (4 endpoints)

- `POST /login/magic-link`
- `POST /login/magic-link/verify`
- `POST /login/phone-magic-link`
- `POST /login/phone-magic-link/verify`

### Permissions (2 endpoints)

- `GET /api/v1/am/applications/{app_id}/permissions`
- `POST /api/v1/am/applications/{app_id}/permissions`

### Principal (5 endpoints)

- `DELETE /principals/roles`
- `POST /authorize`
- `POST /principal/effective`
- `POST /principals/attributes`
- `POST /principals/roles`

### Roles (6 endpoints)

- `DELETE /api/v1/am/applications/{app_id}/roles/{role_id}/permissions`
- `GET /api/v1/am/applications/{app_id}/roles`
- `GET /api/v1/am/applications/{app_id}/roles/{role_id}`
- `GET /api/v1/am/applications/{app_id}/roles/{role_id}/permissions`
- `POST /api/v1/am/applications/{app_id}/roles`
- `POST /api/v1/am/applications/{app_id}/roles/{role_id}/permissions`

### SCIM (4 endpoints)

- `DELETE /{org_id}/scim/users/{user_id}`
- `GET /{org_id}/scim/users`
- `POST /{org_id}/scim/users`
- `PUT /{org_id}/scim/users/{user_id}`

### SSO (10 endpoints)

- `DELETE /{org_id}/saml`
- `GET /{org_id}/scim/groups`
- `GET /{org_id}/scim/groups/{group_id}`
- `POST /{org_id}/allow-saml`
- `POST /{org_id}/create-saml-link`
- `POST /{org_id}/disallow-saml`
- `POST /{org_id}/enable-saml`
- `POST /{org_id}/migrate-to-isolated`
- `POST /{org_id}/oidc-metadata`
- `PUT /{org_id}/saml-metadata`

### Sessions (7 endpoints)

- `GET /api/v1/identity/users/me/userinfo`
- `GET /oauth/authorize`
- `POST /logout`
- `POST /oauth/logout`
- `POST /refresh`
- `POST /token`
- `POST /users/{user_id}/logout-all-sessions`

### Signup (1 endpoints)

- `GET /signup/validate`

### SocialLogin (5 endpoints)

- `GET /social/{provider}/login`
- `GET /users/{user_id}/social/tokens`
- `GET /users/{user_id}/social/tokens/{provider}/refresh`
- `POST /social/{provider}/callback`
- `POST /users/{user_id}/social/link`

### StepUp (1 endpoints)

- `POST /verify/step-up`

### TokenIssuance (1 endpoints)

- `POST /api/v1/identity/users/me/token`

### UserMigration (2 endpoints)

- `POST /users/migrate`
- `POST /users/migrate-password`

### Users (6 endpoints)

- `DELETE /users/{user_id}`
- `GET /users/email`
- `GET /users/query`
- `GET /users/username`
- `GET /users/{user_id}/employee`
- `POST /users`

### Webhooks (3 endpoints)

- `DELETE /{org_id}/webhooks/{subscription_id}`
- `GET /{org_id}/webhooks`
- `POST /{org_id}/webhooks/{subscription_id}/test`

## Status

> This page is **partially-verified** — built from OpenAPI specs, not runtime code.
> Next session should verify implementations match specs.

## Code Anchors

- `openapi/*/openapi.yaml` — Source of truth (see per-service `README.md`)
- `microservices/idam/*/impl/src/` — Handler implementations
