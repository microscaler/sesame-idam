---
title: API Key Entity
status: verified
updated: 2026-05-16
sources: [openapi/api-keys/openapi.yaml]
---

# Entity: API Key

Owned by: **api-keys** service

## Description

M2M API key model. Keys can be user-scoped or org-scoped. Used for server-to-server and CLI access where user sessions don't exist.

## Schema (from impl/ crate — api-keys)

| Column | Type | Notes |
||--------|------|-------|
| id | uuid (PK) | |
| key_hash | text | SHA-256 of stored key |
| key_prefix | varchar(16) | Human-readable prefix (e.g., "sk_live_abc") |
| name | varchar(255) | Key name |
| tenant_id | varchar(255) | **REQUIRED** — keys belong to one platform |
| user_id | uuid (FK -> users, nullable) | User-scoped keys |
| org_id | uuid (FK -> orgs, nullable) | Org-scoped keys |
| permissions | text (nullable) | JSON string of permissions (NOT an array column) |
| expires_at | timestamptz (nullable) | NULL = no expiry |
| active | boolean | Revocation status (not "revoked") |
| created_at | timestamptz | |
| updated_at | timestamptz | |

## Key Design Decisions

1. **Hash-only storage.** Only the SHA-256 hash is stored — never the plaintext key (beyond the initial prefix shown to user).
2. **Validation is fast.** Simple hash comparison — CPU trivial. Can handle tens of thousands validations/sec per core.
3. **User or org scoped.** Either a user_id or org_id can be set (not both required).
4. **Permissions stored as JSON string.** `permissions` column is TEXT (JSON serialization), not a native array or separate table.
5. **No `updated_at` in wiki, but exists in impl.** The impl model tracks updates.

## API Endpoints

| Endpoint | Method | Purpose |
| Service | Endpoint | Purpose |
|---------|----------|---------|
| api-keys | `POST /api-keys` | Create API key (M2M key / service account) |
| api-keys | `GET /api-keys/archived` | Fetch archived (revoked/expired) API keys |
| api-keys | `GET /api-keys/archived/{key_id}` | Fetch archived API key details |
| api-keys | `GET /api-keys/current` | Fetch active API keys |
| api-keys | `POST /api-keys/import` | Import API keys from external system |
| api-keys | `GET /api-keys/usage` | Fetch API key usage |
| api-keys | `POST /api-keys/validate` | Validate API key |
| api-keys | `POST /api-keys/validate/org` | DEPRECATED: Validate organisation API key |
| api-keys | `POST /api-keys/validate/personal` | DEPRECATED: Validate personal API key |
| api-keys | `PUT /api-keys/{key_id}` | Update API key metadata |
| api-keys | `DELETE /api-keys/{key_id}` | Delete API key |

## Drift Found (verified 2026-05-16)

| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|
| `display_name` column | NOT in impl (replaced by `name`) | Medium — wiki overstates naming fields |
| `description` column | NOT in impl | Medium |
| `metadata` jsonb | NOT in impl | Medium |
| `revoked` column | Actual column is `active` (inverted semantics) | High — wiki had wrong column name |
| `tenant_id` is uuid | `tenant_id` is varchar(255), not uuid | Low — type mismatch |
| `last_used_at` column | NOT in impl | Medium — usage tracking not in DB |
| `key_prefix` is text | `key_prefix` is varchar(16) | Low — length limit differs |
| No `permissions` column in wiki | EXISTS as TEXT (JSON string) | High — wiki completely missed permissions field |
| No `updated_at` column in wiki | EXISTS in impl | Medium |
| No `api_key_usage` entity | EXISTS in impl (id, key_id FK, endpoint, method, tenant_id, ip, created_at) | High — missing from wiki entirely |
| No `archived_api_key` entity | EXISTS in impl (id, key_hash, key_prefix, name, reason, archived_at) | High — missing from wiki entirely |
