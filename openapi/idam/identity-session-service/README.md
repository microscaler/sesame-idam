# identity-session-service

> Port: `:???` | OpenAPI 3.1.0 | 13 paths | 56 schemas

Manages user sessions, token refresh, OIDC discovery, step-up MFA, user impersonation, direct token issuance, and MCP authentication.

## Quick Start

```bash
# Check the service
curl http://localhost:???/health

# List available API tags
# Each tag below maps to a handler group in impl/src/handlers/
```

## API Surface by Tag

### Discovery



- `GET /.well-known/jwks.json`
- `GET /.well-known/openid-configuration`

### Identity



- `GET /identity/me`
- `PATCH /identity/me`

### Impersonation

Admin user session switching

- `POST /admin/impersonate`
- `POST /admin/impersonate/restore`

### MCP

Model Context Protocol authentication

- `DELETE /mcp/agents/{agent_id}`
- `GET /mcp/agents`
- `GET /mcp/agents/{agent_id}`
- `POST /mcp/agents`
- `POST /mcp/token`
- `POST /mcp/token/validate`

### Sessions

Session management (refresh, profile, discovery, OIDC)

- `GET /identity/userinfo`
- `POST /refresh`

### StepUp

Multi-factor re-authentication for sensitive operations

- `POST /auth/verify/step-up`

### TokenIssuance

Programmatic token creation (admin/server-side)

- `POST /identity/me/token`

## Schemas (56)

| Schema | Purpose |
|--------|---------|
| `CreateUserRequest` | Schema type |
| `DualOTPCompleteResponse` | Schema type |
| `DualOTPPartialResponse` | Schema type |
| `DualOTPRequest` | Schema type |
| `DualOTPResponse` | Schema type |
| `DualOTPVerifyRequest` | Schema type |
| `EmailOTPRequest` | Schema type |
| `EmailOTPVerifyRequest` | Schema type |
| `EmployeeResponse` | Schema type |
| `Error` | Schema type |
| `ErrorResponse` | Schema type |
| `ForgotPasswordRequest` | Schema type |
| `ImpersonateRequest` | Schema type |
| `ImpersonateResponse` | Schema type |
| `ImpersonateRestoreRequest` | Schema type |
| `JWKS` | Schema type |
| `LinkSocialAccountRequest` | Schema type |
| `LoginRequest` | Schema type |
| `LogoutRequest` | Schema type |
| `McpAgent` | Schema type |
| `McpTokenRequest` | Schema type |
| `McpTokenResponse` | Schema type |
| `McpValidateRequest` | Schema type |
| `McpValidateResponse` | Schema type |
| `MfaRequiredResponse` | Schema type |
| `MfaSetupRequest` | Schema type |
| `MfaSetupResponse` | Schema type |
| `MfaVerifyRequest` | Schema type |
| `MigratePasswordRequest` | Schema type |
| `MigrateUserRequest` | Schema type |
| `OAuthLogoutRequest` | Schema type |
| `OAuthTokenResponse` | Schema type |
| `OpenIDConfiguration` | Schema type |
| `PhoneNumberRequest` | Schema type |
| `PhoneOTPRequest` | Schema type |
| `PhoneOTPVerifyRequest` | Schema type |
| `PhoneVerificationRequest` | Schema type |
| `RefreshRequest` | Schema type |
| `RegisterRequest` | Schema type |
| `ResetPasswordRequest` | Schema type |
| `SocialCallbackRequest` | Schema type |
| `SocialLoginResponse` | Schema type |
| `StepUpRequest` | Schema type |
| `StepUpResponse` | Schema type |
| `TokenIssuanceRequest` | Schema type |
| `TokenListResponse` | Schema type |
| `TokenRequest` | Schema type |
| `TokenResponse` | Schema type |
| `UpdateEmailRequest` | Schema type |
| `UpdatePasswordRequest` | Schema type |
| `UpdateUserProfileRequest` | Schema type |
| `UpdateUserRequest` | Schema type |
| `User` | Schema type |
| `UserProfile` | Schema type |
| `UserQueryItem` | Schema type |
| `UserQueryResponse` | Schema type |

## Codegen

This spec is the source of truth. Generated code lives in `gen/` and is rebuilt via:

```bash
just gen-identity-session-service
```

**Never edit files under `gen/` directly** — they are overwritten on next regeneration. Fix the OpenAPI spec instead.
