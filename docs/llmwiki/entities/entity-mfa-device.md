---
title: MFA Device Entity
status: partially-verified
updated: 2026-01-22
sources: [openapi/identity-user-mgmt-service/openapi.yaml]
---

# Entity: MFA Device

Owned by: **identity-user-mgmt-service**

## Description

Multi-factor authentication device model. Supports TOTP setup, verification, and disable.

## Schema (from OpenAPI)

|| Column | Type | Notes |
||--------|------|-------|
|| id | uuid (PK) | |
|| user_id | uuid (FK) | |
|| type | text | `totp` |
|| secret | text | Encrypted secret key |
|| is_active | boolean | |
|| label | text | Human label (e.g., "iPhone 15") |
|| tenant_id | uuid (FK) | **REQUIRED** — MFA factors scoped to tenant |
|| created_at | timestamptz | |
|| last_used_at | timestamptz | |

## API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/users/{user_id}/mfa/setup` | POST | TOTP setup |
| `/users/{user_id}/mfa/verify` | POST | MFA verify |
| `/users/{user_id}/mfa/disable` | POST | MFA disable |

## Gaps / Drift

> **Open:** Verify actual Lifeguard model against OpenAPI spec.