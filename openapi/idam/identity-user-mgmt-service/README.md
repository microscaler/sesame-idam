# identity-user-mgmt-service

> Port: `:???` | OpenAPI 3.1.0 | 25 paths | 21 schemas

User administration: CRUD, email/phone management, MFA, password resets, social linking, migrations, and password clearing for SSO-only mode.

## Quick Start

```bash
# Check the service
curl http://localhost:???/health

# List available API tags
# Each tag below maps to a handler group in impl/src/handlers/
```

## API Surface by Tag

### AccountSecurity

Enable/disable, MFA, phone, email verification


### AuthFlows



- `POST /users/{user_id}/magiclink`

### Identity



- `PUT /users/{user_id}/email`

### PasswordManagement

Password set, clear, magic link, verification


### PasswordSecurity



- `DELETE /users/{user_id}/password`
- `POST /users/{user_id}/disable`
- `POST /users/{user_id}/email/verify`
- `POST /users/{user_id}/enable`
- `POST /users/{user_id}/mfa/disable`
- `POST /users/{user_id}/mfa/setup`
- `POST /users/{user_id}/mfa/verify`
- `POST /users/{user_id}/phone`
- `POST /users/{user_id}/phone/verify`
- `POST /users/{user_id}/resend-email-confirmation`

### Sessions



- `POST /oauth/logout`
- `POST /users/{user_id}/logout-all-sessions`

### SocialAccounts

Social account linking and token management


### SocialLogin



- `GET /users/{user_id}/social/tokens`
- `GET /users/{user_id}/social/tokens/{provider}/refresh`
- `POST /users/{user_id}/social/link`

### UserMigration

Import users from external auth systems

- `POST /users/migrate`
- `POST /users/migrate-password`

### Users

User lifecycle (CRUD, lookup, search, employee view)

- `DELETE /users/{user_id}`
- `GET /users/email`
- `GET /users/query`
- `GET /users/username`
- `GET /users/{user_id}/employee`
- `POST /users`

## Schemas (21)

| Schema | Purpose |
|--------|---------|
| `CreateUserRequest` | Schema type |
| `EmployeeResponse` | Schema type |
| `Error` | Schema type |
| `LinkSocialAccountRequest` | Schema type |
| `MfaSetupRequest` | Schema type |
| `MfaSetupResponse` | Schema type |
| `MfaVerifyRequest` | Schema type |
| `MigratePasswordRequest` | Schema type |
| `MigrateUserRequest` | Schema type |
| `OAuthLogoutRequest` | Schema type |
| `OAuthTokenResponse` | Schema type |
| `PhoneNumberRequest` | Schema type |
| `PhoneVerificationRequest` | Schema type |
| `TokenListResponse` | Schema type |
| `TokenResponse` | Schema type |
| `UpdateEmailRequest` | Schema type |
| `UpdatePasswordRequest` | Schema type |
| `UpdateUserRequest` | Schema type |
| `User` | Schema type |
| `UserQueryItem` | Schema type |
| `UserQueryResponse` | Schema type |

## Codegen

This spec is the source of truth. Generated code lives in `gen/` and is rebuilt via:

```bash
just gen-identity-user-mgmt-service
```

**Never edit files under `gen/` directly** — they are overwritten on next regeneration. Fix the OpenAPI spec instead.
