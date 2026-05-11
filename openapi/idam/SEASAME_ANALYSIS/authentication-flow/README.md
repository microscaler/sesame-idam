# Authentication Flow

> **Component:** Core identity verification — login, register, MFA, social OAuth, passwordless, magic link
> **Priority:** P0 — Foundation layer; nothing works without authentication
> **Service:** identity-login-service (20 endpoints, 2,024 lines)

---

## The Pitch

**Buyer Question:** *Can I authenticate any user — via password, OTP, social OAuth, magic link, or passwordless — with security that adapts to risk level?*

If the answer is no, you don't have an identity platform — you have a form that checks a database. Authentication is the single most critical function of any identity system. It must handle every login method your users expect, enforce security without friction, and scale to millions of authentications per month without degrading performance.

---

## What This Component Does

Authentication Flow is the entry point to every identity platform. It handles:

1. **Password-based login** — Email/username + password with bcrypt/argon2 hashing, rate limiting, and lockout policies
2. **Social OAuth** — Google, GitHub, and custom OIDC providers for one-click login
3. **Email OTP** — Time-based one-time codes sent via email (60s expiry, 3-attempt limit)
4. **SMS/Phone OTP** — Same as email OTP but delivered via SMS
5. **Magic Links** — Passwordless login via one-click email links with signed JWT tokens
6. **SMS Magic Links** — Passwordless login via SMS links
7. **MFA Setup & Verification** — TOTP (Time-based One-Time Password) enrollment and challenge-response
8. **Password Recovery** — Forgot password with secure token generation, email delivery, and reset
9. **User Registration** — New user creation with email verification, password policy enforcement, and optional social account linking
10. **Logout** — Session invalidation and token revocation

---

## Entity Model

### Login Request Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `email` | String (255) | Yes | User email address |
| `password` | String (128) | Yes | Password (hashed at rest) |
| `mfa_token` | String (64) | No | TOTP code for MFA verification |
| `social_provider` | Enum: [google, github, oidc] | No | Social OAuth provider name |
| `social_code` | String (512) | No | OAuth authorization code from provider |
| `redirect_uri` | String (512) | No | OAuth redirect URI |
| `client_id` | String (255) | No | OAuth client identifier |
| `tenant_id` | UUID | Yes | Tenant isolation scope |
| `device_fingerprint` | String (255) | No | Client device fingerprint for risk analysis |
| `ip_address` | String (45) | No | Client IP address |

### Token Response Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `access_token` | String (1024) | Yes | JWT access token (15min expiry) |
| `refresh_token` | String (512) | Yes | Refresh token (30day expiry) |
| `id_token` | String (2048) | No | OIDC ID token (if OIDC flow) |
| `token_type` | Enum: [Bearer] | Yes | Token type |
| `expires_in` | Integer | Yes | Access token lifetime in seconds |
| `scope` | String (512) | No | Granted scopes |
| `mfa_required` | Boolean | No | Whether MFA is required before full access |

### MFA Setup Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `secret` | String (64) | No | TOTP secret (base32 encoded, encrypted at rest) |
| `qr_code` | String (2048) | No | QR code URI for TOTP app |
| `backup_codes` | Array[String] | No | One-time backup codes for MFA recovery |
| `verified` | Boolean | No | Whether MFA has been verified |
| `algorithm` | Enum: [SHA1, SHA256, SHA512] | No | TOTP hash algorithm |
| `digits` | Integer | No | OTP digit count (6 or 8) |
| `period` | Integer | No | TOTP period in seconds |

### Password Reset Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `token` | String (512) | Yes | One-time password reset token |
| `new_password` | String (128) | Yes | New password (hashed at storage) |
| `expires_at` | DateTime | Yes | Token expiration timestamp |
| `used` | Boolean | No | Whether the token has been consumed |
| `ip_address` | String (45) | No | IP address used for reset |

---

## Entity Relationships

```
LoginRequest → User (via email)          ← Authentication event
LoginRequest → Tenant (via tenant_id)    ← Multi-tenant isolation
TokenResponse → RefreshToken (via id)    ← Token lifecycle
TokenResponse → Principal (via claims)   ← JWT claim enrichment
MFASetup → User (via user_id)            ← MFA enrollment
PasswordReset → User (via email)         ← Recovery flow
```

---

## Required API Endpoints

### Password Authentication

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/login/password` | Password-based login with JWT issuance |
| `POST` | `/api/v1/login/register` | New user registration |
| `POST` | `/api/v1/login/logout` | Session invalidation and token revocation |

### Password Recovery

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/login/password/forgot` | Initiate password reset via email |
| `POST` | `/api/v1/login/password/reset` | Complete password reset with token |

### Social OAuth

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/login/social` | Social OAuth login (Google, GitHub, OIDC) |
| `POST` | `/api/v1/login/social/link` | Link social account to existing user |

### OTP Authentication

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/login/email-otp` | Send email OTP code |
| `POST` | `/api/v1/login/email-otp/verify` | Verify email OTP code |
| `POST` | `/api/v1/login/phone-otp` | Send SMS OTP code |
| `POST` | `/api/v1/login/phone-otp/verify` | Verify SMS OTP code |

### Magic Link

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/login/magic-link` | Send passwordless magic link email |
| `POST` | `/api/v1/login/magic-link/verify` | Verify magic link and issue tokens |
| `POST` | `/api/v1/login/sms-magic-link` | Send SMS magic link |
| `POST` | `/api/v1/login/sms-magic-link/verify` | Verify SMS magic link |

### MFA

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/login/mfa/setup` | Enroll TOTP MFA |
| `POST` | `/api/v1/login/mfa/verify` | Verify MFA code during login |

---

## Competitive Positioning

### Where Sesame-IDAM Wins
- **Rust-native login throughput** — Auth0 and Okta run on Node.js; Sesame-IDAM in Rust handles 10x more login requests per second on the same hardware.
- **Tenant-isolated auth** — Each tenant's users are hard-segmented at the database level. Auth0 requires a separate "connection" per tenant.
- **API-driven MFA** — MFA setup and verification via API, not just dashboard UI. Better for headless and programmatic workflows.

### Where Sesame-IDAM Lags
- **No branded login UI** — Auth0 provides a customizable dashboard login page with branding. Sesame requires custom frontend development.
- **No password breach monitoring** — Auth0 and Okta integrate with HaveIBeenPwned for breach detection.
- **No social provider coverage** — Auth0 supports 100+ social providers. Sesame only covers Google and GitHub.
- **No adaptive MFA** — No risk-based MFA triggering based on device, location, or behavior.

---

## Competitive Intelligence Deep Dive

### Auth0: The Gold Standard for Login UX
Auth0's Universal Login Page is the industry benchmark. It handles password, social, OTP, passwordless, and MFA in a single hosted experience with custom branding. Auth0's "Passwordless" flow (email link, SMS link, magic code) is the most polished. **Sesame Gap:** No hosted login page. Sesame provides API endpoints only — frontend development required for branded UX.

### Okta: Enterprise Login with Adaptive MFA
Okta's Adaptive MFA triggers additional verification steps based on risk signals (new device, unusual location, after-hours access). Okta Secure Password Storage integrates with HaveIBeenPwned. **Sesame Gap:** No risk-based MFA triggering. All MFA is manual enrollment, not adaptive.

### Firebase Auth: Frictionless Mobile Auth
Firebase provides drop-in UI components (FirebaseUI) for social login, phone auth, and email/password. Handles account linking and anonymous-to-registered flows automatically. **Sesame Gap:** No mobile SDKs, no drop-in UI, no account linking flows.

### Keycloak: Open-Source Login Flexibility
Keycloak offers 50+ social identity providers, theme customization, and brute-force protection via the admin console. **Sesame Gap:** Fewer social providers, no admin-console-driven brute-force protection.

---

## Implementation Roadmap

### Phase 1: Core Auth (Complete) — P0
1. Password login/register with rate limiting ✅
2. Social OAuth (Google, GitHub) ✅
3. Email OTP and phone OTP ✅
4. Magic link and SMS magic link ✅
5. MFA setup and verification (TOTP) ✅
6. Password forgot/reset flow ✅

### Phase 2: Branded UI (Not Implemented) — P2
1. Hosted login page with tenant branding
2. Customizable login form fields
3. Social provider configuration via admin API
4. Login page localization (i18n)

### Phase 3: Risk-Based Auth (Not Implemented) — P2
1. Device fingerprinting and risk scoring
2. Adaptive MFA (trigger on risk signals)
3. Geographic velocity tracking
4. IP reputation and threat intelligence integration
5. Password breach monitoring (HaveIBeenPwned)

### Phase 4: Social Provider Expansion (Not Implemented) — P3
1. Apple Sign-In
2. Facebook/Instagram
3. LinkedIn
4. Custom OIDC providers
5. SAML identity provider as login source

---

## Key Takeaway for Buyers

Sesame-IDAM's authentication flow is **functionally complete** — every login method an enterprise needs is implemented. The gap is in **hosted UX** and **risk analysis**. Auth0 and Okta win on branded login pages and adaptive MFA. Firebase wins on mobile UX.

**For developer-first organizations building headless applications or microservices**, Sesame-IDAM's API-first authentication is superior — you get the raw power without the dashboard tax. **For organizations that need a polished login experience out of the box**, Auth0 remains the best choice until Sesame-IDAM builds its branded login page feature.
