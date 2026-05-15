---
title: MFA (Multi-Factor Authentication)
status: unverified
updated: 2026-05-16
sources: [Epics/06-delegation-act/stories/story-6.3.md, design-doc.md, entities/entity-mfa-device.md]
---

# MFA (Multi-Factor Authentication)

## Overview

MFA in Sesame-IDAM supports multiple verification methods (TOTP, WebAuthn, SMS) with strength-based enforcement. The `sx.mfa_verified` claim gates high-consequence actions, and step-up MFA allows users to strengthen their session after login.

## sx.mfa_verified Claim

### Claim Structure

```json
{
  "sx": {
    "mfa_verified": true,
    "mfa_type": "totp",
    "mfa_verified_at": 1715000000
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `mfa_verified` | boolean | Whether MFA was verified in this session |
| `mfa_type` | string | Method used: `totp`, `webauthn`, `sms` |
| `mfa_verified_at` | int64 | Unix timestamp of verification |

### Behavior

- `mfa_verified: true` — User completed MFA in current session; MFA-protected actions can proceed
- `mfa_verified: false` or absent — User has NOT completed MFA in current session; step-up MFA required for protected actions
- **Not a persistent flag**: The claim is set per-session during login/token exchange. A user with MFA enabled may still have `mfa_verified: false` if they logged in without MFA.

## Step-Up MFA Flow

### Endpoint

`POST /auth/step-up/mfa`

### Request

```json
{
  "mfa_type": "totp",
  "code": "123456"
}
```

### Response

```json
{
  "mfa_verified": true,
  "mfa_type": "totp",
  "mfa_verified_at": 1715000000,
  "access_token": "<new_token_with_mfa_verified_claim>",
  "refresh_token": "<new_refresh_token>"
}
```

### Flow

1. User completes login (no MFA verified yet)
2. User attempts MFA-protected action (e.g., `admin:impersonate`)
3. Handler checks `sx.mfa_verified` — it's false, returns 401 with `error="mfa_required"`
4. User calls `POST /auth/step-up/mfa` with their MFA code
5. Service verifies the code against the user's registered MFA devices
6. On success: issues new access token and refresh token with `sx.mfa_verified: true`
7. Old refresh token is denylisted (F-006 fix)
8. User retries the protected action — now allowed

## mfa_type Strength Requirements (F-016)

### Strength Levels

| Method | Strength | Phishing-Resistant | Requirements |
|--------|----------|--------------------|-------------|
| WebAuthn | Strong | Yes | Hardware key or platform authenticator |
| TOTP | Medium | No | 6-digit code from authenticator app (time-based) |
| SMS | Weak | No | 6-digit code sent via SMS |

### Risk-Based Enforcement

| Action Risk Level | Allowed mfa_type | Blocked |
|------------------|------------------|---------|
| Critical | `totp`, `webauthn` | `sms` |
| High | `totp`, `webauthn`, `sms` | — |
| Normal | All types | — |

| Action | Risk Level | mfa_type Requirement |
|--------|-----------|---------------------|
| `admin:create_org` | Critical | TOTP or WebAuthn |
| `admin:impersonate` | Critical | TOTP or WebAuthn |
| `org:config:update` | High | TOTP, WebAuthn, or SMS |
| `api_key:create` | High | TOTP, WebAuthn, or SMS |
| `api_key:revoke` | High | TOTP, WebAuthn, or SMS |
| `role:assign` | High | TOTP, WebAuthn, or SMS |

## F-006 Fix: Refresh Token Invalidation on Step-Up

When a user completes step-up MFA:

1. **Old refresh token is denylisted**: `jti_denylist:{old_jti}` with TTL matching token expiry
2. **New refresh token issued**: Contains `mfa_verified: true` in the associated claims
3. **Rationale**: Prevents replay of the pre-MFA token if intercepted during step-up
4. **Implementation**: The old token's `jti` is added to the denylist cache (Story 5.3)

## MFA Devices (Entity)

### entity-mfa-device

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique device identifier |
| `user_id` | UUID | Owner user |
| `tenant_id` | UUID | Tenant scope |
| `type` | enum | `totp`, `webauthn`, `sms` |
| `label` | string | Human-readable label |
| `is_primary` | boolean | Primary device for this type |
| `created_at` | timestamp | Device creation time |
| `last_used_at` | timestamp | Last successful verification |

### TOTP Configuration

- Time window: 30 seconds
- Drift: ±1 window (allows for clock skew)
- Code length: 6 digits
- Algorithm: SHA-1 (RFC 6238)
- Seed: 20-byte base32-encoded

### WebAuthn Configuration

- Authenticator type: `platform` or `cross-platform`
- Attestation: `none` (privacy-friendly)
- User verification: `required`

## SMS Configuration

- Code length: 6 digits
- TTL: 5 minutes
- Rate limit: 3 attempts per 5 minutes per device
- Max codes sent per day: 10 per user
- **Not allowed for critical actions** (F-016)

## Security Considerations

- **Clock skew tolerance**: TOTP allows ±1 window (60 seconds total)
- **Brute force protection**: Account lockout after 5 failed attempts
- **Rate limiting**: Step-up endpoint rate-limited to 10 requests per minute per user
- **SMS cost**: SMS codes incur cost; users should prefer TOTP/WebAuthn
- **Session binding**: MFA-verified claim is session-specific; cannot be transferred to another session

## Wiki References

- **Related stories**: Story 6.3 (step-up MFA flow), Story 5.1 (token versioning during MFA change)
- **Intersects with**: Story 6.2 (impersonation requires step-up MFA), Story 4.4 (MFA-protected actions use online-only route category)
