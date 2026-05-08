---
title: Session Entity
status: partially-verified
updated: 2026-01-22
sources: [openapi/identity-session-service/openapi.yaml]
---

# Entity: Session

Owned by: **identity-session-service**

## Description

User session model. Sessions are per-user AND per-application — a user has separate sessions per application. Supports token refresh, step-up MFA verification, and admin impersonation.

## Schema (from design-doc.md + OpenAPI)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| user_id | uuid (FK) | |
| tenant_id | uuid (FK) | Per-application sessions |
| session_token | text (hashed) | Also cached in Redis |
| refresh_token | text (hashed) | Also cached in Redis |
| ip_address | inet | |
| user_agent | text | |
| created_at | timestamptz | |
| expires_at | timestamptz | |
| revoked | boolean | Token revocation flag |
| last_used_at | timestamptz | |
| impersonated_by | uuid (FK, nullable) | Admin user ID if impersonating |
| step_up_verified | boolean | Step-up MFA verification flag |
| step_up_verified_at | timestamptz (nullable) | When step-up was verified |

## Key Design Decisions

1. **Per-user AND per-application.** A user logging into App A does not get a session in App B.
2. **Refresh token rotation.** On every `/refresh`, old refresh token is revoked and new one issued. Prevents token replay.
3. **Token hashing.** Both session and refresh tokens are stored hashed. Also cached in Redis for fast lookup.
4. **Step-up MFA support.** Sessions track step-up verification for sensitive operations (delete account, change email, etc.).
5. **Impersonation support.** Admins can impersonate users — tracked via `impersonated_by` field.

## New Features (from PropelAuth gap closure)

| Feature | Description |
|---------|-------------|
| **Step-Up MFA** | `POST /verify/step-up` — Re-authenticate for sensitive operations |
| **User Impersonation** | `POST /admin/users/{user_id}/impersonate` — Admin switches to user session |
| **Direct Token Issuance** | `POST /api/v1/identity/users/me/token` — Admin issues tokens programmatically |
| **MCP Auth** | `POST /mcp/token`, `POST /mcp/token/validate` — Model Context Protocol authentication |

## API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/refresh` | POST | Rotate refresh token |
| `/.well-known/openid-configuration` | GET | OIDC discovery |
| `/.well-known/jwks.json` | GET | Public key set for JWT verification |
| `/api/v1/identity/users/me` | GET | Current user profile |
| `/api/v1/identity/users/me` | PATCH | Update current user profile |
| `/api/v1/identity/users/me/userinfo` | GET | OIDC userinfo endpoint |
| `/verify/step-up` | POST | Step-up MFA verification |
| `/admin/users/{user_id}/impersonate` | POST | Admin impersonate user |
| `/admin/users/{user_id}/impersonate/restore` | POST | Restore admin session |
| `/api/v1/identity/users/me/token` | POST | Direct token issuance (admin) |
| `/mcp/token` | POST | MCP auth token |
| `/mcp/token/validate` | POST | MCP token validation |
| `/api/v1/platform/mcp/agents` | GET | List MCP agents |
| `/api/v1/platform/mcp/agents` | POST | Create MCP agent |
| `/api/v1/platform/mcp/agents/{agent_id}` | GET | Get MCP agent |
| `/api/v1/platform/mcp/agents/{agent_id}` | DELETE | Delete MCP agent |

## Code Anchors

- `microservices/idam/identity-session-service/impl/src/models/` — Lifeguard entity definition
- `openapi/identity-session-service/openapi.yaml` — Refresh/logout endpoints

## Gaps / Drift

> **Open:** Verify actual Lifeguard model. New endpoints (step-up MFA, impersonation, MCP) are in specs but implementations may not exist yet.
