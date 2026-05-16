---
title: Session Entity
status: verified
updated: 2026-05-16
sources: [openapi/identity-session-service/openapi.yaml]
---

# Entity: Session

Owned by: **identity-session-service**

## Description

User session model. Sessions are per-user AND per-application — a user has separate sessions per application. Supports token refresh, step-up MFA verification, and admin impersonation.

**NOTE:** There are TWO session models:
1. **identity-session-service** — Full model with `mfa_verified` and `impersonated_by`
2. **identity-login-service** — Simplified model without `mfa_verified` or `impersonated_by`

## Schema (from impl/ crate — identity-session-service)

| Column | Type | Notes |
||--------|------|-------|
| id | uuid (PK) | |
| user_id | uuid (FK -> users) | |
| token | text | Also cached in Redis |
| refresh_token | text | Also cached in Redis |
| expires_at | timestamptz | |
| ip | varchar(64, nullable) | |
| user_agent | text (nullable) | |
| mfa_verified | boolean | Step-up MFA verification flag |
| impersonated_by | uuid (FK -> users, nullable) | Admin user ID if impersonating |
| created_at | timestamptz | |
| updated_at | timestamptz | |

### Simplified Session Model (identity-login-service impl)

| Column | Type | Notes |
||--------|------|-------|
| id | uuid (PK) | |
| user_id | uuid (FK -> users) | |
| token | text | |
| refresh_token | text | |
| expires_at | timestamptz | |
| ip | varchar(64, nullable) | |
| user_agent | text (nullable) | |
| created_at | timestamptz | |
| updated_at | timestamptz | |

**Missing from simplified model:** `mfa_verified`, `impersonated_by`

## Key Design Decisions

1. **Per-user AND per-application.** A user logging into App A does not get a session in App B.
2. **Two session models.** identity-session-service has a full model (`mfa_verified`, `impersonated_by`); identity-login-service has a simplified model without these fields.
3. **Token hashing.** Both session and refresh tokens are stored hashed. Also cached in Redis for fast lookup.
4. **Step-up MFA support.** Sessions track step-up verification via `mfa_verified` field (full model only).
5. **Impersonation support.** Admins can impersonate users — tracked via `impersonated_by` field (full model only).
6. **No revocation tracking in DB.** `revoked`, `last_used_at`, `step_up_verified_at` are NOT in the impl model. Token lifecycle is managed via `expires_at` + Redis.

## New Features (from PropelAuth gap closure)

| Feature | Description |
|---------|-------------|
| **Step-Up MFA** | `POST /auth/verify/step-up` — Re-authenticate for sensitive operations |
| **User Impersonation** | `POST /admin/impersonate` — Admin switches to user session |
| **Direct Token Issuance** | `POST /identity/me/token` — Admin issues tokens programmatically |
| **MCP Auth** | `POST /mcp/token`, `POST /mcp/token/validate` — Model Context Protocol authentication |

## API Endpoints

| Endpoint | Method | Purpose |
| Service | Endpoint | Purpose |
|---------|----------|---------|
| identity-session-service | `GET /.well-known/jwks.json` | JWKS for JWT verification |
| identity-session-service | `GET /.well-known/openid-configuration` | OIDC discovery |
| identity-session-service | `POST /admin/impersonate` | Impersonate user |
| identity-session-service | `POST /admin/impersonate/restore` | Restore admin session |
| identity-session-service | `POST /auth/verify/step-up` | Step-up MFA verification |
| identity-session-service | `GET /identity/me` | Current user profile |
| identity-session-service | `PATCH /identity/me` | Update current user profile |
| identity-session-service | `POST /identity/me/token` | Issue access token |
| identity-session-service | `GET /identity/userinfo` | User Info endpoint |
| identity-session-service | `GET /mcp/agents` | List agents |
| identity-session-service | `POST /mcp/agents` | Create agent |
| identity-session-service | `GET /mcp/agents/{agent_id}` | Get agent |
| identity-session-service | `DELETE /mcp/agents/{agent_id}` | Delete agent |
| identity-session-service | `POST /mcp/token` | Issue MCP auth token |
| identity-session-service | `POST /mcp/token/validate` | Validate MCP token |
| identity-session-service | `POST /session/refresh` | Refresh access token |

## Code Anchors

- `microservices/idam/identity-session-service/impl/src/models/` — Lifeguard entity definition
- `openapi/identity-session-service/openapi.yaml` — Refresh/logout endpoints

## Drift Found (verified 2026-05-16)

| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|
| `session_token` column name | Actual column is `token` | Low — naming mismatch |
| `ip_address` column name | Actual column is `ip` (varchar(64), not inet) | Low — naming/type mismatch |
| `tenant_id` column | NOT in impl | Medium — session not tenant-scoped in DB (inferred from user) |
| `revoked` column | NOT in impl | Medium — revocation managed via Redis/token expiry only |
| `last_used_at` column | NOT in impl | Low |
| `step_up_verified_at` column | NOT in impl (only `mfa_verified` boolean exists) | Medium |
| Single session model | TWO session models exist (identity-session-service + identity-login-service) | High — simplified model lacks mfa_verified and impersonated_by |
| `token`/`refresh_token` stored hashed | Stored as text (hashing handled at application layer) | Low |
