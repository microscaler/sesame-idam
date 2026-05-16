---
title: API Surface
status: verified
updated: 2026-05-16
sources: [openapi/*/openapi.yaml]
---

# API Surface — Complete Reference

Built from actual OpenAPI specs. Total: 133 endpoints across 6 services.

## api-keys

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/api-keys` | POST | Create API key (M2M key / service account) |
| `/api-keys/archived` | GET | Fetch archived (revoked/expired) API keys |
| `/api-keys/archived/{key_id}` | GET | Fetch archived API key details |
| `/api-keys/current` | GET | Fetch active API keys |
| `/api-keys/import` | POST | Import API keys from external system |
| `/api-keys/usage` | GET | Fetch API key usage |
| `/api-keys/validate` | POST | Validate API key |
| `/api-keys/validate/org` | POST | DEPRECATED: Validate organisation API key |
| `/api-keys/validate/personal` | POST | DEPRECATED: Validate personal API key |
| `/api-keys/{key_id}` | PUT | Update API key metadata |
| `/api-keys/{key_id}` | DELETE | Delete API key |

## authz-core

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/authz/audit/events` | GET | List audit events (simple query) |
| `/authz/audit/events` | POST | Search audit events |
| `/authz/audit/events/stats` | POST | Get audit event statistics |
| `/authz/audit/events/{id}` | GET | Get single audit event |
| `/authz/audit/export` | POST | Request audit event export |
| `/authz/audit/export/{export_id}` | GET | Check export status |
| `/authz/audit/retention` | GET | List retention policies |
| `/authz/audit/retention` | POST | Create retention policy |
| `/authz/audit/retention/{id}` | PATCH | Update retention policy |
| `/authz/audit/retention/{id}` | DELETE | Delete retention policy |
| `/authz/authorize` | POST | Check if principal is allowed to perform action on resource |
| `/authz/principals/attributes` | POST | Set attribute for principal (ABAC) |
| `/authz/principals/effective` | POST | Get effective roles and permissions for principal |
| `/authz/principals/roles` | POST | Assign role to principal |
| `/authz/principals/roles` | DELETE | Revoke role from principal |

## identity-login-service

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/auth/login` | POST | Login with password |
| `/auth/login/dual-otp` | POST | Send OTPs to both email and phone simultaneously |
| `/auth/login/email-otp` | POST | Send email OTP |
| `/auth/login/magic-link` | POST | Send magic link for passwordless login |
| `/auth/login/magic-link/verify` | POST | Verify magic link token and complete login |
| `/auth/login/phone-magic-link` | POST | Send SMS magic link for passwordless login |
| `/auth/login/phone-magic-link/verify` | POST | Verify SMS magic link token and complete login |
| `/auth/login/phone-otp` | POST | Send phone SMS OTP |
| `/auth/logout` | POST | Logout (revoke refresh token) |
| `/auth/password/forgot` | POST | Request password reset email |
| `/auth/password/reset` | POST | Confirm password reset with token |
| `/auth/register` | POST | Register new user with email and password |
| `/auth/signup/validate` | GET | Validate signup eligibility |
| `/auth/social/{provider}/callback` | POST | Exchange OAuth provider callback for tokens |
| `/auth/social/{provider}/login` | GET | Initiate OAuth login with provider |
| `/auth/token` | POST | Token endpoint (refresh, client_credentials, token_exchange RFC 8693) |
| `/auth/verify/dual-otp` | POST | Verify dual OTP codes and complete login |
| `/auth/verify/email-otp` | POST | Verify email OTP and complete login |
| `/auth/verify/phone-otp` | POST | Verify phone SMS OTP and complete login |
| `/oauth/authorize` | GET | OAuth2 authorization endpoint |

## identity-session-service

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/.well-known/jwks.json` | GET | JWKS for JWT verification |
| `/.well-known/openid-configuration` | GET | OIDC discovery |
| `/admin/impersonate` | POST | Impersonate user |
| `/admin/impersonate/restore` | POST | Restore admin session |
| `/auth/verify/step-up` | POST | Step-up MFA verification |
| `/identity/me` | GET | Current user profile |
| `/identity/me` | PATCH | Update current user profile |
| `/identity/me/token` | POST | Issue access token |
| `/identity/userinfo` | GET | User Info endpoint |
| `/mcp/agents` | GET | List agents |
| `/mcp/agents` | POST | Create agent |
| `/mcp/agents/{agent_id}` | GET | Get agent |
| `/mcp/agents/{agent_id}` | DELETE | Delete agent |
| `/mcp/token` | POST | Issue MCP auth token |
| `/mcp/token/validate` | POST | Validate MCP token |
| `/session/refresh` | POST | Refresh access token |

## identity-user-mgmt-service

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/admin/audit/events` | POST | Get user-specific audit events |
| `/admin/audit/users/{user_id}/events/compliance-export` | POST | Export user's audit events (GDPR) |
| `/admin/audit/users/{user_id}/events/count` | GET | Get user event count |
| `/admin/users` | POST | Create user (idempotent by email) |
| `/admin/users/email` | GET | Fetch user by email |
| `/admin/users/migrate` | POST | Migrate user from external auth system |
| `/admin/users/migrate-password` | POST | Bulk migrate passwords (hash+salt) |
| `/admin/users/query` | GET | Paginated query for users with filters |
| `/admin/users/username` | GET | Fetch user by username |
| `/admin/users/{user_id}` | DELETE | Delete user (irreversible) |
| `/admin/users/{user_id}/disable` | POST | Disable/block user |
| `/admin/users/{user_id}/email` | PUT | Change user email |
| `/admin/users/{user_id}/email/verify` | POST | Verify user email |
| `/admin/users/{user_id}/employee` | GET | Fetch user in employee mode |
| `/admin/users/{user_id}/enable` | POST | Enable/unblock user |
| `/admin/users/{user_id}/logout-all-sessions` | POST | Logout all user sessions |
| `/admin/users/{user_id}/magiclink` | POST | Send magic link for login |
| `/admin/users/{user_id}/mfa/disable` | POST | Disable user 2FA |
| `/admin/users/{user_id}/mfa/setup` | POST | Set up TOTP MFA |
| `/admin/users/{user_id}/mfa/verify` | POST | Verify MFA code |
| `/admin/users/{user_id}/password` | DELETE | Clear password (convert to SSO-only) |
| `/admin/users/{user_id}/phone` | POST | Add phone number for user |
| `/admin/users/{user_id}/phone/verify` | POST | Verify phone number |
| `/admin/users/{user_id}/resend-email-confirmation` | POST | Resend email confirmation |
| `/admin/users/{user_id}/social/link` | POST | Link social account to user |
| `/admin/users/{user_id}/social/tokens` | GET | Fetch user's OAuth tokens from providers |
| `/admin/users/{user_id}/social/tokens/{provider}/refresh` | GET | Fetch fresh token from provider |
| `/oauth/logout` | POST | OAuth2 logout endpoint |

## org-mgmt

| Endpoint | Method | Summary |
|----------|--------|---------|
| `/applications` | GET | List applications |
| `/applications` | POST | Register application |
| `/applications/{app_id}` | GET | Get application by id |
| `/applications/{app_id}/permissions` | GET | List permissions for application |
| `/applications/{app_id}/permissions` | POST | Create permission for application |
| `/applications/{app_id}/roles` | GET | List roles for application |
| `/applications/{app_id}/roles` | POST | Create role for application |
| `/applications/{app_id}/roles/{role_id}` | GET | Get role by id |
| `/applications/{app_id}/roles/{role_id}/permissions` | GET | Get permissions for role |
| `/applications/{app_id}/roles/{role_id}/permissions` | POST | Assign permission to role |
| `/applications/{app_id}/roles/{role_id}/permissions` | DELETE | Revoke permission from role |
| `/organizations` | GET | Query for organisations |
| `/organizations/admin/users/{user_id}/invalidate-all-keys` | POST | Invalidate all API keys for user |
| `/organizations/{org_id}` | GET | Fetch organisation by ID |
| `/organizations/{org_id}` | PUT | Update organisation |
| `/organizations/{org_id}` | DELETE | Delete organisation |
| `/organizations/{org_id}/domains` | PUT | Update organisation domain settings |
| `/organizations/{org_id}/invitations` | POST | Invite user to organisation by email |
| `/organizations/{org_id}/invitations/by-id` | POST | Invite existing user to organisation |
| `/organizations/{org_id}/migrate-to-isolated` | POST | Migrate organisation to isolated SAML mode |
| `/organizations/{org_id}/oidc-metadata` | POST | Set OIDC IdP metadata for organisation |
| `/organizations/{org_id}/pending-invitations` | DELETE | Revoke pending organisation invite |
| `/organizations/{org_id}/role-mappings` | GET | Fetch custom role mappings for organisation |
| `/organizations/{org_id}/role-mappings/subscribe` | PUT | Subscribe organisation to a role mapping |
| `/organizations/{org_id}/scim/groups` | GET | Fetch SCIM groups for organisation |
| `/organizations/{org_id}/scim/groups/{group_id}` | GET | Fetch a specific SCIM group |
| `/organizations/{org_id}/scim/users` | GET | List SCIM users in org |
| `/organizations/{org_id}/scim/users` | POST | Create SCIM user in org |
| `/organizations/{org_id}/scim/users/{user_id}` | PUT | Update SCIM user in org |
| `/organizations/{org_id}/scim/users/{user_id}` | DELETE | Delete SCIM user from org |
| `/organizations/{org_id}/users` | GET | Fetch users in organisation |
| `/organizations/{org_id}/users` | POST | Add user to organisation |
| `/organizations/{org_id}/users/{user_id}` | DELETE | Remove user from organisation |
| `/organizations/{org_id}/users/{user_id}/role` | PATCH | Change user role in organisation |
| `/organizations/{org_id}/webhooks` | GET | Fetch organisation webhook subscriptions |
| `/organizations/{org_id}/webhooks/{subscription_id}` | DELETE | Delete webhook subscription |
| `/organizations/{org_id}/webhooks/{subscription_id}/test` | POST | Test webhook delivery |
| `/sso/saml` | DELETE | Delete SAML connection |
| `/sso/saml/allow` | POST | Allow organisation to set up SAML SSO |
| `/sso/saml/disable` | POST | Disallow organisation from using SAML SSO |
| `/sso/saml/enable` | POST | Enable SAML connection for organisation |
| `/sso/saml/link` | POST | Create SAML connection setup link |
| `/sso/saml/metadata` | PUT | Set SAML IdP metadata for organisation |

## Endpoints by Tag

### APIKeys (11 endpoints)

- `DELETE /api-keys/{key_id}` (api-keys)
- `GET /api-keys/archived` (api-keys)
- `GET /api-keys/archived/{key_id}` (api-keys)
- `GET /api-keys/current` (api-keys)
- `GET /api-keys/usage` (api-keys)
- `POST /api-keys` (api-keys)
- `POST /api-keys/import` (api-keys)
- `POST /api-keys/validate` (api-keys)
- `POST /api-keys/validate/org` (api-keys)
- `POST /api-keys/validate/personal` (api-keys)
- `PUT /api-keys/{key_id}` (api-keys)

### AccountSecurity (1 endpoints)

- `POST /organizations/admin/users/{user_id}/invalidate-all-keys` (org-mgmt)

### Applications (3 endpoints)

- `GET /applications` (org-mgmt)
- `GET /applications/{app_id}` (org-mgmt)
- `POST /applications` (org-mgmt)

### Audit (10 endpoints)

- `DELETE /authz/audit/retention/{id}` (authz-core)
- `GET /authz/audit/events` (authz-core)
- `GET /authz/audit/events/{id}` (authz-core)
- `GET /authz/audit/export/{export_id}` (authz-core)
- `GET /authz/audit/retention` (authz-core)
- `PATCH /authz/audit/retention/{id}` (authz-core)
- `POST /authz/audit/events` (authz-core)
- `POST /authz/audit/events/stats` (authz-core)
- `POST /authz/audit/export` (authz-core)
- `POST /authz/audit/retention` (authz-core)

### AuthFlows (9 endpoints)

- `POST /auth/login` (identity-login-service)
- `POST /auth/login/dual-otp` (identity-login-service)
- `POST /auth/login/email-otp` (identity-login-service)
- `POST /auth/login/phone-otp` (identity-login-service)
- `POST /auth/register` (identity-login-service)
- `POST /auth/verify/dual-otp` (identity-login-service)
- `POST /auth/verify/email-otp` (identity-login-service)
- `POST /auth/verify/phone-otp` (identity-login-service)
- `POST /admin/users/{user_id}/magiclink` (identity-user-mgmt-service)

### Discovery (2 endpoints)

- `GET /.well-known/jwks.json` (identity-session-service)
- `GET /.well-known/openid-configuration` (identity-session-service)

### Identity (3 endpoints)

- `GET /identity/me` (identity-session-service)
- `PATCH /identity/me` (identity-session-service)
- `PUT /admin/users/{user_id}/email` (identity-user-mgmt-service)

### Impersonation (2 endpoints)

- `POST /admin/impersonate` (identity-session-service)
- `POST /admin/impersonate/restore` (identity-session-service)

### MCP (6 endpoints)

- `DELETE /mcp/agents/{agent_id}` (identity-session-service)
- `GET /mcp/agents` (identity-session-service)
- `GET /mcp/agents/{agent_id}` (identity-session-service)
- `POST /mcp/agents` (identity-session-service)
- `POST /mcp/token` (identity-session-service)
- `POST /mcp/token/validate` (identity-session-service)

### Membership (9 endpoints)

- `DELETE /organizations/{org_id}/pending-invitations` (org-mgmt)
- `DELETE /organizations/{org_id}/users/{user_id}` (org-mgmt)
- `GET /organizations/{org_id}/role-mappings` (org-mgmt)
- `GET /organizations/{org_id}/users` (org-mgmt)
- `PATCH /organizations/{org_id}/users/{user_id}/role` (org-mgmt)
- `POST /organizations/{org_id}/invitations` (org-mgmt)
- `POST /organizations/{org_id}/invitations/by-id` (org-mgmt)
- `POST /organizations/{org_id}/users` (org-mgmt)
- `PUT /organizations/{org_id}/role-mappings/subscribe` (org-mgmt)

### Organizations (5 endpoints)

- `DELETE /organizations/{org_id}` (org-mgmt)
- `GET /organizations` (org-mgmt)
- `GET /organizations/{org_id}` (org-mgmt)
- `PUT /organizations/{org_id}` (org-mgmt)
- `PUT /organizations/{org_id}/domains` (org-mgmt)

### PasswordReset (2 endpoints)

- `POST /auth/password/forgot` (identity-login-service)
- `POST /auth/password/reset` (identity-login-service)

### PasswordSecurity (10 endpoints)

- `DELETE /admin/users/{user_id}/password` (identity-user-mgmt-service)
- `POST /admin/users/{user_id}/disable` (identity-user-mgmt-service)
- `POST /admin/users/{user_id}/email/verify` (identity-user-mgmt-service)
- `POST /admin/users/{user_id}/enable` (identity-user-mgmt-service)
- `POST /admin/users/{user_id}/mfa/disable` (identity-user-mgmt-service)
- `POST /admin/users/{user_id}/mfa/setup` (identity-user-mgmt-service)
- `POST /admin/users/{user_id}/mfa/verify` (identity-user-mgmt-service)
- `POST /admin/users/{user_id}/phone` (identity-user-mgmt-service)
- `POST /admin/users/{user_id}/phone/verify` (identity-user-mgmt-service)
- `POST /admin/users/{user_id}/resend-email-confirmation` (identity-user-mgmt-service)

### Passwordless (4 endpoints)

- `POST /auth/login/magic-link` (identity-login-service)
- `POST /auth/login/magic-link/verify` (identity-login-service)
- `POST /auth/login/phone-magic-link` (identity-login-service)
- `POST /auth/login/phone-magic-link/verify` (identity-login-service)

### Permissions (2 endpoints)

- `GET /applications/{app_id}/permissions` (org-mgmt)
- `POST /applications/{app_id}/permissions` (org-mgmt)

### Principal (5 endpoints)

- `DELETE /authz/principals/roles` (authz-core)
- `POST /authz/authorize` (authz-core)
- `POST /authz/principals/attributes` (authz-core)
- `POST /authz/principals/effective` (authz-core)
- `POST /authz/principals/roles` (authz-core)

### Roles (6 endpoints)

- `DELETE /applications/{app_id}/roles/{role_id}/permissions` (org-mgmt)
- `GET /applications/{app_id}/roles` (org-mgmt)
- `GET /applications/{app_id}/roles/{role_id}` (org-mgmt)
- `GET /applications/{app_id}/roles/{role_id}/permissions` (org-mgmt)
- `POST /applications/{app_id}/roles` (org-mgmt)
- `POST /applications/{app_id}/roles/{role_id}/permissions` (org-mgmt)

### SCIM (4 endpoints)

- `DELETE /organizations/{org_id}/scim/users/{user_id}` (org-mgmt)
- `GET /organizations/{org_id}/scim/users` (org-mgmt)
- `POST /organizations/{org_id}/scim/users` (org-mgmt)
- `PUT /organizations/{org_id}/scim/users/{user_id}` (org-mgmt)

### SSO (10 endpoints)

- `DELETE /sso/saml` (org-mgmt)
- `GET /organizations/{org_id}/scim/groups` (org-mgmt)
- `GET /organizations/{org_id}/scim/groups/{group_id}` (org-mgmt)
- `POST /organizations/{org_id}/migrate-to-isolated` (org-mgmt)
- `POST /organizations/{org_id}/oidc-metadata` (org-mgmt)
- `POST /sso/saml/allow` (org-mgmt)
- `POST /sso/saml/disable` (org-mgmt)
- `POST /sso/saml/enable` (org-mgmt)
- `POST /sso/saml/link` (org-mgmt)
- `PUT /sso/saml/metadata` (org-mgmt)

### Sessions (7 endpoints)

- `GET /oauth/authorize` (identity-login-service)
- `POST /auth/logout` (identity-login-service)
- `POST /auth/token` (identity-login-service)
- `GET /identity/userinfo` (identity-session-service)
- `POST /session/refresh` (identity-session-service)
- `POST /admin/users/{user_id}/logout-all-sessions` (identity-user-mgmt-service)
- `POST /oauth/logout` (identity-user-mgmt-service)

### Signup (1 endpoints)

- `GET /auth/signup/validate` (identity-login-service)

### SocialLogin (5 endpoints)

- `GET /auth/social/{provider}/login` (identity-login-service)
- `POST /auth/social/{provider}/callback` (identity-login-service)
- `GET /admin/users/{user_id}/social/tokens` (identity-user-mgmt-service)
- `GET /admin/users/{user_id}/social/tokens/{provider}/refresh` (identity-user-mgmt-service)
- `POST /admin/users/{user_id}/social/link` (identity-user-mgmt-service)

### StepUp (1 endpoints)

- `POST /auth/verify/step-up` (identity-session-service)

### TokenIssuance (1 endpoints)

- `POST /identity/me/token` (identity-session-service)

### UserAudit (3 endpoints)

- `GET /admin/audit/users/{user_id}/events/count` (identity-user-mgmt-service)
- `POST /admin/audit/events` (identity-user-mgmt-service)
- `POST /admin/audit/users/{user_id}/events/compliance-export` (identity-user-mgmt-service)

### UserMigration (2 endpoints)

- `POST /admin/users/migrate` (identity-user-mgmt-service)
- `POST /admin/users/migrate-password` (identity-user-mgmt-service)

### Users (6 endpoints)

- `DELETE /admin/users/{user_id}` (identity-user-mgmt-service)
- `GET /admin/users/email` (identity-user-mgmt-service)
- `GET /admin/users/query` (identity-user-mgmt-service)
- `GET /admin/users/username` (identity-user-mgmt-service)
- `GET /admin/users/{user_id}/employee` (identity-user-mgmt-service)
- `POST /admin/users` (identity-user-mgmt-service)

### Webhooks (3 endpoints)

- `DELETE /organizations/{org_id}/webhooks/{subscription_id}` (org-mgmt)
- `GET /organizations/{org_id}/webhooks` (org-mgmt)
- `POST /organizations/{org_id}/webhooks/{subscription_id}/test` (org-mgmt)

## Status

> This page is **verified** — regenerated from OpenAPI specs on 2026-05-16.
