---
title: Login Flow
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, service-topology-design.md]
---

# Login Flow

## Complete Flow

```
Client → POST /auth/login {email, password} →
  identity-login-service:
    1. Query PG: user by email
    2. Verify password hash (bcrypt/scrypt)
    3. Call authz-core POST /authz/principals/effective {user_id, org_id}
       authz-core:
         a. Query PG: resolve roles + permissions
         b. Return effective claims
    4. Sign JWT with all claims (RS256)
    5. Store session in PG + Redis
    6. Return {access_token, refresh_token, user}
```

## Key Points

1. **authz-core is called ONCE at login.** The resulting JWT contains all role/permission claims. Subsequent requests use the JWT directly.
2. **Login routes are NOT protected by JWT common-path authz.** They CREATE trust, not evaluate it. Authentication IS the authorization.
3. **Password hashing is the bottleneck.** CPU-bound operation. Needs to scale vertically.
4. **Session is stored in both PG and Redis.** Redis for fast refresh lookups, PG for persistence.
5. **Post-2026 hybrid model (Epic 4):** All per-request auth after login uses the hybrid model (jwt-only, jwt-with-fallback, online-only), NOT authz-core for every request.

## Variants

| Variant | Flow |
|---------|------|
| **Email+Password** | `POST /auth/login` → `POST /auth/token` (if MFA required) |
| **Email OTP** | `POST /auth/login/email-otp` → `POST /auth/verify/email-otp` |
| **Phone OTP** | `POST /auth/login/phone-otp` → `POST /auth/verify/phone-otp` |
| **Dual OTP** | `POST /auth/login/dual-otp` → `POST /auth/verify/dual-otp` (email + phone) |
| **Social OAuth** | `GET /auth/social/{provider}/login` → redirect → `POST /auth/social/{provider}/callback` |
| **Email Magic Link** | `POST /auth/login/magic-link` → click link → `POST /auth/login/magic-link/verify` |
| **SMS Magic Link** | `POST /auth/login/phone-magic-link` → click link → `POST /auth/login/phone-magic-link/verify` |

## New Auth Flows (from PropelAuth gap closure)

| Feature | Endpoints | Description |
|---------|-----------|-------------|
| **Signup Validation** | `GET /auth/signup/validate` | Pre-registration validation before form submission |
| **Step-Up MFA** | `POST /auth/verify/step-up` | Re-authenticate for sensitive operations |
| **Direct Token Issuance** | `POST /identity/me/token` | Admin issues tokens programmatically |
| **MCP Auth** | `POST /mcp/token`, `POST /mcp/token/validate` | Model Context Protocol authentication |

## Code Anchors

- `microservices/idam/identity-login-service/impl/src/` — Login handler logic
- `openapi/identity-login-service/openapi.yaml` — Login/request/response schemas

## Gaps / Drift

> **Open:** Verify actual flow against implementation. The design doc describes the ideal flow; the actual implementation may differ.
