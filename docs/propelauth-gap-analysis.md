# PropelAuth Gap Analysis

> **Purpose**: Systematic comparison of PropelAuth's API surface vs our sesame-idam, prioritized by effort and impact.
> **Last updated**: 2025-01-XX
> **Status**: Actionable - each gap maps to concrete OpenAPI spec changes.

---

## Executive Summary

PropelAuth exceeds us in **14 areas** across authentication, session management, user administration, and B2B features. 

- **3 P0 gaps**: Hosted UI (frontend), email/SMS delivery (infrastructure), signup validation (API)
- **3 P1 gaps**: Step-up MFA, user impersonation, direct token creation
- **5 P2 gaps**: Clear password, passwordless login, MCP auth, hosted password reset, enterprise SSO
- **3 P3 gaps**: SMS passwordless, token invalidation, SCIM provisioning

**API-level changes needed: ~14 new/updated endpoints across 4 services.**

---

## Priority Matrix

| Priority | Count | Effort | Can we close? |
|----------|-------|--------|---------------|
| P0 | 3 | HIGH (2) + LOW (1) | Only signup validation is API-level LOW |
| P1 | 3 | MEDIUM (3) | All 3 achievable in 1-2 weeks |
| P2 | 5 | LOW-MEDIUM (4) + HIGH (1) | 4 achievable, enterprise SSO is HIGH |
| P3 | 3 | LOW-MEDIUM (3) | All achievable |

---

## P0 - Must Have

### 1. Signup Query Validation

| Field | Value |
|-------|-------|
| **PropelAuth has** | `/api/users/me/signup/validate` - checks if email/phone allowed to register BEFORE form submission |
| **Our state** | We validate on submit but have no pre-check endpoint |
| **Impact** | UX - users discover invalid email/phone AFTER filling forms instead of BEFORE |
| **Effort** | LOW (1-2 hours) |

**API Change - `identity-login-service/openapi.yaml`:**

```yaml
tags:
  - name: Signup
    description: Pre-registration validation

paths:
  /signup/validate:
    get:
      tags: [Signup]
      summary: Validate signup eligibility
      description: Check if email/phone is allowed to register before the user starts filling forms
      parameters:
        - name: email
          in: query
          schema: { type: string }
        - name: phone
          in: query
          schema: { type: string }
      responses:
        '200':
          content:
            application/json:
              schema:
                type: object
                properties:
                  allowed: { type: boolean }
                  reasons: { type: array, items: { type: string } }
                  requires_mfa: { type: boolean }
```

---

### 2. Transactional Email/SMS Delivery

| Field | Value |
|-------|-------|
| **PropelAuth has** | Sends OTP emails, password reset emails, magic links, and SMS natively |
| **Our state** | Our login flows return 'OTP sent' but have no actual delivery mechanism |
| **Impact** | Critical - identity flows are non-functional without email/SMS |
| **Effort** | MEDIUM (1-2 days) |

**No API spec change needed.** This is infrastructure:
- Add SendGrid/Mailgun integration to `identity-login-service`
- Add Twilio/Abstract API integration for SMS
- Wire into existing `/login/email-otp`, `/login/phone-otp`, `/forgot-password` endpoints

PriceWhisperer already has Abstract API email/phone validation - reuse that pattern for delivery.

---

### 3. Hosted UI Pages

| Field | Value |
|-------|-------|
| **PropelAuth has** | Pre-built login, register, password reset pages |
| **Our state** | Zero frontend for identity flows |
| **Impact** | Every new user needs custom frontend code to register/login |
| **Effort** | HIGH (frontend framework needed) |

**No API spec change.** This is frontend work. Options:
- Build a SolidJS frontend (matches our stack)
- Use a hosted auth UI from a library
- Accept that we're API-only and build all frontend ourselves

---

## P1 - Important

### 4. Step-Up MFA

| Field | Value |
|-------|-------|
| **PropelAuth has** | Re-authentication for sensitive actions (e.g., MFA before deleting account) |
| **Our state** | MFA setup/verify exists but no step-up enforcement middleware |
| **Impact** | Security - sensitive ops could happen without re-verification |
| **Effort** | MEDIUM (2-3 hours) |

**API Change - `identity-login-service/openapi.yaml`:**

```yaml
tags:
  - name: StepUp
    description: Multi-factor re-authentication for sensitive operations

paths:
  /verify/step-up:
    post:
      tags: [StepUp]
      summary: Step-up MFA verification
      description: Re-authenticate for sensitive operations (delete account, change email, etc.)
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [user_id, action, session_id]
              properties:
                user_id: { type: string, format: uuid }
                action:
                  type: string
                  enum: [delete_account, change_email, change_password, delete_org]
                session_id: { type: string, format: uuid }
                mfa_method:
                  type: string
                  enum: [totp, email, phone]
      responses:
        '200':
          content:
            application/json:
              schema:
                type: object
                properties:
                  verified: { type: boolean }
                  mfa_method: { type: string }
                  session_id: { type: string, format: uuid }
```

---

### 5. User Impersonation

| Field | Value |
|-------|-------|
| **PropelAuth has** | Admins can view the product AS any user via session switching |
| **Our state** | No impersonation endpoint exists |
| **Impact** | Support/ops - cannot debug user issues without their credentials |
| **Effort** | MEDIUM (3-4 hours) |

**API Change - `identity-session-service/openapi.yaml`:**

```yaml
tags:
  - name: Impersonation
    description: Admin user session switching

paths:
  /admin/users/{user_id}/impersonate:
    post:
      tags: [Impersonation]
      summary: Impersonate user
      description: Admin switches to user session for debugging/support
      parameters:
        - name: user_id
          in: path
          required: true
          schema: { type: string, format: uuid }
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [actor_user_id]
              properties:
                actor_user_id: { type: string, format: uuid }
                reason: { type: string, enum: [debug, support] }
      responses:
        '200':
          content:
            application/json:
              schema:
                type: object
                properties:
                  impersonated_user_id: { type: string, format: uuid }
                  access_token: { type: string }
                  refresh_token: { type: string }
                  original_user_id: { type: string, format: uuid }

  /admin/users/{user_id}/impersonate/restore:
    post:
      tags: [Impersonation]
      summary: Restore admin session
      description: Switch back from impersonated user to admin
```

---

### 6. Direct Access Token Creation

| Field | Value |
|-------|-------|
| **PropelAuth has** | `/api/users/me/token` - programmatically issues access tokens |
| **Our state** | Our `/token` requires valid credentials (login), no admin-issued token endpoint |
| **Impact** | Developer experience - no programmatic token issuance for CLI tools, admin scripts |
| **Effort** | LOW (1-2 hours) |

**API Change - `identity-session-service/openapi.yaml`:**

```yaml
tags:
  - name: TokenIssuance
    description: Programmatic token creation (admin/server-side)

paths:
  /api/v1/identity/users/me/token:
    post:
      tags: [TokenIssuance]
      summary: Issue access token
      description: Programmatically create tokens for server-side flows and admin scripts
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [user_id, scope]
              properties:
                user_id: { type: string, format: uuid }
                scope:
                  type: string
                  enum: [full, read, write]
                expires_in: { type: integer, default: 3600 }
      responses:
        '200':
          content:
            application/json:
              schema:
                type: object
                properties:
                  access_token: { type: string }
                  refresh_token: { type: string }
                  expires_in: { type: integer }
                  token_type: { type: string, example: "Bearer" }
```

---

## P2 - Nice to Have

### 7. Clear User Password

| Field | Value |
|-------|-------|
| **PropelAuth has** | Removes user's password for SSO/social-only login |
| **Our state** | `DELETE /users/{user_id}/password` exists but semantics unclear |
| **Impact** | Edge case - needed when migrating users to SSO-only |
| **Effort** | LOW (30 min) |

**API Change - `identity-user-mgmt-service/openapi.yaml`:**

Clarify `DELETE /users/{user_id}/password` description:
> "Removes the user's password, forcing SSO/social login only. Irreversible."

Or add explicit endpoint:
```yaml
paths:
  /users/{user_id}/clear-password:
    post:
      tags: [PasswordSecurity]
      summary: Clear user password
      description: Remove password, forcing SSO/social login only
```

---

### 8. Passwordless Magic Link Login

| Field | Value |
|-------|-------|
| **PropelAuth has** | Email-only passwordless magic link as PRIMARY auth method |
| **Our state** | `POST /users/{user_id}/magiclink` exists but is admin-initiated, not user login |
| **Impact** | User experience - passwordless is a growing auth preference |
| **Effort** | MEDIUM (2-3 hours) |

**API Change - `identity-login-service/openapi.yaml`:**

```yaml
tags:
  - name: Passwordless
    description: Passwordless magic link authentication

paths:
  /login/magic-link:
    post:
      tags: [Passwordless]
      summary: Send magic link
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [email]
              properties:
                email: { type: string, format: email }
      responses:
        '200':
          content:
            application/json:
              schema:
                type: object
                properties:
                  magic_link_sent: { type: boolean }
                  expires_in: { type: integer, example: 900 }

  /login/magic-link/verify:
    post:
      tags: [Passwordless]
      summary: Verify magic link token
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [token]
              properties:
                token: { type: string }
      responses:
        '200':
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TokenResponse'
```

---

### 9. MCP Authentication

| Field | Value |
|-------|-------|
| **PropelAuth has** | Built-in MCP (Model Context Protocol) authentication for AI agent tool access |
| **Our state** | Removed (see earlier discussion). No MCP endpoints. |
| **Impact** | Future-proofing - if building AI integrations, MCP auth is essential |
| **Effort** | MEDIUM (3-4 hours) |

**API Change - `identity-session-service/openapi.yaml`:**

```yaml
tags:
  - name: MCP
    description: Model Context Protocol authentication

paths:
  /mcp/token:
    post:
      tags: [MCP]
      summary: Issue MCP auth token
      ...
  /mcp/token/validate:
    post:
      tags: [MCP]
      summary: Validate MCP token
      ...
  /api/v1/platform/mcp/agents:
    get: { tags: [MCP], summary: List agents }
    post: { tags: [MCP], summary: Create agent }
  /api/v1/platform/mcp/agents/{agent_id}:
    get: { tags: [MCP], summary: Get agent }
    delete: { tags: [MCP], summary: Delete agent }
```

---

### 10. Hosted Password Reset Flow

| Field | Value |
|-------|-------|
| **PropelAuth has** | Hosted password reset page: forgot -> verify -> reset in one flow |
| **Our state** | API endpoints exist but no hosted page or reset token endpoint |
| **Impact** | UX - password reset is fragmented across API calls |
| **Effort** | LOW (1-2 hours) |

**API Change - `identity-login-service/openapi.yaml`:**

```yaml
paths:
  /api/auth/reset-password/{token}:
    get:
      tags: [PasswordReset]
      summary: Validate password reset token
      description: Frontend uses this to render reset form or show error
      parameters:
        - name: token
          in: path
          required: true
          schema: { type: string }
      responses:
        '200':
          content:
            application/json:
              schema:
                type: object
                properties:
                  valid: { type: boolean }
                  user_id: { type: string, format: uuid }
                  expires_at: { type: string, format: date-time }
        '400':
          description: Invalid or expired token
```

---

## P3 - Future

### 11. Passwordless SMS Login

| Field | Value |
|-------|-------|
| **PropelAuth has** | Phone-based passwordless (SMS magic link) |
| **Our state** | We have phone OTP but no SMS magic link login |
| **Effort** | LOW (1-2 hours) |

**API Change - `identity-login-service/openapi.yaml`:**

```yaml
paths:
  /login/phone-magic-link:
    post:
      tags: [Passwordless]
      summary: Send SMS magic link
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [phone]
              properties:
                phone: { type: string }
      responses:
        '200':
          content:
            application/json:
              schema:
                type: object
                properties:
                  magic_link_sent: { type: boolean }
```

---

### 12. Automatic Token Invalidation on Block/Delete

| Field | Value |
|-------|-------|
| **PropelAuth has** | Automatically invalidates ALL user API keys on block/delete |
| **Our state** | UNKNOWN - need to verify if api-keys service responds to user-block events |
| **Impact** | Security - blocked users might retain API access |
| **Effort** | MEDIUM (2-3 hours) |

**API Change - `identity-user-mgmt-service/openapi.yaml` or `org-mgmt/openapi.yaml`:**

```yaml
paths:
  /admin/users/{user_id}/invalidate-all-keys:
    post:
      tags: [PasswordSecurity]
      summary: Invalidate all API keys for user
      description: Called when user is blocked or deleted
```

---

### 13. SCIM User Provisioning

| Field | Value |
|-------|-------|
| **PropelAuth has** | Full SCIM 2.0 user provisioning |
| **Our state** | SCIM groups exist but no SCIM user endpoints |
| **Impact** | B2B - enterprise customers use SCIM to auto-provision users |
| **Effort** | MEDIUM (3-4 hours) |

**API Change - `org-mgmt/openapi.yaml`:**

```yaml
tags:
  - name: SCIM
    description: SCIM 2.0 user provisioning

paths:
  /{org_id}/scim/users:
    get: { tags: [SCIM], summary: List users }
    post: { tags: [SCIM], summary: Create user }
  /{org_id}/scim/users/{user_id}:
    put: { tags: [SCIM], summary: Update user }
    delete: { tags: [SCIM], summary: Remove user }
```

---

## Effort Summary

| Service | New Endpoints | Effort |
|---------|--------------|--------|
| `identity-login-service` | 5-6 | MEDIUM (1-2 days) |
| `identity-session-service` | 3-4 | MEDIUM (1-2 days) |
| `identity-user-mgmt-service` | 1 | LOW (30 min) |
| `org-mgmt` | 4 | MEDIUM (3-4 hours) |
| **Total** | **~14 endpoints** | **~3-5 days** |

---

## Not Building (Out of Scope)

| Feature | Why not |
|---------|---------|
| Hosted UI pages | Frontend work, not API. We're API-first. |
| Enterprise SSO (Okta/Entra/OneLogin) | HIGH effort. We have raw SAML which covers 80% of use cases. |
| Transactional email/SMS delivery | Infrastructure, not API spec. Done separately. |

---

## Dual OTP Source of Truth

The Dual OTP concept (login with BOTH email AND phone OTP) originated in PriceWhisperer's IDAM microservice as a fraud prevention mechanism. The flow:

1. User provides email + phone during registration/login
2. **Dual OTP** sends OTP to BOTH channels simultaneously
3. User must verify BOTH codes before access is granted
4. Abstract API validates both email reputation and phone number validity BEFORE sending
5. If either channel fails validation, the entire flow is rejected

This is documented in:
- `~/Workspace/microscaler/PriceWhisperer/microservices/trader/idam/EMAIL_VALIDATION_ANALYSIS.md`
- `~/Workspace/microscaler/PriceWhisperer/microservices/trader/idam/PHONE_VALIDATION_ANALYSIS.md`
- `~/Workspace/microscaler/PriceWhisperer/microservices/trader/idam/src/controllers/send_dual_otp.rs`

Our sesame-idam `identity-login-service` already implements this as `POST /login/dual-otp` and `POST /verify/dual-otp`.

---

## Next Steps (Recommended Order)

1. **P1.6**: Add direct token creation to `identity-session-service` (1-2 hours) - immediate developer DX win
2. **P0.1**: Add signup validation to `identity-login-service` (1-2 hours) - UX improvement
3. **P2.7**: Clarify clear-password semantics in `identity-user-mgmt-service` (30 min) - cleanup
4. **P1.4**: Add step-up MFA to `identity-login-service` (2-3 hours) - security hardening
5. **P2.10**: Add hosted password reset token check to `identity-login-service` (1-2 hours) - UX
6. **P1.5**: Add user impersonation to `identity-session-service` (3-4 hours) - support tooling
7. **P2.8**: Add passwordless magic link to `identity-login-service` (2-3 hours) - auth expansion
8. **P3.11**: Add SMS passwordless to `identity-login-service` (1-2 hours) - auth expansion
9. **P3.12**: Add invalidate-all-keys endpoint (2-3 hours) - security
10. **P3.13**: Add SCIM user provisioning to `org-mgmt` (3-4 hours) - B2B
11. **P2.9**: Re-add MCP authentication (3-4 hours) - if AI integrations planned
