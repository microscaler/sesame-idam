---
title: User Entity
status: verified
updated: 2026-05-16
sources: [openapi/*/openapi.yaml, microservices/*/impl/src/models/]
---

# Entity: User

Owned by: **identity-login-service** (also consumed by authz-core for principal data, identity-user-mgmt-service for CRUD)

## Description

Single user table with two user types (`customer` | `platform`). No separate tables for platform vs customer users. The `user_type` JWT claim distinguishes them.

Users support multiple authentication methods: password, email OTP, phone OTP, dual OTP (email + phone), social OAuth, and magic links.

## Schema (from impl/ crate — identity-login-service)

|| Column | Type | Notes |
||--------|------|-------|
|| id | uuid (PK) | |
|| email | varchar(255) | Email address |
|| password_hash | text | Bcrypt/scrypt hash |
|| tenant_id | varchar(255) | **REQUIRED** — partitions data per tenant |
|| email_verified | boolean | Email confirmed via OTP or link |
|| phone | varchar(64, nullable) | Phone number |
|| phone_verified | boolean | Phone confirmed via SMS OTP |
|| status | varchar(32) | Active, suspended, etc. (replaces enabled/locked) |
|| created_at | timestamptz | |
|| updated_at | timestamptz | |

## Multi-Tenancy

**Critical:** `UNIQUE(tenant_id, email)` — the same email can exist on different tenants but represents unrelated users. `alice@corp.com` on `Tenant A` and `alice@corp.com` on `Tenant B` are different people. No cross-tenant identity exists.

Within a single tenant, email is globally unique. Within a multi-application tenant (e.g., hauliage with hauliage-web, hauliage-api, hauliage-admin), all applications share the same user base.

## Auth Method Flags

|| Field | Description |
||-------|-------------|
|| `email_verified` | Email address confirmed (via OTP or link) |
|| `phone_verified` | Phone number confirmed (via SMS OTP) |
|| `status` | Active, suspended, etc. (single status field replaces separate enabled/locked flags) |

**Note:** `has_password`, `first_name`, `last_name`, `username`, `picture_url`, `extra_properties`, `deleted_at` are NOT in the impl model. `username` appears only in OpenAPI request/response schemas, not in the database.

## Key Design Decisions

1. **Single user table.** User type is distinguished by JWT claim, not a DB column. The `user_type` column from the wiki does not exist in the impl.
2. **Single status field.** The `status` column (varchar(32)) replaces separate `enabled`/`locked` flags. No soft delete (`deleted_at`) exists.
3. **Multiple auth methods supported simultaneously.** Users can have password + email OTP + phone OTP + social OAuth active.
4. **Dual OTP for high security.** Both email and phone must be verified for login (see `POST /login/dual-otp` and `POST /verify/dual-otp`).
5. **Password clearing for SSO-only.** `DELETE /users/{user_id}/password` removes password, forcing SSO/social login only.
6. **No PII fields.** `first_name`, `last_name`, `picture_url` are NOT in the database model. `username` exists only in OpenAPI schemas, not in the impl.

## API Endpoints (User)

| Service | Endpoint | Purpose |
|---------|----------|---------|
| identity-login-service | `POST /auth/login` | Login with password |
| identity-login-service | `POST /auth/login/dual-otp` | Send OTPs to both email and phone simultaneously |
| identity-login-service | `POST /auth/login/email-otp` | Send email OTP |
| identity-login-service | `POST /auth/login/magic-link` | Send magic link for passwordless login |
| identity-login-service | `POST /auth/login/magic-link/verify` | Verify magic link token and complete login |
| identity-login-service | `POST /auth/login/phone-magic-link` | Send SMS magic link for passwordless login |
| identity-login-service | `POST /auth/login/phone-magic-link/verify` | Verify SMS magic link token and complete login |
| identity-login-service | `POST /auth/login/phone-otp` | Send phone SMS OTP |
| identity-login-service | `POST /auth/logout` | Logout (revoke refresh token) |
| identity-login-service | `POST /auth/password/forgot` | Request password reset email |
| identity-login-service | `POST /auth/password/reset` | Confirm password reset with token |
| identity-login-service | `POST /auth/register` | Register new user with email and password |
| identity-login-service | `GET /auth/signup/validate` | Validate signup eligibility |
| identity-login-service | `POST /auth/social/{provider}/callback` | Exchange OAuth provider callback for tokens |
| identity-login-service | `GET /auth/social/{provider}/login` | Initiate OAuth login with provider |
| identity-login-service | `POST /auth/token` | Token endpoint (refresh, client_credentials, token_exchange RFC 8693) |
| identity-login-service | `POST /auth/verify/dual-otp` | Verify dual OTP codes and complete login |
| identity-login-service | `POST /auth/verify/email-otp` | Verify email OTP and complete login |
| identity-login-service | `POST /auth/verify/phone-otp` | Verify phone SMS OTP and complete login |
| identity-login-service | `GET /oauth/authorize` | OAuth2 authorization endpoint |
| identity-user-mgmt-service | `POST /admin/audit/events` | Get user-specific audit events |
| identity-user-mgmt-service | `POST /admin/audit/users/{user_id}/events/compliance-export` | Export user's audit events (GDPR) |
| identity-user-mgmt-service | `GET /admin/audit/users/{user_id}/events/count` | Get user event count |
| identity-user-mgmt-service | `POST /admin/users` | Create user (idempotent by email) |
| identity-user-mgmt-service | `GET /admin/users/email` | Fetch user by email |
| identity-user-mgmt-service | `POST /admin/users/migrate` | Migrate user from external auth system |
| identity-user-mgmt-service | `POST /admin/users/migrate-password` | Bulk migrate passwords (hash+salt) |
| identity-user-mgmt-service | `GET /admin/users/query` | Paginated query for users with filters |
| identity-user-mgmt-service | `GET /admin/users/username` | Fetch user by username |
| identity-user-mgmt-service | `DELETE /admin/users/{user_id}` | Delete user (irreversible) |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/disable` | Disable/block user |
| identity-user-mgmt-service | `PUT /admin/users/{user_id}/email` | Change user email |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/email/verify` | Verify user email |
| identity-user-mgmt-service | `GET /admin/users/{user_id}/employee` | Fetch user in employee mode |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/enable` | Enable/unblock user |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/logout-all-sessions` | Logout all user sessions |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/magiclink` | Send magic link for login |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/mfa/disable` | Disable user 2FA |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/mfa/setup` | Set up TOTP MFA |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/mfa/verify` | Verify MFA code |
| identity-user-mgmt-service | `DELETE /admin/users/{user_id}/password` | Clear password (convert to SSO-only) |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/phone` | Add phone number for user |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/phone/verify` | Verify phone number |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/resend-email-confirmation` | Resend email confirmation |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/social/link` | Link social account to user |
| identity-user-mgmt-service | `GET /admin/users/{user_id}/social/tokens` | Fetch user's OAuth tokens from providers |
| identity-user-mgmt-service | `GET /admin/users/{user_id}/social/tokens/{provider}/refresh` | Fetch fresh token from provider |
| identity-user-mgmt-service | `POST /oauth/logout` | OAuth2 logout endpoint |

## Code Anchors

- `microservices/idam/*/impl/src/models/` — Lifeguard entity definitions per service
- `openapi/*/openapi.yaml` — API request/response schemas (see per-service `README.md`)

## Drift Found (verified 2026-05-16)

| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|
| `user_type` column exists | Column does not exist; type is a JWT claim only | Critical — schema reference was wrong |
| `first_name`, `last_name`, `picture_url`, `extra_properties` | NOT in impl | High — wiki overstates user profile fields |
| `username` column | Only in OpenAPI schemas, NOT in impl | Medium — DB lookup by username not possible |
| `locked` / `enabled` separate flags | Single `status` (varchar(32)) column | Medium — status enum replaces boolean flags |
| `has_password` flag | NOT in impl (password existence inferred from password_hash being non-null) | Low |
| `deleted_at` soft delete | NOT in impl | Medium — no soft delete support |
| `last_active_at` | NOT in impl | Low |
| `tenant_id` is uuid | `tenant_id` is varchar(255), not uuid | Low — type mismatch |
