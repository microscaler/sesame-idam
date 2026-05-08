# identity-login-service

> Port: `:???` | OpenAPI 3.1.0 | 20 paths | 29 schemas

Handles all authentication entry points: login, register, MFA, social OAuth, OTP flows, passwordless magic links, and signup validation.

## Quick Start

```bash
# Check the service
curl http://localhost:???/health

# List available API tags
# Each tag below maps to a handler group in impl/src/handlers/
```

## API Surface by Tag

### AuthFlows

Authentication flows (login, register, token exchange, OTP, social)

- `POST /login`
- `POST /login/dual-otp`
- `POST /login/email-otp`
- `POST /login/phone-otp`
- `POST /register`
- `POST /verify/dual-otp`
- `POST /verify/email-otp`
- `POST /verify/phone-otp`

### PasswordReset

Forgot/reset password flows

- `POST /forgot-password`
- `POST /reset-password`

### Passwordless

Passwordless magic link authentication

- `POST /login/magic-link`
- `POST /login/magic-link/verify`
- `POST /login/phone-magic-link`
- `POST /login/phone-magic-link/verify`

### Sessions

Token management and session lifecycle

- `GET /oauth/authorize`
- `POST /logout`
- `POST /token`

### Signup

Pre-registration validation

- `GET /signup/validate`

### SocialLogin

OAuth provider login redirects (GitHub, Google, SAML)

- `GET /social/{provider}/login`
- `POST /social/{provider}/callback`

## Schemas (29)

| Schema | Purpose |
|--------|---------|
| `DualOTPCompleteResponse` | Schema type |
| `DualOTPPartialResponse` | Schema type |
| `DualOTPRequest` | Schema type |
| `DualOTPResponse` | Schema type |
| `DualOTPVerifyRequest` | Schema type |
| `EmailOTPRequest` | Schema type |
| `EmailOTPVerifyRequest` | Schema type |
| `ErrorResponse` | Schema type |
| `ForgotPasswordRequest` | Schema type |
| `LoginRequest` | Schema type |
| `LogoutRequest` | Schema type |
| `MagicLinkRequest` | Schema type |
| `MagicLinkResponse` | Schema type |
| `MagicLinkVerifyRequest` | Schema type |
| `MfaRequiredResponse` | Schema type |
| `PendingEmailVerificationResponse` | Schema type |
| `PhoneOTPRequest` | Schema type |
| `PhoneOTPVerifyRequest` | Schema type |
| `RegisterRequest` | Schema type |
| `ResetPasswordRequest` | Schema type |
| `SignupValidationRequest` | Schema type |
| `SignupValidationResponse` | Schema type |
| `SmsMagicLinkRequest` | Schema type |
| `SmsMagicLinkResponse` | Schema type |
| `SmsMagicLinkVerifyRequest` | Schema type |
| `SocialCallbackRequest` | Schema type |
| `SocialLoginResponse` | Schema type |
| `TokenRequest` | Schema type |
| `TokenResponse` | Schema type |

## Codegen

This spec is the source of truth. Generated code lives in `gen/` and is rebuilt via:

```bash
just gen-identity-login-service
```

**Never edit files under `gen/` directly** — they are overwritten on next regeneration. Fix the OpenAPI spec instead.
