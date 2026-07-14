---
title: Login Flow
status: verified
updated: 2026-07-14
sources: [identity-login-service/impl/src/controllers/auth_login.rs, services/token_issuer.rs, services/authz_client.rs]
---

# Login Flow

## Tenant gate (2026-07-14)

All auth entry points call `TenantService::require_active` **before** credentials:

- Unknown `X-Tenant-ID` → `404 tenant_unknown`
- Non-`active` tenant → `403 tenant_not_active`

See [topic-platform-tenants.md](./topic-platform-tenants.md).

## Complete Flow (IMPLEMENTED 2026-07-06)

```
Client → POST /auth/login {email, password} + X-Tenant-ID →
  identity-login-service:
    0. TenantService::require_active(X-Tenant-ID)     [tenant gate]
    1. Query PG: user by (tenant_id, email)          [UserService]
    2. Verify password hash (argon2id)               [services::password]
    3. Call authz-core POST /authz/principals/effective  [services::authz_client, may_http, 500ms timeout]
       authz-core:
         a. Query PG: role_assignments + principal_attributes (tenant-scoped)
         b. Return effective roles (permissions pending org-mgmt mapping)
       — best-effort: on failure login proceeds with empty roles
    4. Sign JWT (EdDSA/Ed25519, typ=at+jwt) with roles in sx claims
       [sesame_common::jwt::Ed25519Signer, shared key via
        SESAME_JWT_SIGNING_KEY_PKCS8_B64 — same key session-service
        publishes in JWKS]
    5. Store refresh token metadata in Redis (refresh:{jti}, family:{sid})
       compatible with session-service rotation
    6. Return TokenResponse {access_token, refresh_token, roles, ...}
```

Failure modes: unknown user / wrong password / non-active account all
return an identical 401 `invalid_credentials` (no user enumeration).
`ver` claim comes from the Redis `VersionStore` (fallback 1 when Redis is
down).

## Key Points

1. **authz-core is called ONCE at login** (decision confirmed + implemented; Epics INDEX open question #1 resolved). The JWT carries role claims; per-request hybrid authz is Epic 4.
2. **Enrichment is best-effort.** Login availability never depends on authz-core; tokens degrade to empty roles.
3. **Login routes are NOT protected by JWT common-path authz.** They CREATE trust, not evaluate it.
4. **Password hashing is the bottleneck.** argon2id, CPU-bound. Needs to scale vertically.
5. **Refresh state is Redis-only today** (refresh:{jti} + family sets). PG persistence of sessions is not implemented.

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

- `microservices/idam/identity-login-service/impl/src/controllers/auth_login.rs` — password login (real)
- `microservices/idam/identity-login-service/impl/src/controllers/auth_register.rs` — registration (real)
- `microservices/idam/identity-login-service/impl/src/services/` — password, user_service, token_issuer, authz_client
- `microservices/idam/identity-login-service/impl/tests/bdd/auth_flow.rs` — live-DB BDD (register→login round trip, tenant isolation)
- `openapi/idam/identity-login-service/openapi.yaml` — request/response schemas

## Gaps / Drift

- Email+password login/register and **Google/Microsoft OAuth** (`social_login` + `social_callback`) are implemented (2026-07-14 MVP). OAuth uses `tenant_oauth_providers` metadata + K8s env secrets. OTP, magic links, GitHub, MFA/step-up remain stubs.
- **Platform admin API** (`/platform/tenants/*`) not yet implemented — see [PRD-P1](../../PRD-P1-platform-tenant-admin.md).
- `POST /auth/token` (refresh/token-exchange in login-service) still uses placeholder signing — real refresh goes through session-service `/auth/refresh`.
