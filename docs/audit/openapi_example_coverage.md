# OpenAPI Example Coverage Audit

> **Date:** 2026-05-10
> **Scope:** All 6 Sesame-IDAM OpenAPI specs, 120 endpoints, 179 schemas
> **Purpose:** Identify gaps in `example` fields before backend implementation

## Executive Summary

| Metric | Count | Percentage |
|--------|-------|------------|
| Total endpoints | 120 | 100% |
| Endpoints with ANY examples | 77 | 64.2% |
| Endpoints with NO examples | 43 | **35.8%** |
| Endpoints with request examples | 0 | **0.0%** |
| Endpoints with response examples | 77 | 64.2% |
| Total schemas | 179 | 100% |
| Schemas with examples | 36 | 20.1% |
| Schemas WITHOUT examples | 143 | **79.9%** |

## Critical Findings

### 1. Zero Request Body Examples (CRITICAL)

- **No endpoint across any spec has request body examples**
- All 120 endpoints: 0/120 with request body examples
- This is the single largest gap — frontend clients cannot validate payload shapes
- Every POST, PUT, PATCH endpoint is missing request examples

### 2. 35.8% of Endpoints Have Zero Examples

- 43 endpoints have neither request nor response examples
- These are mostly in `identity-login-service` (20/20 = 100% missing)
- Login, register, magic-link, OTP flows have NO examples anywhere

### 3. Schema-Level Coverage is Low

- Only 20.1% of schemas (36/179) have examples
- 143 schemas are completely bare — no examples at all
- Of the 36 with examples: 21 have root-level examples, 15 have property-level
- Zero schemas have array item-level examples

### 4. Response Example Quality

- All 96 response examples are error responses (400, 401, 403, 404)
- Zero success response examples (200, 201, 204) exist anywhere
- Error response examples are consistent but repetitive

---

## Per-Service Breakdown

### identity-login-service

**Spec file:** `openapi/idam/identity-login-service/openapi.yaml`
**Schemas defined:** 29 | **Endpoints:** 20

| Category | Count | % |
|----------|-------|---|
| Schemas with examples | 6/29 | 20.7% |
| Schemas WITHOUT examples | 23/29 | 79.3% |
| Endpoints with request examples | 0/20 | 0.0% |
| Endpoints with response examples | 0/20 | 0.0% |
| Endpoints with ANY examples | 0/20 | 0.0% |
| Endpoints with NO examples | 20/20 | 100.0% |

**Schemas WITH examples (6):**
  - `PendingEmailVerificationResponse`: property-level (message)
  - `MfaRequiredResponse`: property-level (mfa_required)
  - `DualOTPResponse`: property-level (success, email_verified, phone_verified, both_verified, message)
  - `DualOTPPartialResponse`: property-level (success, message)
  - `MagicLinkResponse`: property-level (magic_link_sent, expires_in)
  - `SmsMagicLinkResponse`: property-level (magic_link_sent, expires_in)
**Schemas WITHOUT examples (23) — NEED EXAMPLES:**
  - `ErrorResponse`
  - `LoginRequest`
  - `TokenResponse`
  - `EmailOTPRequest`
  - `EmailOTPVerifyRequest`
  - `PhoneOTPRequest`
  - `PhoneOTPVerifyRequest`
  - `DualOTPRequest`
  - `DualOTPVerifyRequest`
  - `DualOTPCompleteResponse`
  - `RegisterRequest`
  - `TokenRequest`
  - `ForgotPasswordRequest`
  - `ResetPasswordRequest`
  - `LogoutRequest`
  - `SocialCallbackRequest`
  - `SocialLoginResponse`
  - `SignupValidationRequest`
  - `SignupValidationResponse`
  - `MagicLinkRequest`
  - `MagicLinkVerifyRequest`
  - `SmsMagicLinkRequest`
  - `SmsMagicLinkVerifyRequest`

### identity-session-service

**Spec file:** `openapi/idam/identity-session-service/openapi.yaml`
**Schemas defined:** 59 | **Endpoints:** 16

| Category | Count | % |
|----------|-------|---|
| Schemas with examples | 16/59 | 27.1% |
| Schemas WITHOUT examples | 43/59 | 72.9% |
| Endpoints with request examples | 0/16 | 0.0% |
| Endpoints with response examples | 13/16 | 81.2% |
| Endpoints with ANY examples | 13/16 | 81.2% |
| Endpoints with NO examples | 3/16 | 18.8% |

**Schemas WITH examples (16):**
  - `CreateUserRequest`: root object (9 keys: email, email_confirmed, send_email_confirmation, first_name, last_name, username, send_welcome_email, extra_properties... (+1))
  - `DualOTPPartialResponse`: property-level (success, message)
  - `DualOTPResponse`: property-level (success, email_verified, phone_verified, both_verified, message)
  - `JWKS`: property-level (keys)
  - `LoginRequest`: root object (3 keys: email, password, organization_id)
  - `McpTokenResponse`: property-level (token_type)
  - `MfaRequiredResponse`: root object (3 keys: session_id, mfa_required, mfa_type)
  - `MfaSetupRequest`: root object (2 keys: password, name)
  - `MfaSetupResponse`: root object (3 keys: provisioning_uri, secret, user_id)
  - `MfaVerifyRequest`: root object (2 keys: code, session_id)
  - `OpenIDConfiguration`: root object (12 keys: issuer, authorization_endpoint, token_endpoint, jwks_uri, userinfo_endpoint, scopes_supported, response_types_supported, response_modes_supported... (+4))
  - `PhoneVerificationRequest`: root object (2 keys: phone_number, code)
  - `UpdateUserProfileRequest`: root object (2 keys: first_name, last_name)
  - `UpdateUserRequest`: root object (3 keys: first_name, last_name, locked)
  - `User`: root object (19 keys: user_id, email, email_confirmed, first_name, last_name, username, picture_url, properties... (+11))
  - `UserProfile`: root object (16 keys: sub, email, email_verified, name, preferred_username, first_name, last_name, username... (+8))
**Schemas WITHOUT examples (43) — NEED EXAMPLES:**
  - `DualOTPCompleteResponse`
  - `DualOTPRequest`
  - `DualOTPVerifyRequest`
  - `EmailOTPRequest`
  - `EmailOTPVerifyRequest`
  - `EmployeeResponse`
  - `Error`
  - `ErrorResponse`
  - `ForgotPasswordRequest`
  - `ImpersonateRequest`
  - `ImpersonateResponse`
  - `ImpersonateRestoreRequest`
  - `LinkSocialAccountRequest`
  - `LogoutRequest`
  - `McpAgent`
  - `McpAgentCreateResponse`
  - `McpAgentListResponse`
  - `McpTokenRequest`
  - `McpValidateRequest`
  - `McpValidateResponse`
  - `MfaFactor`
  - `MigratePasswordRequest`
  - `MigrateUserRequest`
  - `OAuthLogoutRequest`
  - `OAuthTokenResponse`
  - `PhoneNumberRequest`
  - `PhoneOTPRequest`
  - `PhoneOTPVerifyRequest`
  - `RefreshRequest`
  - `RegisterRequest`
  - `ResetPasswordRequest`
  - `SocialCallbackRequest`
  - `SocialLoginResponse`
  - `StepUpRequest`
  - `StepUpResponse`
  - `TokenIssuanceRequest`
  - `TokenListResponse`
  - `TokenRequest`
  - `TokenResponse`
  - `UpdateEmailRequest`
  - `UpdatePasswordRequest`
  - `UserQueryItem`
  - `UserQueryResponse`

### identity-user-mgmt-service

**Spec file:** `openapi/idam/identity-user-mgmt-service/openapi.yaml`
**Schemas defined:** 23 | **Endpoints:** 25

| Category | Count | % |
|----------|-------|---|
| Schemas with examples | 6/23 | 26.1% |
| Schemas WITHOUT examples | 17/23 | 73.9% |
| Endpoints with request examples | 0/25 | 0.0% |
| Endpoints with response examples | 21/25 | 84.0% |
| Endpoints with ANY examples | 21/25 | 84.0% |
| Endpoints with NO examples | 4/25 | 16.0% |

**Schemas WITH examples (6):**
  - `CreateUserRequest`: root object (9 keys: email, email_confirmed, send_email_confirmation, first_name, last_name, username, send_welcome_email, extra_properties... (+1))
  - `MfaSetupRequest`: root object (2 keys: password, name)
  - `MfaSetupResponse`: root object (3 keys: provisioning_uri, secret, user_id)
  - `MfaVerifyRequest`: root object (2 keys: code, session_id)
  - `PhoneVerificationRequest`: root object (2 keys: phone_number, code)
  - `ErrorResponse`: root object (2 keys: error, error_description)
**Schemas WITHOUT examples (17) — NEED EXAMPLES:**
  - `Error`
  - `EmployeeResponse`
  - `LinkSocialAccountRequest`
  - `MigrateUserRequest`
  - `MigratePasswordRequest`
  - `OAuthLogoutRequest`
  - `OAuthTokenResponse`
  - `PhoneNumberRequest`
  - `TokenListResponse`
  - `TokenResponse`
  - `UserQueryItem`
  - `UserQueryResponse`
  - `UpdateUserRequest`
  - `UpdateEmailRequest`
  - `UpdatePasswordRequest`
  - `User`
  - `LinkSocialAccountResponse`

### authz-core

**Spec file:** `openapi/idam/authz-core/openapi.yaml`
**Schemas defined:** 8 | **Endpoints:** 5

| Category | Count | % |
|----------|-------|---|
| Schemas with examples | 0/8 | 0.0% |
| Schemas WITHOUT examples | 8/8 | 100.0% |
| Endpoints with request examples | 0/5 | 0.0% |
| Endpoints with response examples | 5/5 | 100.0% |
| Endpoints with ANY examples | 5/5 | 100.0% |
| Endpoints with NO examples | 0/5 | 0.0% |


### api-keys

**Spec file:** `openapi/idam/api-keys/openapi.yaml`
**Schemas defined:** 16 | **Endpoints:** 11

| Category | Count | % |
|----------|-------|---|
| Schemas with examples | 2/16 | 12.5% |
| Schemas WITHOUT examples | 14/16 | 87.5% |
| Endpoints with request examples | 0/11 | 0.0% |
| Endpoints with response examples | 8/11 | 72.7% |
| Endpoints with ANY examples | 8/11 | 72.7% |
| Endpoints with NO examples | 3/11 | 27.3% |

**Schemas WITH examples (2):**
  - `CreateApiKeyRequest`: property-level (permissions, expires_in_days)
  - `ErrorResponse`: root object (2 keys: error, error_description)
**Schemas WITHOUT examples (14) — NEED EXAMPLES:**
  - `Error`
  - `UpdateApiKeyRequest`
  - `ValidateApiKeyRequest`
  - `ApiKeyCreateResponse`
  - `ApiKey`
  - `ApiKeyListResponse`
  - `ApiKeyValidationResponse`
  - `PersonalApiKeyValidationResponse`
  - `OrgApiKeyValidationResponse`
  - `ApiKeyUsageResponse`
  - `ArchivedApiKey`
  - `ArchivedApiKeyListResponse`
  - `ImportApiKeysRequest`
  - `ImportApiKeysResponse`

### org-mgmt

**Spec file:** `openapi/idam/org-mgmt/openapi.yaml`
**Schemas defined:** 44 | **Endpoints:** 43

| Category | Count | % |
|----------|-------|---|
| Schemas with examples | 6/44 | 13.6% |
| Schemas WITHOUT examples | 38/44 | 86.4% |
| Endpoints with request examples | 0/43 | 0.0% |
| Endpoints with response examples | 30/43 | 69.8% |
| Endpoints with ANY examples | 30/43 | 69.8% |
| Endpoints with NO examples | 13/43 | 30.2% |

**Schemas WITH examples (6):**
  - `ScimUser`: property-level (schemas)
  - `ScimUserCreateRequest`: property-level (schemas)
  - `ScimUserUpdateRequest`: property-level (schemas)
  - `ScimUserListResponse`: property-level (schemas)
  - `ErrorResponse`: root object (2 keys: error, error_description)
  - `ScimError`: root object (4 keys: schemas, detail, status, scimType)
**Schemas WITHOUT examples (38) — NEED EXAMPLES:**
  - `Error`
  - `CreateOrgRequest`
  - `Org`
  - `UpdateOrgRequest`
  - `OrgListResponse`
  - `UsersInOrgResponse`
  - `AddUserToOrgRequest`
  - `InviteUserToOrgRequest`
  - `InviteUserToOrgByIdRequest`
  - `ChangeUserRoleRequest`
  - `RemoveUserFromOrgRequest`
  - `PendingInvitesResponse`
  - `RevokeInviteRequest`
  - `RoleMappingResponse`
  - `SubscribeRoleMappingRequest`
  - `OrgDomainsRequest`
  - `SamlLinkRequest`
  - `SamlConnectionLinkResponse`
  - `OidcMetadataRequest`
  - `ScimGroupsResponse`
  - `ScimGroup`
  - `ApplicationListResponse`
  - `Application`
  - `CreateApplicationRequest`
  - `RoleListResponse`
  - `Role`
  - `CreateRoleRequest`
  - `PermissionListResponse`
  - `Permission`
  - `CreatePermissionRequest`
  - `AssignPermissionRequest`
  - `CreateWebhookSubscriptionRequest`
  - `UpdateWebhookSubscriptionRequest`
  - `WebhookSubscription`
  - `WebhookSubscriptionListResponse`
  - `WebhookTestResponse`
  - `WebhookEvent`
  - `InvalidateKeysResponse`

---

## Endpoints Missing All Examples (43 total)

These endpoints have **zero** examples — neither request body nor any response.
They are the highest priority for filling.

### identity-login-service (20 missing)

| Method | Path | Operation ID |
|--------|------|-------------|
| POST | `/login` | `auth_login` |
| POST | `/login/email-otp` | `login_email_otp` |
| POST | `/verify/email-otp` | `verify_email_otp` |
| POST | `/login/phone-otp` | `login_phone_otp` |
| POST | `/verify/phone-otp` | `verify_phone_otp` |
| POST | `/login/dual-otp` | `login_dual_otp` |
| POST | `/verify/dual-otp` | `verify_dual_otp` |
| POST | `/register` | `auth_register` |
| POST | `/token` | `auth_token` |
| POST | `/forgot-password` | `auth_forgot_password` |
| POST | `/reset-password` | `auth_reset_password` |
| POST | `/logout` | `auth_logout` |
| GET | `/social/{provider}/login` | `social_login` |
| POST | `/social/{provider}/callback` | `social_callback` |
| GET | `/oauth/authorize` | `oauth_authorize` |
| GET | `/signup/validate` | `signup_validate` |
| POST | `/login/magic-link` | `magic_link_send` |
| POST | `/login/magic-link/verify` | `magic_link_verify` |
| POST | `/login/phone-magic-link` | `sms_magic_link_send` |
| POST | `/login/phone-magic-link/verify` | `sms_magic_link_verify` |

### identity-session-service (3 missing)

| Method | Path | Operation ID |
|--------|------|-------------|
| GET | `/.well-known/openid-configuration` | `openid_configuration` |
| GET | `/.well-known/jwks.json` | `jwks` |
| POST | `/verify/step-up` | `step_up_verify` |

### identity-user-mgmt-service (4 missing)

| Method | Path | Operation ID |
|--------|------|-------------|
| POST | `/users/{user_id}/social/link` | `link_social_account` |
| GET | `/users/{user_id}/social/tokens` | `fetch_user_oauth_tokens` |
| GET | `/users/{user_id}/social/tokens/{provider}/refresh` | `fetch_fresh_oauth_token` |
| POST | `/oauth/logout` | `oauth_logout` |

### api-keys (3 missing)

| Method | Path | Operation ID |
|--------|------|-------------|
| GET | `/current` | `fetch_active_api_keys` |
| GET | `/usage` | `fetch_api_key_usage` |
| GET | `/archived` | `fetch_archived_api_keys` |

### org-mgmt (13 missing)

| Method | Path | Operation ID |
|--------|------|-------------|
| GET | `/` | `query_orgs` |
| GET | `/{org_id}/users` | `fetch_users_in_org` |
| GET | `/{org_id}/role-mappings` | `fetch_role_mappings` |
| POST | `/{org_id}/create-saml-link` | `create_saml_link` |
| GET | `/{org_id}/scim/groups` | `fetch_scim_groups` |
| GET | `/api/v1/am/applications` | `list_applications` |
| GET | `/api/v1/am/applications/{app_id}/roles` | `list_roles` |
| GET | `/api/v1/am/applications/{app_id}/permissions` | `list_permissions` |
| GET | `/{org_id}/webhooks` | `fetch_webhook_subscriptions` |
| GET | `/{org_id}/scim/users` | `scim_list_users` |
| POST | `/{org_id}/scim/users` | `scim_create_user` |
| PUT | `/{org_id}/scim/users/{user_id}` | `scim_update_user` |
| DELETE | `/{org_id}/scim/users/{user_id}` | `scim_delete_user` |

---

## Endpoints With Response Examples Only (77 total)

These endpoints have response examples but **NO request body examples**.
Priority: HIGH — request examples needed for every endpoint that accepts a body.

### identity-session-service (13 endpoints)

| Method | Path | Response Examples |
|--------|------|------------------|
| POST | `/refresh` | res.401.application/json |
| GET | `/api/v1/identity/users/me` | res.401.application/json |
| PATCH | `/api/v1/identity/users/me` | res.401.application/json |
| GET | `/api/v1/identity/users/me/userinfo` | res.401.application/json |
| POST | `/api/v1/identity/users/me/token` | res.403.application/json, res.404.application/json |
| POST | `/mcp/token` | res.401.application/json |
| POST | `/mcp/token/validate` | res.401.application/json |
| GET | `/api/v1/platform/mcp/agents` | res.401.application/json |
| POST | `/api/v1/platform/mcp/agents` | res.400.application/json, res.401.application/json |
| GET | `/api/v1/platform/mcp/agents/{agent_id}` | res.404.application/json |
| DELETE | `/api/v1/platform/mcp/agents/{agent_id}` | res.404.application/json |
| POST | `/admin/impersonate` | res.403.application/json, res.404.application/json |
| POST | `/admin/impersonate/restore` | res.403.application/json |

### identity-user-mgmt-service (21 endpoints)

| Method | Path | Response Examples |
|--------|------|------------------|
| POST | `/users` | res.400.application/json |
| DELETE | `/users/{user_id}` | res.404.application/json |
| GET | `/users/query` | res.400.application/json |
| GET | `/users/email` | res.404.application/json |
| GET | `/users/username` | res.404.application/json |
| GET | `/users/{user_id}/employee` | res.404.application/json |
| POST | `/users/{user_id}/disable` | res.404.application/json |
| POST | `/users/{user_id}/enable` | res.404.application/json |
| POST | `/users/{user_id}/logout-all-sessions` | res.404.application/json |
| POST | `/users/{user_id}/magiclink` | res.404.application/json |
| DELETE | `/users/{user_id}/password` | res.404.application/json, res.400.application/json |
| POST | `/users/{user_id}/email/verify` | res.400.application/json, res.404.application/json |
| POST | `/users/{user_id}/resend-email-confirmation` | res.404.application/json |
| POST | `/users/{user_id}/mfa/disable` | res.404.application/json |
| POST | `/users/{user_id}/mfa/setup` | res.400.application/json, res.404.application/json |
| POST | `/users/{user_id}/mfa/verify` | res.400.application/json, res.404.application/json |
| POST | `/users/{user_id}/phone` | res.400.application/json, res.404.application/json |
| POST | `/users/{user_id}/phone/verify` | res.400.application/json, res.404.application/json |
| PUT | `/users/{user_id}/email` | res.400.application/json, res.404.application/json |
| POST | `/users/migrate` | res.400.application/json |
| POST | `/users/migrate-password` | res.400.application/json |

### authz-core (5 endpoints)

| Method | Path | Response Examples |
|--------|------|------------------|
| POST | `/principals/roles` | res.400.application/json, res.404.application/json |
| DELETE | `/principals/roles` | res.404.application/json |
| POST | `/principals/attributes` | res.400.application/json |
| POST | `/principal/effective` | res.400.application/json, res.404.application/json |
| POST | `/authorize` | res.400.application/json |

### api-keys (8 endpoints)

| Method | Path | Response Examples |
|--------|------|------------------|
| POST | `/` | res.400.application/json |
| PUT | `/{key_id}` | res.404.application/json |
| DELETE | `/{key_id}` | res.404.application/json |
| POST | `/validate` | res.401.application/json |
| POST | `/validate/personal` | res.401.application/json |
| GET | `/archived/{key_id}` | res.404.application/json |
| POST | `/import` | res.400.application/json |
| POST | `/validate/org` | res.401.application/json |

### org-mgmt (30 endpoints)

| Method | Path | Response Examples |
|--------|------|------------------|
| GET | `/{org_id}` | res.404.application/json |
| PUT | `/{org_id}` | res.400.application/json, res.404.application/json |
| DELETE | `/{org_id}` | res.404.application/json |
| POST | `/{org_id}/users` | res.400.application/json, res.404.application/json |
| POST | `/{org_id}/invite-user` | res.400.application/json |
| POST | `/{org_id}/invite-user-by-id` | res.400.application/json |
| DELETE | `/{org_id}/pending-invites` | res.404.application/json |
| PUT | `/{org_id}/subscribe-role-mapping` | res.400.application/json, res.404.application/json |
| PUT | `/{org_id}/domains` | res.400.application/json, res.404.application/json |
| POST | `/{org_id}/allow-saml` | res.404.application/json |
| POST | `/{org_id}/disallow-saml` | res.404.application/json |
| PUT | `/{org_id}/saml-metadata` | res.400.application/json |
| POST | `/{org_id}/oidc-metadata` | res.400.application/json |
| POST | `/{org_id}/enable-saml` | res.404.application/json |
| DELETE | `/{org_id}/saml` | res.404.application/json |
| POST | `/{org_id}/migrate-to-isolated` | res.400.application/json, res.404.application/json |
| GET | `/{org_id}/scim/groups/{group_id}` | res.404.application/json |
| POST | `/api/v1/am/applications` | res.400.application/json |
| GET | `/api/v1/am/applications/{app_id}` | res.404.application/json |
| POST | `/api/v1/am/applications/{app_id}/roles` | res.400.application/json |
| GET | `/api/v1/am/applications/{app_id}/roles/{role_id}` | res.404.application/json |
| POST | `/api/v1/am/applications/{app_id}/permissions` | res.400.application/json |
| GET | `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | res.404.application/json |
| POST | `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | res.400.application/json, res.404.application/json |
| DELETE | `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | res.404.application/json |
| DELETE | `/{org_id}/webhooks/{subscription_id}` | res.404.application/json |
| POST | `/{org_id}/webhooks/{subscription_id}/test` | res.404.application/json |
| POST | `/admin/users/{user_id}/invalidate-all-keys` | res.403.application/json |
| DELETE | `/{org_id}/users/{user_id}` | res.404.application/json |
| PATCH | `/{org_id}/users/{user_id}/role` | res.400.application/json, res.404.application/json |

---

## Response Example Distribution

All existing response examples are error responses:

| Status Code | Count | Description |
|-------------|-------|-------------|
| 400 | 32 | Bad Request / Validation Error |
| 401 | 11 | Unauthorized (invalid/missing token) |
| 403 | 4 | Forbidden |
| 404 | 49 | Not Found |
| **Total** | **96** | |

### Critical Gap: Zero Success Response Examples

- No endpoint has a `200` or `201` or `204` response example
- This means clients cannot see what successful responses look like
- Every endpoint that returns data has no example success payload

---

## Sample Existing Examples (for reference)

### Well-formed examples found

**`identity-login-service` → `MfaRequiredResponse`:**
**`identity-login-service` → `DualOTPResponse`:**
**`identity-session-service` → `CreateUserRequest`:**
Keys: `email, `email_confirmed, `send_email_confirmation, `first_name, `last_name, `username` ...

```json
{
  "email": "newuser@example.com",
  "email_confirmed": "False",
  "send_email_confirmation": "True",
  "first_name": "New",
  "last_name": "User",
  "username": "newuser",
  "send_welcome_email": "True",
  "extra_properties": "dict",
  "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492"
}
```

**`identity-session-service` → `DualOTPResponse`:**
**`identity-session-service` → `MfaRequiredResponse`:**
Keys: `session_id, `mfa_required, `mfa_type` ...

```json
{
  "session_id": "sess_abc123",
  "mfa_required": "True",
  "mfa_type": "sms"
}
```

**`identity-session-service` → `User`:**
Keys: `user_id, `email, `email_confirmed, `first_name, `last_name, `username` ...

```json
{
  "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
  "email": "test@example.com",
  "email_confirmed": "True",
  "first_name": "Test",
  "last_name": "User",
  "username": "testuser",
  "picture_url": "https://example.com/avatar.png",
  "properties": "dict",
  "locked": "False",
  "enabled": "True",
  "has_password": "True",
  "update_password_required": "False",
  "mfa_enabled": "False",
  "phone_number": "+14155551234",
  "phone_verified": "True",
  "mfa_factors": "list",
  "can_create_org
```

**`identity-user-mgmt-service` → `CreateUserRequest`:**
Keys: `email, `email_confirmed, `send_email_confirmation, `first_name, `last_name, `username` ...

```json
{
  "email": "newuser@example.com",
  "email_confirmed": "False",
  "send_email_confirmation": "True",
  "first_name": "New",
  "last_name": "User",
  "username": "newuser",
  "send_welcome_email": "True",
  "extra_properties": "dict",
  "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492"
}
```

**`api-keys` → `CreateApiKeyRequest`:**
---

## Recommendations

### Priority 1 — MUST have before backend implementation

1. **Add request body examples to ALL POST/PUT/PATCH endpoints**
   - 43 endpoints missing all examples are the worst offenders
   - 77 endpoints missing request examples (all POST/PUT/PATCH)
   - This is the #1 gap — without request examples, generated clients
     have no idea what payload shape to expect

2. **Add success response examples (200/201/204) for every endpoint**
   - Zero success response examples exist anywhere
   - Clients need to see what successful responses look like

3. **Fill schema-level examples for all request/response types**
   - 143 schemas have no examples at all
   - Focus on: core domain types (User, Org, ApiKey, Role, etc.)

### Priority 2 — SHOULD have for client generation quality

4. **Add examples for authentication endpoints**
   - `identity-login-service` is 0% covered (20/20 endpoints missing)
   - This is the most critical service for frontend clients

5. **Standardize error response examples**
   - Current error examples are consistent (good) but only cover 4xx
   - Consider adding 5xx examples too

### Priority 3 — NICE TO HAVE

6. **Add array item-level examples**
   - Zero schemas currently have examples showing what array items look like
   - Important for list endpoints (e.g., what does a single User look like in UserListResponse)

7. **Add path parameter examples**
   - Endpoints like `/users/{user_id}/social/link` need path param examples

---

## Methodology

1. Loaded all 6 OpenAPI specs from `openapi/idam/{service}/openapi.yaml`
2. For each endpoint: checked `requestBody.content.*.example` and `responses.*.content.*.example`
3. For each schema: recursively extracted `example` fields from root, properties, items
4. Categorized gaps by severity and service
5. CSV output: `docs/audit/openapi_example_coverage.csv`
