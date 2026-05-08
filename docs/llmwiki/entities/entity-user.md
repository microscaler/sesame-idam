---
title: User Entity
status: partially-verified
updated: 2026-01-22
sources: [openapi/*/openapi.yaml, microservices/*/impl/src/models/]
---

# Entity: User

Owned by: **identity-login-service** (also consumed by authz-core for principal data, identity-user-mgmt-service for CRUD)

## Description

Single user table with two user types (`customer` | `platform`). No separate tables for platform vs customer users. The `user_type` JWT claim distinguishes them.

Users support multiple authentication methods: password, email OTP, phone OTP, dual OTP (email + phone), social OAuth, and magic links.

## Schema (from OpenAPI specs + design-doc.md)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| email | text | Email address |
| tenant_id | uuid (FK, UK part 1) | **REQUIRED** — partitions data per tenant |
| email_confirmed | boolean | |
| phone_number | text (nullable) | |
| phone_confirmed | boolean | |
| user_type | text | `customer` or `platform` |
| first_name | text | |
| last_name | text | |
| username | text (nullable) | |
| picture_url | text | |
| extra_properties | jsonb | Custom metadata |
| locked | boolean | Account lockout |
| enabled | boolean | Soft disable flag |
| has_password | boolean | Whether password hash exists |
| password_hash | text | Bcrypt/scrypt hash |
| created_at | timestamptz | |
| updated_at | timestamptz | |
| last_active_at | timestamptz | |
| deleted_at | timestamptz | Soft delete |

## Multi-Tenancy

**Critical:** `UNIQUE(tenant_id, email)` — the same email can exist on different tenants but represents unrelated users. `alice@corp.com` on `Tenant A` and `alice@corp.com` on `Tenant B` are different people. No cross-tenant identity exists.

Within a single tenant, email is globally unique. Within a multi-application tenant (e.g., hauliage with hauliage-web, hauliage-api, hauliage-admin), all applications share the same user base.

## Auth Method Flags

| Field | Description |
|-------|-------------|
| `has_password` | User can login with email+password |
| `email_verified` | Email address confirmed (via OTP or link) |
| `phone_verified` | Phone number confirmed (via SMS OTP) |
| `social_providers` | JSON array of social providers (google, github, etc.) |
| `sso_only` | `has_password=false` + SSO-only mode (set via `DELETE /users/{user_id}/password`) |

## Key Design Decisions

1. **One user table.** The `user_type` column distinguishes customer from platform users. JWT claim shape differs per type.
2. **Soft deletes everywhere.** `deleted_at` column allows graceful deletion with auditability.
3. **Multiple auth methods supported simultaneously.** Users can have password + email OTP + phone OTP + social OAuth active.
4. **Dual OTP for high security.** Both email and phone must be verified for login (see `POST /login/dual-otp` and `POST /verify/dual-otp`).
5. **Password clearing for SSO-only.** `DELETE /users/{user_id}/password` removes password, forcing SSO/social login only.

## API Endpoints (User)

| Service | Endpoint | Purpose |
|---------|----------|---------|
| identity-login-service | `POST /register` | Create user with email+password |
| identity-login-service | `POST /login` | Email+password login |
| identity-login-service | `POST /login/email-otp` | Send email OTP |
| identity-login-service | `POST /verify/email-otp` | Verify email OTP code |
| identity-login-service | `POST /login/phone-otp` | Send SMS OTP |
| identity-login-service | `POST /verify/phone-otp` | Verify SMS OTP code |
| identity-login-service | `POST /login/dual-otp` | Send OTPs to email + phone |
| identity-login-service | `POST /verify/dual-otp` | Verify dual OTP codes |
| identity-login-service | `POST /login/magic-link` | Send magic link (passwordless) |
| identity-login-service | `POST /login/magic-link/verify` | Verify magic link token |
| identity-login-service | `POST /login/phone-magic-link` | Send SMS magic link |
| identity-login-service | `POST /login/phone-magic-link/verify` | Verify SMS magic link |
| identity-session-service | `GET /api/v1/identity/users/me` | Current user profile |
| identity-session-service | `PATCH /api/v1/identity/users/me` | Update current user profile |
| identity-session-service | `POST /api/v1/identity/users/me/token` | Issue direct token (admin) |
| identity-user-mgmt-service | `POST /users` | Admin create user |
| identity-user-mgmt-service | `GET /users` | Admin list users |
| identity-user-mgmt-service | `GET /users/query` | Admin paginated search |
| identity-user-mgmt-service | `GET /users/email` | Lookup user by email |
| identity-user-mgmt-service | `GET /users/username` | Lookup user by username |
| identity-user-mgmt-service | `GET /users/{user_id}` | Get user by ID |
| identity-user-mgmt-service | `DELETE /users/{user_id}` | Delete user |
| identity-user-mgmt-service | `PUT /users/{user_id}/email` | Update email |
| identity-user-mgmt-service | `DELETE /users/{user_id}/password` | Clear password (SSO-only) |
| identity-user-mgmt-service | `POST /users/{user_id}/mfa/setup` | TOTP setup |
| identity-user-mgmt-service | `POST /users/{user_id}/mfa/verify` | MFA verify |
| identity-user-mgmt-service | `POST /users/{user_id}/mfa/disable` | MFA disable |
| identity-user-mgmt-service | `POST /users/{user_id}/phone` | Add/update phone |
| identity-user-mgmt-service | `POST /users/{user_id}/phone/verify` | Verify phone |
| identity-user-mgmt-service | `POST /users/{user_id}/social/link` | Link social account |
| identity-user-mgmt-service | `GET /users/{user_id}/social/tokens` | List social tokens |
| identity-user-mgmt-service | `GET /users/{user_id}/social/tokens/{provider}/refresh` | Refresh social token |
| identity-user-mgmt-service | `POST /users/{user_id}/disable` | Disable user |
| identity-user-mgmt-service | `POST /users/{user_id}/enable` | Enable user |
| identity-user-mgmt-service | `POST /users/{user_id}/logout-all-sessions` | Logout all sessions |
| identity-user-mgmt-service | `POST /users/{user_id}/magiclink` | Admin send magic link |
| identity-user-mgmt-service | `POST /users/{user_id}/email/verify` | Verify email |
| identity-user-mgmt-service | `POST /users/{user_id}/resend-email-confirmation` | Resend confirmation |
| identity-user-mgmt-service | `POST /users/migrate` | Bulk migrate users |
| identity-user-mgmt-service | `POST /users/migrate-password` | Bulk migrate passwords |
| identity-session-service | `POST /admin/users/{user_id}/impersonate` | Admin impersonate user |
| identity-session-service | `POST /admin/users/{user_id}/impersonate/restore` | Restore admin session |

## Code Anchors

- `microservices/idam/*/impl/src/models/` — Lifeguard entity definitions per service
- `openapi/*/openapi.yaml` — API request/response schemas (see per-service `README.md`)

## Gaps / Drift

> **Open:** Verify actual Lifeguard models in impl crates against design doc schema. Some endpoints (step-up MFA, impersonation, direct token) are newly added to specs — implementations may not exist yet.
