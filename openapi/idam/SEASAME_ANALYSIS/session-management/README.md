# Session Management

> **Component:** Token lifecycle — JWT issuance, refresh, step-up authentication, impersonation, MCP session registration
> **Priority:** P0 — Session tokens are the currency of trust in any identity system
> **Service:** identity-session-service (16 endpoints, 2,697 lines)

---

## The Pitch

**Buyer Question:** *Can I issue short-lived access tokens with self-contained JWT claims, manage token refresh securely, and enforce step-up authentication for sensitive operations?*

If the answer is yes, you have a production-grade identity platform. Tokens are the currency of trust — they carry your identity across the network, authorize your actions, and must be managed with surgical precision. Too short and users face constant re-authentication. Too long and stolen tokens are a security nightmare. Session management is where identity meets operations.

---

## What This Component Does

Session Management is the engine that issues, validates, and revokes tokens throughout the user lifecycle:

1. **JWT Access Token Issuance** — Generate short-lived (15min) JWTs with user claims, roles, and permissions
2. **Refresh Token Rotation** — Issue long-lived (30day) refresh tokens with rotation to detect token theft
3. **Step-Up Authentication** — Require additional verification (MFA, re-authentication) for sensitive operations
4. **Session Impersonation** — Admin impersonation for support with temporary token generation and audit logging
5. **Session Revocation** — Revoke individual sessions, all sessions for a user, or all sessions for a tenant
6. **MCP Session Registration** — Machine-to-machine session registration for AI/ML workloads
7. **Social Account Session Linking** — Manage sessions for users who authenticated via social OAuth
8. **Session Validation** — Validate tokens, check expiry, verify signatures, and resolve principal claims
9. **Token Blacklisting** — Maintain revoked token blacklist for immediate invalidation
10. **Concurrent Session Control** — Limit concurrent sessions per user, enforce single-session policies

---

## Entity Model

### Session Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Session identifier |
| `user_id` | UUID | Yes | Associated user |
| `tenant_id` | UUID | Yes | Tenant scope |
| `org_id` | UUID | No | Organization scope |
| `client_id` | String (255) | No | OAuth client identifier |
| `refresh_token_hash` | String (255) | Yes | SHA-256 hash of refresh token |
| `ip_address` | String (45) | No | Client IP address |
| `user_agent` | String (512) | No | Client user agent |
| `device_id` | String (255) | No | Device fingerprint |
| `geo_location` | JSON | No | Geographic location data |
| `created_at` | DateTime | Yes | Session creation timestamp |
| `last_accessed` | DateTime | Yes | Last token use timestamp |
| `expires_at` | DateTime | Yes | Session expiration |
| `is_revoked` | Boolean | Yes | Whether session is revoked |
| `is_impersonated` | Boolean | No | Whether session is impersonated |
| `impersonated_by` | UUID | No | Admin who initiated impersonation |

### JWT Claims Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `sub` | String (255) | Yes | Subject (user ID) |
| `iss` | String (255) | Yes | Issuer (tenant identifier) |
| `aud` | String (255) | Yes | Audience (resource server) |
| `exp` | Integer | Yes | Expiration timestamp |
| `nbf` | Integer | No | Not before timestamp |
| `iat` | Integer | Yes | Issued at timestamp |
| `jti` | String (255) | Yes | JWT unique identifier |
| `scope` | String (512) | No | Granted scopes |
| `roles` | Array[String] | No | User roles |
| `permissions` | Array[String] | No | Effective permissions |
| `tenant_id` | UUID | Yes | Tenant scope |
| `org_id` | UUID | No | Organization scope |
| `session_id` | UUID | Yes | Associated session |

### Refresh Token Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Refresh token identifier |
| `user_id` | UUID | Yes | Associated user |
| `session_id` | UUID | Yes | Associated session |
| `token_hash` | String (255) | Yes | SHA-256 hash (never stored raw) |
| `is_active` | Boolean | Yes | Whether token is active |
| `is_rotated` | Boolean | Yes | Whether token has been rotated |
| `created_at` | DateTime | Yes | Creation timestamp |
| `expires_at` | DateTime | Yes | Expiration timestamp |
| `used_count` | Integer | No | Number of times used |
| `last_used_at` | DateTime | No | Last usage timestamp |
| `last_used_ip` | String (45) | No | Last usage IP |

### Step-Up Authentication Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Step-up challenge identifier |
| `user_id` | UUID | Yes | User being challenged |
| `session_id` | UUID | Yes | Associated session |
| `challenge_type` | Enum: [password, mfa, biometric] | Yes | Type of challenge |
| `required_level` | Integer | Yes | Authentication strength required (1-5) |
| `achieved_level` | Integer | No | Current authentication level achieved |
| `expires_at` | DateTime | Yes | Challenge expiration |
| `status` | Enum: [pending, completed, expired, failed] | Yes | Challenge status |

---

## Entity Relationships

```
Session ───┬── User (via user_id)           ← Session owner
           ├── RefreshToken (via session_id) ← Token rotation
           ├── JWT (via session_id)          ← Token issuance
           ├── StepUpChallenge (via session_id) ← MFA challenges
           └── ImpersonationLog (via session_id) ← Admin audit
```

---

## Required API Endpoints

### Token Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/session/refresh` | Refresh access token using refresh token |
| `POST` | `/api/v1/session/revoke` | Revoke a single session |
| `POST` | `/api/v1/session/revoke-all` | Revoke all sessions for a user |
| `GET` | `/api/v1/session` | List active sessions |
| `GET` | `/api/v1/session/{id}` | Get session details |

### Step-Up Authentication

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/session/step-up` | Initiate step-up authentication |
| `POST` | `/api/v1/session/step-up/verify` | Complete step-up challenge |

### Session Impersonation

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/session/impersonate` | Admin impersonate a user |
| `POST` | `/api/v1/session/impersonate/revoke` | Revoke impersonation session |

### MCP Session Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/session/mcp/register` | Register MCP session |
| `POST` | `/api/v1/session/mcp/unregister` | Unregister MCP session |
| `GET` | `/api/v1/session/mcp` | List MCP sessions |

### Social Session Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/session/social/link` | Link social account to session |
| `POST` | `/api/v1/session/social/logout` | Logout social session |

### Session Validation

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/session/validate` | Validate a token |
| `POST` | `/api/v1/session/validate-bulk` | Validate multiple tokens |
| `GET` | `/api/v1/session/blacklist` | Check token against blacklist |

---

## Competitive Positioning

### Where Sesame-IDAM Wins
- **JWT self-contained claims** — All permissions embedded in token. No database lookups needed for validation.
- **Refresh token rotation** — Detects token theft by invalidating old refresh tokens.
- **Rust-native token processing** — JWT validation in Rust is orders of magnitude faster than Node.js implementations.
- **MCP session registration** — Native support for machine-to-machine sessions in API-first architecture.

### Where Sesame-IDAM Lags
- **No device fingerprinting** — Auth0 and Okta track device fingerprints for anomaly detection.
- **No session timeouts** — No idle/inactivity timeout configuration.
- **No SSO single logout** — No coordinated logout across multiple applications.

---

## Competitive Intelligence Deep Dive

### Auth0: Token Signing
Auth0 supports RS256, HS256, ES256 JWT algorithms with configurable expiration and claims mapping. The Management API supports token introspection and revocation. **Sesame Gap:** No token introspection endpoint (RFC 7662).

### Okta: Adaptive Session Control
Okta's session control includes idle timeout, absolute timeout, device posture checks, and geographic restrictions. **Sesame Gap:** No timeout configuration, no device posture checks.

### Firebase: Token Management
Firebase provides id tokens (1hr) and custom tokens. No refresh token rotation. Token management is handled by Firebase SDKs automatically. **Sesame Gap:** Firebase's token model is simple — Sesame's rotation is more secure.

---

## Implementation Roadmap

### Phase 1: Core Session (Complete) — P0
1. JWT access token issuance ✅
2. Refresh token with rotation ✅
3. Session revocation ✅
4. Step-up authentication ✅
5. Impersonation ✅

### Phase 2: Advanced Sessions (Not Implemented) — P1
1. Token introspection (RFC 7662)
2. Device fingerprinting
3. Concurrent session limits
4. Idle/inactivity timeouts

### Phase 3: Enterprise SSO (Not Implemented) — P2
1. SLO (Single Logout) via SAML/OIDC
2. Federated session validation
3. Cross-tenant session bridging
4. Session analytics and monitoring

---

## Key Takeaway for Buyers

Sesame-IDAM's session management is **functionally complete for basic JWT issuance and refresh**. The gap is in **enterprise session controls**: introspection, device posture, and SLO. For standard web and API applications, Sesame is fully sufficient. For enterprise SSO environments requiring SLO and device trust, the platform needs expansion.
