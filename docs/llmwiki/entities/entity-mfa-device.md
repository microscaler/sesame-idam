---
title: MFA Setup Entity
status: verified
updated: 2026-05-16
sources: [openapi/*/openapi.yaml, microservices/*/impl/src/models/]
---

# Entity: MFA Setup

Owned by: **identity-user-mgmt-service** AND **identity-session-service** (identical model in both)

## Description

Multi-factor authentication device/model. Supports TOTP setup, verification, and disable. The same `mfa_setup` table exists in two services with identical schema.

## Schema (from impl/ crate — both services)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| user_id | uuid (FK -> users) | |
| factor_type | varchar(32) | TOTP, SMS, etc. |
| secret | text (nullable) | Encrypted secret key |
| enabled | boolean | Whether this factor is enabled |
| created_at | timestamptz | |
| updated_at | timestamptz | |

## Key Design Decisions

1. **Duplicate models.** The exact same `mfa_setup` model exists in both `identity-session-service` and `identity-user-mgmt-service` impls.
2. **Single factor type.** Currently only TOTP is supported (`factor_type` is varchar(32)). SMS/WebAuthn are planned but not yet in the impl.
3. **Secret stored encrypted.** The `secret` column stores encrypted TOTP secrets.
4. **Single `enabled` field.** Not `is_active` — the impl uses `enabled` boolean.

## API Endpoints

| Service | Endpoint | Purpose |
|---------|----------|---------|
| identity-user-mgmt-service | `POST /admin/users/{user_id}/mfa/setup` | Set up TOTP MFA |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/mfa/verify` | Verify MFA code |
| identity-user-mgmt-service | `POST /admin/users/{user_id}/mfa/disable` | Disable user 2FA |
| identity-session-service | `POST /auth/verify/step-up` | Step-up MFA verification |

## Drift Found (verified 2026-05-16)

| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|--------|
| Entity named `mfa_device` | Actual model is `mfa_setup` in TWO services | High — renaming needed |
| `label` column | NOT in impl (no label field) | Medium — wiki overstates |
| `is_active` column | Actual field is `enabled` (boolean) | Medium — naming mismatch |
| `last_used_at` | NOT in impl | Low — usage tracking missing |
| `tenant_id` column | NOT in impl (not in either service's mfa_setup) | Medium — scope unclear |
| `type` field | Actual field is `factor_type` | Low — naming mismatch |
| Single MFA model | TWO identical models (identity-session + identity-user-mgmt) | High — duplicate in two impls |
