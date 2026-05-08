---
title: API Key Entity
status: partially-verified
updated: 2026-01-22
sources: [openapi/api-keys/openapi.yaml]
---

# Entity: API Key

Owned by: **api-keys** service

## Description

M2M API key model. Keys can be user-scoped or org-scoped. Used for server-to-server and CLI access where user sessions don't exist.

## Schema (from OpenAPI)

|| Column | Type | Notes |
||--------|------|-------|
|| id | uuid (PK) | |
|| user_id | uuid (FK, nullable) | User-scoped keys |
|| org_id | uuid (FK, nullable) | Org-scoped keys |
|| key_hash | text | SHA-256 of stored key — PG only |
|| key_prefix | text | Human-readable prefix (e.g., "sk_live_abc") |
|| display_name | text | |
|| description | text (nullable) | |
|| metadata | jsonb | Custom metadata |
|| expires_at | timestamptz (nullable) | NULL = no expiry |
|| revoked | boolean | Revocation flag |
|| application_id | uuid (FK) | **REQUIRED** — keys belong to one platform |
|| created_at | timestamptz | |
|| last_used_at | timestamptz | |

## Key Design Decisions

1. **Hash-only storage.** Only the SHA-256 hash is stored — never the plaintext key (beyond the initial prefix shown to user).
2. **Validation is fast.** Simple hash comparison — CPU trivial. Can handle tens of thousands validations/sec per core.
3. **User or org scoped.** Either a user_id or org_id can be set (not both required).

## API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api-keys` | POST | Create API key (M2M key / service account) |
| `/api-keys/current` | GET | Fetch active API keys |
| `/api-keys/{key_id}` | DELETE | Delete API key |
| `/api-keys/archived` | GET | Fetch archived (revoked/expired) API keys |
| `/api-keys/archived/{key_id}` | GET | Fetch archived API key details |
| `/api-keys/import` | POST | Import API keys from external system |
| `/api-keys/usage` | GET | Fetch API key usage |
| `/api-keys/validate` | POST | Validate API key |
| `/api-keys/validate/personal` | POST | Validate personal API key |
| `/api-keys/validate/org` | POST | Validate organisation API key |

## Gaps / Drift

> **Open:** Verify actual Lifeguard model against OpenAPI spec. Implementations may not yet support all endpoints.
