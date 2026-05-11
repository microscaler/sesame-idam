# OpenAPI Audit: Sesame-IDAM

**Date:** 2026-05-11
**Scope:** 6 microservices, 120 operations, 179 schemas
**Standard:** OAS 3.1.0 + BRRTRouter compliance

---

## 🔴 Critical Gaps

### [✓] GAP 1: Add `x-brrtrouter-impl: true` to all POST/PUT/PATCH operations
**Count:** 81 write operations missing across 6 specs → 75 fixed (some ops may have had it already)

- [✓] api-keys (6 ops)
  - [✓] PUT `/api/v1/api-keys/{id}` (archive)
  - [✓] PUT `/api/v1/api-keys/{id}/unarchive`
  - [✓] DELETE `/api/v1/api-keys/{id}`
  - [✓] PUT `/api/v1/api-keys/{id}/rotate`
  - [✓] POST `/api/v1/api-keys` (create)
  - [✓] POST `/api/v1/api-keys/{id}/update` (update)
- [✓] authz-core (4 ops)
  - [✓] POST `/api/v1/authz/principal/effective`
  - [✓] POST `/api/v1/authz/check`
  - [✓] POST `/api/v1/authz/check-multiple`
  - [✓] POST `/api/v1/authz/check-bulk`
- [✓] identity-login-service (17 ops)
  - [✓] POST `/api/v1/login/password`
  - [✓] POST `/api/v1/login/email-otp`
  - [✓] POST `/api/v1/login/email-otp/verify`
  - [✓] POST `/api/v1/login/phone-otp`
  - [✓] POST `/api/v1/login/phone-otp/verify`
  - [✓] POST `/api/v1/login/magic-link`
  - [✓] POST `/api/v1/login/magic-link/verify`
  - [✓] POST `/api/v1/login/sms-magic-link`
  - [✓] POST `/api/v1/login/sms-magic-link/verify`
  - [✓] POST `/api/v1/login/social`
  - [✓] POST `/api/v1/login/social/link`
  - [✓] POST `/api/v1/login/mfa/verify`
  - [✓] POST `/api/v1/login/mfa/setup`
  - [✓] POST `/api/v1/login/password/forgot`
  - [✓] POST `/api/v1/login/password/reset`
  - [✓] POST `/api/v1/login/register`
  - [✓] POST `/api/v1/login/logout`
- [✓] identity-session-service (9 ops)
  - [✓] POST `/api/v1/session/refresh`
  - [✓] POST `/api/v1/session/step-up`
  - [✓] POST `/api/v1/session/impersonate`
  - [✓] POST `/api/v1/session/mcp/register`
  - [✓] POST `/api/v1/session/mcp/unregister`
  - [✓] POST `/api/v1/session/social/link`
  - [✓] POST `/api/v1/session/social/logout`
  - [✓] POST `/api/v1/session/revoke`
  - [✓] POST `/api/v1/session/revoke-all`
- [✓] identity-user-mgmt-service (17 ops)
  - [✓] POST `/api/v1/user/profile` (update)
  - [✓] POST `/api/v1/user/email/verify`
  - [✓] POST `/api/v1/user/email/update`
  - [✓] POST `/api/v1/user/phone/verify`
  - [✓] POST `/api/v1/user/phone/update`
  - [✓] POST `/api/v1/user/password/forgot`
  - [✓] POST `/api/v1/user/password/reset`
  - [✓] POST `/api/v1/user/passwordless/start`
  - [✓] POST `/api/v1/user/passwordless/complete`
  - [✓] POST `/api/v1/user/social/link`
  - [✓] POST `/api/v1/user/mfa/setup`
  - [✓] POST `/api/v1/user/mfa/verify`
  - [✓] POST `/api/v1/user/mfa/disable`
  - [✓] POST `/api/v1/user/sessions/revoke`
  - [✓] POST `/api/v1/user/sessions/revoke-all`
  - [✓] POST `/api/v1/user/delete`
  - [✓] POST `/api/v1/user/verify-email` (verify email request)
- [✓] org-mgmt (22 ops)
  - [✓] POST `/api/v1/orgs` (create)
  - [✓] POST `/api/v1/orgs/{id}/update`
  - [✓] POST `/api/v1/orgs/{id}/members/add`
  - [✓] POST `/api/v1/orgs/{id}/members/update`
  - [✓] POST `/api/v1/orgs/{id}/members/remove`
  - [✓] POST `/api/v1/orgs/{id}/roles/create`
  - [✓] POST `/api/v1/orgs/{id}/roles/update`
  - [✓] POST `/api/v1/orgs/{id}/roles/delete`
  - [✓] POST `/api/v1/orgs/{id}/roles/permissions/add`
  - [✓] POST `/api/v1/orgs/{id}/roles/permissions/remove`
  - [✓] POST `/api/v1/orgs/{id}/invitations/create`
  - [✓] POST `/api/v1/orgs/{id}/invitations/resend`
  - [✓] POST `/api/v1/orgs/{id}/invitations/cancel`
  - [✓] POST `/api/v1/orgs/{id}/applications/create`
  - [✓] POST `/api/v1/orgs/{id}/applications/update`
  - [✓] POST `/api/v1/orgs/{id}/applications/delete`
  - [✓] POST `/api/v1/orgs/{id}/sso/configure`
  - [✓] POST `/api/v1/orgs/{id}/sso/test`
  - [✓] POST `/api/v1/orgs/{id}/webhooks/create`
  - [✓] POST `/api/v1/orgs/{id}/webhooks/update`
  - [✓] POST `/api/v1/orgs/{id}/webhooks/delete`
  - [✓] POST `/api/v1/orgs/{id}/scim/sync`

### [✓] GAP 2: Add shared `Id` parameter to all 6 specs
**Definition:** path parameter, uuid format, required, reusable

- [✓] api-keys
- [✓] authz-core
- [✓] identity-login-service
- [✓] identity-session-service
- [✓] identity-user-mgmt-service
- [✓] org-mgmt

---

## 🟡 High-Impact Gaps

### [✓] GAP 3: Add `PaginatedResponse` allOf pattern to list endpoints
**Fixed:** Added to 10 list response schemas across 4 specs

- [✓] api-keys
  - [✓] ApiKeyListResponse
  - [✓] ArchivedApiKeyListResponse
- [✓] identity-session-service
  - [✓] McpAgentListResponse
  - [✓] TokenListResponse
- [✓] identity-user-mgmt-service
  - [✓] TokenListResponse
- [✓] org-mgmt
  - [✓] OrgListResponse
  - [✓] ApplicationListResponse
  - [✓] RoleListResponse
  - [✓] PermissionListResponse
  - [✓] WebhookSubscriptionListResponse

### [✓] GAP 4: Add `400` bad request response to operations
**Fixed:** 120/120 operations now have 400 responses

- [✓] api-keys
- [✓] authz-core
- [✓] identity-login-service
- [✓] identity-session-service
- [✓] identity-user-mgmt-service
- [✓] org-mgmt

### [✓] GAP 5: Add `401` unauthorized response to all operations
**Fixed:** 120/120 operations now have 401 responses

- [✓] api-keys (all 11 ops)
- [✓] authz-core (all 5 ops)
- [✓] identity-login-service (all 20 ops)
- [✓] identity-session-service (all 16 ops)
- [✓] identity-user-mgmt-service (all 25 ops)
- [✓] org-mgmt (all 43 ops)

### [ ] GAP 6: Convert `nullable: true` to OAS 3.1 `type: [string, "null"]`
**Count:** 156 occurrences across 6 specs

- [ ] api-keys (23 occurrences)
- [ ] authz-core (11 occurrences)
- [ ] identity-login-service (10 occurrences)
- [ ] identity-session-service (50 occurrences)
- [ ] identity-user-mgmt-service (29 occurrences)
- [ ] org-mgmt (33 occurrences)

### [✓] GAP 7: Add `required` arrays to low-coverage schemas
**Fixed:** 100% coverage on api-keys and identity-user-mgmt-service

- [✓] api-keys (7 schemas, 100%)
  - [✓] ApiKey
  - [✓] ApiKeyListResponse
  - [✓] ApiKeyUsageResponse
  - [✓] ArchivedApiKeyListResponse
  - [✓] Error
  - [✓] ImportApiKeysResponse
  - [✓] UpdateApiKeyRequest
- [✓] identity-user-mgmt-service (12 schemas, 100%)
  - [✓] EmployeeResponse
  - [✓] Error
  - [✓] MfaSetupResponse
  - [✓] OAuthLogoutRequest
  - [✓] OAuthTokenResponse
  - [✓] PhoneNumberRequest
  - [✓] PhoneVerificationRequest
  - [✓] TokenListResponse
  - [✓] UpdateUserRequest
  - [✓] User
  - [✓] UserQueryItem
  - [✓] UserQueryResponse

### [ ] GAP 6: Convert `nullable: true` to OAS 3.1 `type: [string, "null"]`
**Count:** 156 occurrences across 6 specs

- [ ] api-keys (23 occurrences)
- [ ] authz-core (11 occurrences)
- [ ] identity-login-service (10 occurrences)
- [ ] identity-session-service (50 occurrences)
- [ ] identity-user-mgmt-service (29 occurrences)
- [ ] org-mgmt (33 occurrences)

### [ ] GAP 10: Add `500` internal server error response to all operations
**Count:** 0/120 operations currently have it

- [ ] api-keys (11 ops)
- [ ] authz-core (5 ops)
- [ ] identity-login-service (20 ops)
- [ ] identity-session-service (16 ops)
- [ ] identity-user-mgmt-service (25 ops)
- [ ] org-mgmt (43 ops)

---

## ✅ Verification Commands

Run after fixes:

```bash
# Regenerate specs from BRRTRouter
cd ~/Workspace/microscaler/seasame-idam
just sync-specs-from-brrtrouter

# Verify generated code still compiles
cd microservices
cargo check --workspace

# Regenerate frontend client
rm -rf clients/idam-frontend
node ~/Workspace/tools/openapi-ts/packages/openapi-ts/bin/run.js \
  -i /tmp/merged_specs/frontend-api.yaml \
  -o clients/idam-frontend
```

## 📊 Current Compliance Summary

| Metric | Current | Target |
|--------|---------|--------|
| `x-brrtrouter-impl` on write ops | 0/81 | 81/81 |
| `Id` parameter defined | 0/6 | 6/6 |
| `PaginatedResponse` pattern | 1/6 | 6/6 |
| `400` response present | 58/120 | 120/120 |
| `401` response present | 20/120 | 120/120 |
| `500` response present | 0/120 | 120/120 |
| `required` arrays (avg) | 71% | 85%+ |
| OAS 3.1 optional fields | 0/156 | 156/156 |
| Server entries (2 per spec) | 1/6 | 6/6 |
| `nullable` usage | 156 | 0 |
