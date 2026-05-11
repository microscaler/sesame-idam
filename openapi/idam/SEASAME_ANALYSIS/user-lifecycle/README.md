# User Lifecycle

> **Component:** Complete user management — profile, email/phone, MFA, passwordless, account recovery
> **Priority:** P0 — User data is the core entity that everything else relates to
> **Service:** identity-user-mgmt-service (25 endpoints, 2,285 lines)

---

## The Pitch

**Buyer Question:** *Can I manage the complete user lifecycle — from registration and profile management to MFA, passwordless login, and account recovery — all through a consistent, tenant-aware API?*

If the answer is no, you have a database table, not a user management system. User lifecycle isn't just CRUD on users — it's the orchestration of identity verification, security hardening, and recovery flows that keep users authenticated and their data safe. It's the bridge between authentication (who you are) and authorization (what you can do).

---

## What This Component Does

User Lifecycle manages every aspect of a user's identity data and security posture:

1. **Profile Management** — User profile CRUD with tenant-scoped attributes and custom fields
2. **Email Verification** — Verify, update, and manage user email addresses with confirmation flows
3. **Phone Verification** — SMS-based phone number verification and management
4. **MFA Management** — Enroll, verify, and disable MFA methods (TOTP, SMS, backup codes)
5. **Password Management** — Change, reset, and enforce password policies
6. **Passwordless Login** — Email-based passwordless authentication flows
7. **Account Recovery** — Secure account recovery via email or phone
8. **Social Account Linking** — Link/unlink social OAuth accounts to existing users
9. **Session Management** — View, revoke, and manage user sessions from profile
10. **Account Deletion** — GDPR-compliant account deletion with data retention policies

---

## Entity Model

### User Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | User identifier (primary key) |
| `email` | String (255) | Yes | Primary email address |
| `email_verified` | Boolean | Yes | Whether email has been verified |
| `username` | String (255) | No | Human-readable username |
| `password_hash` | String (255) | No | Bcrypt/argon2 password hash |
| `first_name` | String (128) | No | Given name |
| `last_name` | String (128) | No | Family name |
| `phone_number` | String (64) | No | Primary phone number |
| `phone_verified` | Boolean | No | Whether phone has been verified |
| `avatar_url` | String (1024) | No | Profile picture URL |
| `mfa_enabled` | Boolean | Yes | Whether MFA is enabled |
| `mfa_secret` | String (64) | No | TOTP secret (encrypted at rest) |
| `last_login_at` | DateTime | No | Last successful login timestamp |
| `created_at` | DateTime | Yes | Account creation timestamp |
| `updated_at` | DateTime | Yes | Last profile update timestamp |
| `deleted_at` | DateTime | No | Soft-delete timestamp (GDPR) |
| `tenant_id` | UUID | Yes | Tenant isolation scope |

### User Profile Update Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `first_name` | String (128) | No | Updated given name |
| `last_name` | String (128) | No | Updated family name |
| `avatar_url` | String (1024) | No | Updated profile picture |
| `phone_number` | String (64) | No | Updated phone number |
| `metadata` | JSON | No | Custom profile attributes |

### Email Update Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `email` | String (255) | Yes | New email address |
| `current_password` | String (128) | Yes | Current password for verification |
| `callback_url` | String (512) | No | Verification callback URL |

### MFA Setup Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `method` | Enum: [totp, sms, backup_codes] | Yes | MFA method to setup |
| `phone_number` | String (64) | No | Phone for SMS MFA |
| `secret` | String (64) | No | TOTP secret |
| `backup_codes` | Array[String] | No | Generated backup codes |

### User Query Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `query` | String (255) | Yes | Search query (email, username, name) |
| `tenant_id` | UUID | Yes | Tenant scope for search |
| `include_deleted` | Boolean | No | Include soft-deleted users |
| `limit` | Integer | Yes | Result limit (default: 50) |
| `offset` | Integer | Yes | Pagination offset |

---

## Entity Relationships

```
User ───┬── Session (one2many)          ← User sessions
        ├── RoleAssignment (many2many)   ← User roles per org
        ├── EmailVerification (one2one)  ← Email verification token
        ├── PhoneVerification (one2one)  ← Phone verification token
        ├── MFAMethod (one2one)          ← MFA enrollment
        ├── PasswordlessRequest (one2many) ← Passwordless login attempts
        └── SocialAccount (one2many)      ← Linked social accounts
```

---

## Required API Endpoints

### Profile Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/users/{id}` | Get user profile |
| `GET` | `/api/v1/users/{id}/profile` | Get detailed user profile |
| `PATCH` | `/api/v1/users/{id}/profile` | Update user profile |
| `GET` | `/api/v1/users` | List users (admin) |
| `GET` | `/api/v1/users/me` | Get current user's profile |

### Email Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/user/email/verify` | Send email verification |
| `POST` | `/api/v1/user/email/update` | Request email change |
| `GET` | `/api/v1/user/email/verify/{token}` | Verify email with token |

### Phone Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/user/phone/verify` | Send SMS verification code |
| `POST` | `/api/v1/user/phone/update` | Update phone number |
| `POST` | `/api/v1/user/phone/verify/{token}` | Verify phone with code |

### MFA Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/user/mfa/setup` | Enroll MFA (TOTP) |
| `POST` | `/api/v1/user/mfa/verify` | Verify MFA code |
| `POST` | `/api/v1/user/mfa/disable` | Disable MFA |
| `GET` | `/api/v1/user/mfa/status` | Get MFA enrollment status |

### Password Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/user/password/change` | Change current password |
| `POST` | `/api/v1/user/password/forgot` | Initiate password reset |
| `POST` | `/api/v1/user/password/reset` | Complete password reset |

### Passwordless & Social

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/user/passwordless/start` | Start passwordless flow |
| `POST` | `/api/v1/user/passwordless/complete` | Complete passwordless login |
| `POST` | `/api/v1/user/social/link` | Link social account to user |
| `DELETE` | `/api/v1/user/social/unlink` | Unlink social account |

### Session Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/user/sessions` | List user's active sessions |
| `POST` | `/api/v1/user/sessions/revoke` | Revoke a specific session |
| `POST` | `/api/v1/user/sessions/revoke-all` | Revoke all user sessions |

### Account Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/user/delete` | Delete user account |
| `POST` | `/api/v1/user/export` | Export user data (GDPR) |
| `GET` | `/api/v1/user/activity` | User activity log |

---

## Competitive Positioning

### Where Sesame-IDAM Wins
- **API-driven profile management** — Every user operation is a REST endpoint. No admin console dependency.
- **Tenant-scoped users** — Users are automatically isolated by tenant_id on every query.
- **Rust performance** — User searches across 1M records are instantaneous in Rust.
- **GDPR-ready** — Account deletion, data export, and privacy controls are API-first.

### Where Sesame-IDAM Lags
- **No user attributes API** — Auth0 provides a comprehensive /users/{id}/app_metadata endpoint for custom attributes.
- **No user import** — Auth0 and Okta support CSV/bulk user import with deduplication.
- **No account linking** — Auth0 handles cross-account merging and social account linking automatically.
- **No user activity log** — Okta provides a comprehensive audit trail per user.

---

## Competitive Intelligence Deep Dive

### Okta: User Profile API
Okta's User Management API supports custom attributes, groups, and complex profile schemas. The /api/v1/users endpoint supports filtering, sorting, and pagination with full CRUD. **Sesame Gap:** No custom attribute schema, no bulk import, no user groups.

### Auth0: User Metadata
Auth0's app_metadata and user_metadata fields allow unlimited custom attributes per user. The management API supports search by any field. **Sesame Gap:** No metadata fields, no arbitrary search.

### Firebase: Simple User Model
Firebase Auth's user model is minimal — email, phone, displayName, photoURL. Everything else is handled by Firestore. **Sesame Gap:** Firebase relies on external database for profile data; Sesame has it built-in.

---

## Implementation Roadmap

### Phase 1: Core User Management (Complete) — P0
1. User profile CRUD ✅
2. Email/phone verification ✅
3. MFA setup/verify/disable ✅
4. Password change/reset ✅
5. Passwordless login ✅
6. Social account linking ✅
7. Session management ✅
8. Account deletion ✅

### Phase 2: User Attributes (Not Implemented) — P1
1. Custom user attributes (app_metadata, user_metadata)
2. Bulk user import from CSV
3. User search by any attribute
4. User avatar upload

### Phase 3: Advanced Features (Not Implemented) — P2
1. Account merge/linking (cross-provider)
2. User activity timeline
3. Data export (GDPR)
4. Account deactivation (soft-delete with reactivation)

---

## Key Takeaway for Buyers

Sesame-IDAM's user lifecycle is **functionally complete for basic CRUD** — profile, email, phone, MFA, password, social linking, sessions, and deletion are all implemented. The gap is in **advanced user features**: custom attributes, bulk import, and activity logging.

**For organizations that need basic user management with API-first access**, Sesame-IDAM is sufficient. **For enterprises that need user attributes, bulk operations, and audit trails**, Okta or Auth0 remain the better choice until these features are implemented.
