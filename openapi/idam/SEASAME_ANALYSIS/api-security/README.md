# API Security

> **Component:** API key lifecycle, M2M authentication, key rotation, archival, and validation
> **Priority:** P0 — Machine-to-machine auth is foundational for microservice architectures
> **Service:** api-keys (11 endpoints, 1,354 lines)

---

## The Pitch

**Buyer Question:** *Can I issue, rotate, validate, and revoke API keys for machine-to-machine communication with full lifecycle management, tenant isolation, and audit trails?*

If the answer is yes, you have production-grade API security for microservices. API keys are the primary mechanism for service-to-service authentication in distributed systems. They must support secure generation, rotation without downtime, archival for compliance, and real-time validation. In a multi-tenant environment, they must also respect tenant isolation boundaries.

---

## What This Component Does

API Security manages the complete lifecycle of machine-to-machine credentials:

1. **API Key Generation** — Create cryptographically secure API keys with configurable permissions and expiry
2. **Key Rotation** — Rotate keys without service disruption using dual-key validation
3. **Key Archival** — Soft-delete keys with archival for compliance and forensic analysis
4. **Key Validation** — Real-time API key validation with permission resolution
5. **Personal API Keys** — User-owned API keys for programmatic access (like GitHub personal access tokens)
6. **Organization API Keys** — Org-owned API keys for shared service accounts
7. **Key Usage Analytics** — Track API key usage patterns, rate limits, and abuse detection
8. **Key Scoping** — Limit keys to specific resources, actions, or scopes
9. **Key Expiry Management** — Configure key lifetime, auto-renewal, and expiry notifications
10. **Key Revocation** — Immediate key invalidation with audit logging

---

## Entity Model

### API Key Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | API key identifier (public prefix) |
| `key_hash` | String (255) | Yes | SHA-256 hash (never stored raw) |
| `key_prefix` | String (255) | Yes | Public prefix for identification |
| `name` | String (255) | Yes | Human-readable key name |
| `type` | Enum: [personal, org, system] | Yes | Key ownership type |
| `user_id` | UUID | No | Owner user (for personal keys) |
| `org_id` | UUID | No | Owner organization |
| `tenant_id` | UUID | Yes | Tenant isolation scope |
| `permissions` | Array[String] | Yes | Permission scope |
| `scopes` | Array[String] | No | Resource scope restrictions |
| `is_active` | Boolean | Yes | Whether key is active |
| `is_archived` | Boolean | No | Whether key is archived |
| `expires_at` | DateTime | Yes | Key expiration timestamp |
| `last_used_at` | DateTime | No | Last usage timestamp |
| `created_at` | DateTime | Yes | Creation timestamp |
| `created_by` | UUID | No | Creator principal |
| `archived_at` | DateTime | No | Archive timestamp |
| `archived_reason` | String (512) | No | Reason for archival |
| `usage_count` | Integer | No | Number of API calls made |
| `rate_limit` | Integer | No | Requests per minute |

### API Key Create Request

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `name` | String (255) | Yes | Key display name |
| `permissions` | Array[String] | Yes | Permission strings |
| `scopes` | Array[String] | No | Resource scope restrictions |
| `expires_in_days` | Integer | No | Key lifetime (default: 365) |
| `type` | Enum: [personal, org] | No | Ownership type (default: personal) |
| `metadata` | JSON | No | Custom key attributes |

### API Key Validation Response

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `api_key_id` | UUID | Yes | Validated key identifier |
| `user_id` | UUID | No | Associated user |
| `org_id` | UUID | No | Associated org |
| `tenant_id` | UUID | Yes | Tenant scope |
| `scope_type` | Enum: [personal, org, system] | Yes | Key ownership type |
| `permissions` | Array[String] | Yes | Granted permissions |
| `scopes` | Array[String] | No | Resource scope restrictions |
| `expires_at` | DateTime | Yes | Key expiration |
| `is_active` | Boolean | Yes | Key validity |
| `rate_limit_remaining` | Integer | No | Remaining requests this minute |

---

## Entity Relationships

```
APIKey ───┬── User (via user_id)        ← Personal key owner
          ├── Organization (via org_id)  ← Org key owner
          ├── Permission (many2many)     ← Key permissions
          └── UsageLog (one2many)        ← Key usage tracking
```

---

## Required API Endpoints

### Key CRUD

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/api-keys` | Create a new API key |
| `GET` | `/api/v1/api-keys` | List active API keys |
| `GET` | `/api/v1/api-keys/{id}` | Get API key details |
| `POST` | `/api/v1/api-keys/{id}/update` | Update API key (name, permissions) |
| `DELETE` | `/api/v1/api-keys/{id}` | Delete (revoke) an API key |

### Key Lifecycle

| Method | Endpoint | Description |
|--------|----------|-------------|
| `PUT` | `/api/v1/api-keys/{id}` | Archive an API key |
| `PUT` | `/api/v1/api-keys/{id}/unarchive` | Restore archived key |
| `PUT` | `/api/v1/api-keys/{id}/rotate` | Rotate key (issue new, deprecate old) |

### Key Validation

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/api-keys/validate` | Validate API key |
| `POST` | `/api/v1/api-keys/validate-personal` | Validate personal API key |
| `POST` | `/api/v1/api-keys/validate-org` | Validate organization API key |

### Usage and Analytics

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/api-keys/usage` | Get API key usage statistics |
| `GET` | `/api/v1/api-keys/usage/{id}` | Get specific key usage |

### Archived Keys

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/api-keys/archived` | List archived API keys |
| `POST` | `/api/v1/api-keys/import` | Bulk import API keys (migration) |

---

## Competitive Positioning

### Where Sesame-IDAM Wins
- **Dual validation during rotation** — Old and new keys work simultaneously, preventing downtime during rotation.
- **Immediate revocation** — Keys are invalidated on the next validation call (no cache delay).
- **Usage analytics** — Real-time usage tracking per key with rate limit enforcement.
- **Tenant-scoped keys** — API keys are automatically isolated by tenant_id.

### Where Sesame-IDAM Lags
- **No key rotation webhook** — No notification when keys expire or are rotated.
- **No key audit log** — No per-key audit trail showing who created, rotated, or revoked.
- **No key usage dashboard** — No visual analytics for key usage patterns.
- **No IP restrictions** — Auth0 and Okta can restrict keys to specific IP ranges.

---

## Competitive Intelligence Deep Dive

### Auth0: Machine-to-Machine Clients
Auth0's M2M clients are configured with scopes, not raw permissions. Clients are tied to applications, and permissions are mapped via scopes. **Sesame Gap:** Sesame uses direct permission strings rather than scope-based mapping.

### Okta: API Authorization
Okta's API Authorization supports OAuth 2.0 access tokens for machine identities. Custom claims allow fine-grained scoping. **Sesame Gap:** Okta's M2M model is more aligned with OAuth 2.0 standard.

### AWS Cognito: M2M Authorization
Cognito supports OAuth 2.0 client credentials flow for M2M with custom claims. **Sesame Gap:** Cognito's M2M is tightly coupled to AWS resources.

---

## Implementation Roadmap

### Phase 1: Core API Keys (Complete) — P0
1. API key creation with permissions ✅
2. Key listing and retrieval ✅
3. Key update (name, permissions) ✅
4. Key deletion/revocation ✅
5. Key validation ✅
6. Key rotation ✅
7. Key archival ✅
8. Usage tracking ✅

### Phase 2: Advanced Security (Not Implemented) — P1
1. IP-based key restrictions
2. Key usage anomaly detection
3. Key rotation webhook notifications
4. Per-key audit logging

### Phase 3: Enterprise Features (Not Implemented) — P2
1. Key usage dashboard
2. Bulk key operations (import/export)
3. Key policy templates
4. Key expiration scheduling with notifications

---

## Key Takeaway for Buyers

Sesame-IDAM's API key management is **functionally complete for basic M2M authentication**. The gap is in **advanced security features**: IP restrictions, anomaly detection, and usage dashboards. For standard microservice architectures, Sesame is sufficient. For regulated industries requiring detailed key audits and IP restrictions, the platform needs expansion.
