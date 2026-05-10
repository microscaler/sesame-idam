# Sesame-IDAM Frontend Client

TypeScript client for direct browser usage. Generated from merged OpenAPI specs filtered to `x-consumer: frontend` endpoints only.

## Contents

- **53 endpoints** across 15 service classes
- **60 TypeScript model types** for request/response schemas
- **Axios-based HTTP client** with typed promises
- **6 core utilities** (API errors, cancelable promises, request builder)

## Generated From

Merged OpenAPI spec (`frontend-api.yaml`) with `x-consumer: frontend` filter applied.

**Sources:** 6 backend services (120 total endpoints)
**Included:** 53 frontend endpoints (44.2%)
**Excluded:** 67 non-frontend endpoints (55.8%)

## Directory Structure

```
idam-frontend/
├── core/              # HTTP client utilities
│   ├── ApiError.ts    # Typed error class
│   ├── ApiRequestOptions.ts
│   ├── ApiResult.ts
│   ├── CancelablePromise.ts
│   ├── OpenAPI.ts     # Configuration types
│   └── request.ts     # Axios request wrapper
├── models/            # 60 TypeScript types
│   ├── ApiKey.ts
│   ├── CreateApiKeyRequest.ts
│   ├── LoginRequest.ts
│   ├── TokenResponse.ts
│   └── ...
├── services/          # 15 service classes
│   ├── AuthFlowsService.ts
│   ├── ApiKeysService.ts
│   └── ...
└── index.ts           # Re-exports all services and models
```

## Service Classes

| Service | Methods | Purpose |
|---------|---------|---------|
| **AuthFlowsService** | 8 | Login, register, OTP flows (email/phone/dual) |
| **ApiKeysService** | 7 | CRUD for user API keys, usage stats |
| **PasswordSecurityService** | 7 | MFA setup/verify, email/phone verification |
| **SessionsService** | 7 | Token refresh, logout, OAuth, session management |
| **SocialLoginService** | 4 | OAuth login, social account linking, token management |
| **PasswordlessService** | 4 | Magic link authentication (email + SMS) |
| **RolesService** | 3 | List/get roles and their permissions |
| **IdentityService** | 3 | User profile CRUD (get, update email) |
| **DiscoveryService** | 2 | OpenID Connect discovery, JWKS endpoint |
| **ApplicationsService** | 2 | List/get applications |
| **PasswordResetService** | 2 | Forgot password, reset password |
| **SignupService** | 1 | Signup data validation |
| **OrganizationsService** | 1 | Fetch organization details |
| **StepUpService** | 1 | Step-up authentication verification |
| **PermissionsService** | 1 | List available permissions |

## Included Endpoints

### Authentication (23 endpoints)
- `POST /login` — Password login
- `POST /login/email-otp` — Send email OTP
- `POST /verify/email-otp` — Verify email OTP
- `POST /login/phone-otp` — Send SMS OTP
- `POST /verify/phone-otp` — Verify SMS OTP
- `POST /login/dual-otp` — Send OTP to email + phone
- `POST /verify/dual-otp` — Verify dual OTP
- `POST /register` — User registration
- `POST /token` — Token refresh / OAuth exchange
- `POST /forgot-password` — Request password reset
- `POST /reset-password` — Reset password with token
- `POST /logout` — Logout current session
- `GET /social/{provider}/login` — Initiate social OAuth
- `GET /oauth/authorize` — OAuth authorization endpoint
- `GET /signup/validate` — Validate signup data
- `POST /login/magic-link` — Send magic link (email)
- `POST /login/magic-link/verify` — Verify magic link
- `POST /login/phone-magic-link` — Send magic link (SMS)
- `POST /login/phone-magic-link/verify` — Verify SMS magic link
- `POST /refresh` — Refresh access token
- `GET /.well-known/openid-configuration` — OpenID discovery
- `GET /.well-known/jwks.json` — Public JWKS
- `GET /api/v1/identity/users/me/userinfo` — Get user info

### User Self-Service (13 endpoints)
- `GET /api/v1/identity/users/me` — Get user profile
- `PATCH /api/v1/identity/users/me` — Update user profile
- `PUT /users/{user_id}/email` — Update user email
- `POST /users/{user_id}/email/verify` — Verify user email
- `POST /users/{user_id}/resend-email-confirmation` — Resend verification
- `POST /users/{user_id}/mfa/setup` — Setup TOTP MFA
- `POST /users/{user_id}/mfa/verify` — Verify MFA code
- `POST /users/{user_id}/mfa/disable` — Disable MFA
- `POST /users/{user_id}/phone` — Setup phone number
- `POST /users/{user_id}/phone/verify` — Verify phone number
- `GET /users/{user_id}/social/tokens` — Fetch OAuth tokens
- `GET /users/{user_id}/social/tokens/{provider}/refresh` — Fresh OAuth token
- `POST /users/{user_id}/social/link` — Link social account
- `POST /users/{user_id}/logout-all-sessions` — Logout all sessions
- `POST /oauth/logout` — Logout OAuth session
- `POST /verify/step-up` — Verify step-up authentication

### API Keys (7 endpoints)
- `POST /` — Create API key
- `GET /current` — List active API keys
- `PUT /{key_id}` — Update API key
- `DELETE /{key_id}` — Delete API key
- `GET /usage` — Check API key usage stats
- `GET /archived` — List archived API keys
- `GET /archived/{key_id}` — Get archived API key

### Organization & App Management (7 endpoints)
- `GET /{org_id}` — Fetch organization details
- `GET /api/v1/am/applications` — List applications
- `GET /api/v1/am/applications/{app_id}` — Get application
- `GET /api/v1/am/applications/{app_id}/roles` — List roles
- `GET /api/v1/am/applications/{app_id}/roles/{role_id}` — Get role
- `GET /api/v1/am/applications/{app_id}/roles/{role_id}/permissions` — Get role permissions
- `GET /api/v1/am/applications/{app_id}/permissions` — List permissions

## Excluded Endpoints

### Admin Console (41 endpoints)
User/org management, SSO configuration, role/permission CRUD, SCIM provisioning, webhook management, org migration, and other administrative operations.

Examples:
- `POST /users` — Create user (admin)
- `DELETE /users/{user_id}` — Delete user (admin)
- `PUT /{org_id}` — Update organization (admin)
- `DELETE /{org_id}` — Delete organization (admin)
- `POST /{org_id}/invite-user` — Invite user to org (admin)
- `POST /{org_id}/saml` — Enable SAML for org (admin)
- All SCIM endpoints (SCIM provisioner only)

### Backend Service (13 endpoints)
Internal service-to-service operations: key validation, authorization checks, user migration, RBAC management.

Examples:
- `POST /validate` — Validate API key (called by backend services)
- `POST /validate/personal` — Validate personal API key
- `POST /validate/org` — Validate org API key
- `POST /authorize` — Check authorization (authz middleware)
- `POST /principal/effective` — Get effective permissions
- `POST /migrate` — Migrate user (backend only)

### Agent (5 endpoints)
MCP (Model Context Protocol) agent management — never called from browser.

Examples:
- `POST /mcp/token` — Generate agent token
- `POST /mcp/validate` — Validate agent token
- `POST /mcp/agents` — List/create/delete agents

### External Provider (1 endpoint)
OAuth provider callback — called by OAuth providers (GitHub, Google), not the frontend.

- `POST /social/{provider}/callback` — OAuth callback endpoint

## Usage

### Installation

```bash
npm install @sesame-idam/frontend
# or
yarn add @sesame-idam/frontend
```

### Basic Usage

```typescript
import { OpenAPI, AuthFlowsService, IdentityService } from '@sesame-idam/frontend';

// Configure the client
OpenAPI.BASE = 'https://idam.example.com';
OpenAPI.TOKEN = async () => {
  return localStorage.getItem('access_token');
};

// Login
const response = await AuthFlowsService.authLogin({
  email: 'alice@example.com',
  password: 'SecureP@ss123!',
});

// Get user profile
const profile = await IdentityService.usersMeGet();

// Create API key
const apiKey = await ApiKeysService.createApiKey({
  name: 'My App Key',
  permissions: ['read', 'write'],
});
```

### Typed Imports

```typescript
import type { LoginRequest, TokenResponse, User } from '@sesame-idam/frontend';

// TypeScript will validate your request/response types
const loginRequest: LoginRequest = {
  email: 'alice@example.com',
  password: 'SecureP@ss123!',
};

const response: TokenResponse = await AuthFlowsService.authLogin(loginRequest);
```

### Error Handling

```typescript
try {
  const response = await AuthFlowsService.authLogin({
    email: 'alice@example.com',
    password: 'wrong-password',
  });
} catch (error) {
  if (error instanceof ApiError) {
    console.log('Status:', error.statusCode);
    console.log('Message:', error.message);
  }
}
```

### Cancelable Requests

```typescript
const cancelable = await AuthFlowsService.authLogin({
  email: 'alice@example.com',
  password: 'SecureP@ss123!',
});

// Cancel the request if needed
cancelable.cancel();
```

## Configuration

```typescript
import { OpenAPI } from '@sesame-idam/frontend';

OpenAPI.configure({
  BASE: 'https://idam.example.com',
  TOKEN: async () => {
    return localStorage.getItem('access_token');
  },
  HEADERS: {
    'X-Custom-Header': 'value',
  },
});
```

## Regeneration

To regenerate this client after OpenAPI spec changes:

```bash
openapi -i openapi/frontend-api.yaml -o clients/idam-frontend -c axios
```

The merged spec is at `/tmp/merged_specs/frontend-api.yaml` (or the canonical merged spec location).

## Notes

- This client is **frontend-only** — admin panels and service-to-service communication use other clients
- All endpoints require `X-Tenant-ID` header or authentication via Bearer token
- Token refresh and OAuth flows are handled through `SessionsService`
- MFA, email, and phone verification are handled through `PasswordSecurityService`
- Magic link authentication (passwordless) is handled through `PasswordlessService`
