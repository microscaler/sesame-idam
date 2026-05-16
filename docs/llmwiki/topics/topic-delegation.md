---
title: Delegation & Act Claims
status: unverified
updated: 2026-05-16
sources: [Epics/06-delegation-act/delegation.md, Epics/06-delegation-act/stories/story-6.1.md, Epics/06-delegation-act/stories/story-6.2.md]
---

# Delegation & Act Claims

## Overview

Delegation support enables RFC 8693 `act` claim for service-to-service impersonation and support tool flows. The `act` claim identifies the current actor on behalf of whom the request is made, while the JWT `sub` claim identifies the subject (the user being acted upon).

## RFC 8693 Token Exchange (Story 6.1)

### Endpoint

`POST /auth/token` with `grant_type=urn:ietf:params:oauth:grant-type:token-exchange`

### Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| `subject_token` | Yes | The token being exchanged (JWT, API key, or refresh token) |
| `subject_token_type` | Yes | Type of subject token (`access_token`, `urn:ietf:params:oauth:token-type:refresh_token`) |
| `actor_token` | No | Token representing the actor (for delegation) |
| `actor_token_type` | No | Type of actor token |
| `requested_scope` | No | Scopes requested for the new token |

### Subject/Actor Validation

1. **Subject token parsing**: Extract claims from subject token (JWT/API key/refresh token)
2. **Actor token parsing** (if provided): Extract actor claims (sub, tenant, roles, permissions)
3. **Tenant match**: Actor and subject must be in the same tenant (403 Forbidden if mismatch)
4. **Can-delegate check**: Verify actor has delegation permissions:
   - `platform_admin`: can delegate any user in tenant
   - `org_admin`: can delegate users in same org only
   - `service_account` with `delegate:*` permission: delegated
   - Regular users: cannot delegate

### Scope Intersection

The new token's scope is the intersection of three scope sets:
```
new_scope = subject_scope ∩ requested_scope ∩ actor_scope
```

If intersection is empty, the request is denied.

### Act Claim Structure

When actor is present, the new token includes:
```json
{
  "sub": "subject-user-id",
  "act": {
    "sub": "actor-user-id",
    "tenant": "tenant-id",
    "roles": ["support_agent", "org_admin"]
  }
}
```

For nested delegation, `act.chain` contains the full chain of actors.

### RFC 8693 Response Fields

- `iss`: Issuer URI
- `aud`: Audience (merged from subject and actor audiences)
- `iat`: Issued-at timestamp
- `scope`: Granted scopes (intersection result)

### Metrics

- `token_exchange_total{result: "success"|"denied"|"invalid"}`: Counter per exchange outcome
- `delegation_total{action: "exchange"|"impersonation"}`: Counter per delegation type

## Support Impersonation (Story 6.2)

### Flow

1. Platform admin initiates: `POST /auth/impersonate {user_id}`
2. System validates:
   - Admin has `support_agent` role
   - Target user is in the same tenant
   - Admin is assigned to the target user's org (if org-scoped)
3. System creates delegation token with:
   - `act` claim pointing to admin
   - `impersonated_by` field for audit
   - `impersonation_scope` for session tracking
   - Short TTL (2-5 minutes)
4. Audit log entry: `{actor_id, subject_id, timestamp, action: "impersonation"}`
5. User is notified of impersonation event

### Token Structure

```json
{
  "sub": "target-user-id",
  "act": {
    "sub": "admin-user-id",
    "tenant": "tenant-id",
    "roles": ["platform_admin"]
  },
  "impersonated_by": "admin-user-id",
  "impersonation_scope": "support_session"
}
```

### Security Controls

- Cross-tenant impersonation is always blocked (403 Forbidden)
- Admin cannot delegate beyond their own permissions
- Impersonation tokens expire quickly (2-5 minutes)
- All impersonation events are logged for audit
- Admin actions during impersonation are still verified via authz-core

## Step-Up MFA (Story 6.3)

### sx.mfa_verified Claim

The `sx.mfa_verified` claim indicates whether the user has completed MFA verification in the current session. When `true`, MFA-protected actions can proceed without re-authentication. When `false` or absent, the user must complete step-up MFA.

### MFA-Protected Actions

Six actions require `sx.mfa_verified = true`:

| Action | Risk Level | Description |
|--------|-----------|-------------|
| `admin:create_org` | Critical | Create new organization |
| `org:config:update` | High | Update org configuration/SSO |
| `admin:impersonate` | Critical | Impersonate another user |
| `api_key:create` | High | Create new API key |
| `api_key:revoke` | High | Revoke existing API key |
| `role:assign` | High | Assign/modify roles |

### Step-Up MFA Endpoint

`POST /auth/verify/step-up`

### mfa_type Strength Requirements (F-016)

| Action Risk | Allowed mfa_type | Blocked |
|------------|------------------|---------|
| Critical | TOTP, WebAuthn | SMS |
| High | TOTP, WebAuthn, SMS | |
| Normal | All types | |

### F-006 Fix: Old Refresh Token Invalidation

When a user completes step-up MFA:
1. Old refresh token is denylisted in Redis (`jti_denylist:{old_jti}` with TTL matching token expiry)
2. New refresh token is issued with `mfa_verified: true`
3. This prevents replay of the pre-MFA token even if intercepted

### Token Claim After Step-Up

```json
{
  "sx": {
    "mfa_verified": true,
    "mfa_type": "totp",
    "mfa_verified_at": 1715000000
  }
}
```

## Wiki References

- **Related stories**: Story 4.4 (route classification includes delegated actions), Story 5.1 (token versioning on delegation), Story 5.2 (version cache for elevated risk delegated actions)
- **Intersects with**: Story 4.2 (JWT common-path middleware validates act claim), Story 4.3 (delegated actions use online-only fallback)
