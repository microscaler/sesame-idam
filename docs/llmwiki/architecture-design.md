# Sesame-IDAM Architecture Design

> **Status:** Draft — based on hauliage production reference
> **Created:** 2026-05-12
> **Last Updated:** 2026-05-12

This document describes the architecture for sesame-idam, modeled after the hauliage production pattern. It covers the database layer, entity definitions, service layer, controller layer, and deployment configuration.

---

## 1. Architecture Overview

Sesame-IDAM is an Identity and Access Management platform with **6 microservices**, **133 endpoints**, and **198 schemas**. Each service is a standalone BRRTRouter binary with its own OpenAPI spec, generated types, and implementation controllers.

### Current State

| Service | Port | Endpoints | Schemas | Access Pattern |
|---------|------|-----------|---------|----------------|
| identity-login-service | 8101 | 20 | 29 | HIGH — login, register, OAuth, OTP |
| identity-session-service | 8105 | 16 | 60 | HIGH — refresh, OIDC, MCP, impersonation |
| identity-user-mgmt-service | 8106 | 28 | 28 | MEDIUM — user CRUD, MFA, email/phone |
| authz-core | 8102 | 15 | 19 | EXTREME — every consumer API request |
| api-keys | 8103 | 11 | 17 | HIGH — M2M key validation |
| org-mgmt | 8104 | 43 | 45 | LOW — org lifecycle, SSO/SCIM, webhooks |

### Architectural Layers (per service)

```
┌─────────────────────────────────────────────────┐
│  OpenAPI Spec (openapi/idam/<service>/)         │  ← Source of truth
├─────────────────────────────────────────────────┤
│  gen/ (brrtrouter-gen generated)                │  ← Types, handlers, main.rs stubs
├─────────────────────────────────────────────────┤
│  impl/                                            │
│  ├── controllers/     ← Business logic handlers │  ← Your code (NOT regenerated)
│  ├── models/          ← Lifeguard entity structs│  ← Your code (derive LifeModel/LifeRecord)
│  ├── services/        ← Domain services (optional)│
│  ├── main.rs          ← Server, routing, middleware│
│  └── lib.rs           ← Module declarations     │
├─────────────────────────────────────────────────┤
│  database/ (hauliage_database equivalent)       │  ← Shared pool (see §2)
└─────────────────────────────────────────────────┘
```

---

## 2. Database Layer

### 2.1 LifeguardPool Pattern

Every hauliage microservice uses a shared `LifeguardPool` initialized once at process startup:

```rust
// hauliage_database/src/lib.rs
use lifeguard::{query_value, DatabaseConfig, LifeguardPool, PooledLifeExecutor};
use std::sync::{Arc, OnceLock};

static EXECUTOR: OnceLock<PooledLifeExecutor> = OnceLock::new();

pub fn pooled_executor() -> &'static PooledLifeExecutor {
    EXECUTOR.get_or_init(|| {
        let (cfg, splash) = load_pool_config();
        let pool = LifeguardPool::from_database_config(&cfg, vec![], 0)
            .unwrap_or_else(|e| panic!("LifeguardPool::from_database_config: {e}"));
        PooledLifeExecutor::new(Arc::new(pool))
    })
}

pub fn db() -> &'static PooledLifeExecutor {
    pooled_executor()
}
```

### 2.2 Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DB_HOST` | `localhost` | PostgreSQL host (detects K8s `KUBERNETES_SERVICE_HOST`) |
| `DB_PORT` | `5432` | PostgreSQL port |
| `DB_USER` | `sesame_idam` | Database user |
| `DB_PASS` | *(empty)* | Database password |
| `DB_NAME` | `sesame_idam` | Database name |
| `DB_POOL_MAX` | `10` | Max connection pool size (all 6 services share this pool) |
| `DB_REPLICA_URLS` | *(disabled)* | Read replicas (temporarily empty vec in hauliage) |

### 2.3 Pool Scaling

All 6 IDAM services share the same PostgreSQL database. The `DB_POOL_MAX` env var controls the total pool size **per process**. In production, each service's pool should be sized proportionally to its endpoint access pattern:

- `authz-core` (EXTREME): pool 30-100 (called per consumer API request)
- `identity-login-service` (HIGH): pool 20-50
- `identity-session-service` (HIGH): pool 20-50
- `api-keys` (HIGH): pool 15-30
- `identity-user-mgmt-service` (MEDIUM): pool 10-30
- `org-mgmt` (LOW): pool 5-15

---

## 3. Entity Definitions (Lifeguard ORM)

### 3.1 Entity Pattern

Each entity is a Rust struct with `LifeModel` and `LifeRecord` derives:

```rust
use lifeguard_derive::{LifeModel, LifeRecord};

#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "users"]
#[schema_name = "sesame_idam"]
pub struct User {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,
    
    #[column_type = "VARCHAR(255)"]
    pub email: String,
    
    #[column_type = "VARCHAR(255)")]
    pub tenant_id: String,
    
    #[column_type = "TIMESTAMP WITH TIME ZONE")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    #[column_type = "TIMESTAMP WITH TIME ZONE")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

Key conventions:
- **`#[primary_key]`** — marks the PK column (always UUID in sesame-idam)
- **`#[column_type = "..."]`** — PostgreSQL type (VARCHAR(n), TEXT, UUID, etc.)
- **`#[foreign_key = "..."]`** — FK constraint with ON DELETE action
- **`#[nullable]`** — marks optional fields
- **`#[table_name = "..."]`** — maps to table name
- **`#[schema_name = "..."]`** — PostgreSQL schema (saas: `sesame_idam`, self-hosted: `app`)

### 3.2 Entity Registry

Entities are registered in `models/mod.rs` via OUT_DIR include:

```rust
pub mod entity_registry {
    include!(concat!(env!("OUT_DIR"), "/entity_registry.rs"));
}
```

### 3.3 Entity Model Catalog

#### identity-login-service entities

| Entity | Table | Fields |
|--------|-------|--------|
| `User` | `users` | id, email, password_hash, tenant_id, email_verified, phone, phone_verified, created_at, updated_at |
| `Session` | `sessions` | id, user_id, token, refresh_token, expires_at, ip, user_agent, created_at, updated_at |
| `OTPToken` | `otp_tokens` | id, user_id, type (email/phone/dual), code, expires_at, attempts, created_at, updated_at |
| `MagicLinkToken` | `magic_link_tokens` | id, user_id, link, expires_at, used, created_at, updated_at |
| `SocialCredential` | `social_credentials` | id, user_id, provider, provider_user_id, access_token, refresh_token, created_at, updated_at |

#### identity-session-service entities

| Entity | Table | Fields |
|--------|-------|--------|
| `Session` | `sessions` | id, user_id, token, refresh_token, expires_at, ip, user_agent, mfa_verified, impersonated_by, created_at, updated_at |
| `MFASetup` | `mfa_setup` | id, user_id, factor_type (TOTP), secret, enabled, created_at, updated_at |
| `Impersonation` | `impersonations` | id, user_id, impersonator_id, session_id, created_at, restored_at |
| `MCPAgent` | `mcp_agents` | id, user_id, name, description, config, created_at, updated_at |
| `Token` | `tokens` | id, user_id, session_id, type, token, expires_at, created_at, updated_at |
| `UserProfile` | `user_profiles` | id, user_id, first_name, last_name, avatar_url, created_at, updated_at |

#### identity-user-mgmt-service entities

| Entity | Table | Fields |
|--------|-------|--------|
| `User` | `users` | id, email, password_hash, tenant_id, status (active/disabled/deleted), email_verified, phone, phone_verified, created_at, updated_at |
| `Employee` | `employees` | id, user_id, employee_id, department, title, manager_id, created_at, updated_at |
| `MFASetup` | `mfa_setup` | id, user_id, factor_type, secret, enabled, created_at, updated_at |
| `EmailVerification` | `email_verifications` | id, user_id, token, expires_at, created_at, updated_at |
| `SocialAccount` | `social_accounts` | id, user_id, provider, provider_user_id, access_token, refresh_token, created_at, updated_at |
| `AuditEvent` | `audit_events` | id, tenant_id, user_id, event_type, severity, actor, data, ip, user_agent, created_at |

#### authz-core entities

| Entity | Table | Fields |
|--------|-------|--------|
| `RoleAssignment` | `role_assignments` | id, principal_id, role_id, resource_type, resource_id, tenant_id, created_at, updated_at |
| `PrincipalAttribute` | `principal_attributes` | id, principal_id, key, value, tenant_id, created_at, updated_at |
| `AuditEvent` | `audit_events` | id, tenant_id, event_type, severity, actor, data, ip, created_at |
| `AuditRetentionPolicy` | `audit_retention_policies` | id, tenant_id, retention_days, enabled, created_at, updated_at |
| `Authorization` | `authorizations` | id, principal_id, action, resource, effect, tenant_id, created_at, updated_at |

#### api-keys entities

| Entity | Table | Fields |
|--------|-------|--------|
| `ApiKey` | `api_keys` | id, key_hash, key_prefix, name, tenant_id, user_id, org_id, permissions, expires_at, active, created_at, updated_at |
| `ApiKeyUsage` | `api_key_usage` | id, key_id, endpoint, method, tenant_id, ip, created_at |
| `ArchivedApiKey` | `archived_api_keys` | id, key_hash, key_prefix, name, reason, archived_at |

#### org-mgmt entities

| Entity | Table | Fields |
|--------|-------|--------|
| `Org` | `organizations` | id, name, tenant_id, status, domain, created_at, updated_at |
| `OrgMembership` | `org_memberships` | id, org_id, user_id, role, status (pending/active), created_at, updated_at |
| `OrgInvite` | `org_invites` | id, org_id, email, role, token, expires_at, created_at, accepted_at |
| `OrgDomain` | `org_domains` | id, org_id, domain, verified, created_at, updated_at |
| `SamlConnection` | `saml_connections` | id, org_id, issuer, metadata_url, sso_url, signing_cert, created_at, updated_at |
| `Application` | `applications` | id, org_id, name, client_id, client_secret, redirect_uris, created_at, updated_at |
| `Role` | `roles` | id, org_id, name, description, created_at, updated_at |
| `Permission` | `permissions` | id, org_id, name, description, resource, action, created_at, updated_at |
| `RolePermission` | `role_permissions` | id, role_id, permission_id, created_at |
| `WebhookSubscription` | `webhook_subscriptions` | id, org_id, url, events, secret, active, created_at, updated_at |
| `ScimUser` | `scim_users` | id, org_id, external_id, username, email, created_at, updated_at |

---

## 4. Controller Pattern

### 4.1 Controller Structure

Controllers are generated stubs that you fill in. They are NOT regenerated (unlike `gen/`).

```rust
// impl/src/controllers/get_current_user.rs
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use idam_service_gen::handlers::get_current_user::{Request, Response};
use lifeguard::{ColumnTrait, LifeModelTrait};

#[handler(GetCurrentUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let exec = idam_database::db();  // ← Shared pool
    let tenant_id = req.tenant_context.tenant_id;  // ← From middleware
    
    // Query using Lifeguard ORM
    let user = UserEntity::find()
        .filter(UserColumn::Id.eq(user_id))
        .one(exec);
    
    match user {
        Ok(Some(u)) => Response {
            id: Some(u.id.to_string()),
            email: Some(u.email),
            // ... map to Response fields
        },
        Ok(None) => Response { /* empty/default */ },
        Err(e) => {
            eprintln!("DB Error: {}", e);
            Response { /* error/default */ }
        }
    }
}
```

### 4.2 Controller Pattern Summary

1. **Get executor:** `let exec = idam_database::db();`
2. **Extract context:** `req.tenant_context.tenant_id` (from BRRTRouter middleware)
3. **Query via Lifeguard ORM:** `Entity::find().filter(...).one(exec)` or `...all(exec)`
4. **Handle errors:** Catch `LifeError::Other` for "not found", log other errors
5. **Return typed Response:** Map entity fields to OpenAPI-derived Response struct

### 4.3 CRUD Patterns

**CREATE:**
```rust
let mut record = UserRecord::new();
record.set_id(uuid::Uuid::new_v4());
record.set_email(req.data.email.clone());
record.set_password_hash(hash_password(&req.data.password));
record.set_created_at(chrono::Utc::now());
record.set_updated_at(chrono::Utc::now());
record.insert(exec)?;
Response { id: Some(record.id().to_string()), ... }
```

**READ (single):**
```rust
let entity = UserEntity::find()
    .filter(UserColumn::Id.eq(user_id))
    .one(exec)?;
```

**READ (list):**
```rust
let entities = UserEntity::find()
    .filter(UserColumn::TenantId.eq(tenant_id))
    .order_by_asc(UserColumn::CreatedAt)
    .limit(page_size)
    .offset(offset)
    .all(exec)?;
```

**UPDATE:**
```rust
let entity = UserEntity::find()
    .filter(UserColumn::Id.eq(user_id))
    .one(exec)?
    .ok_or_else(|| /* 404 */)?;
let mut record = UserRecord::from_model(&entity);
record.set_email(new_email);
record.set_updated_at(chrono::Utc::now());
record.update(exec)?;
```

**DELETE:**
```rust
UserEntity::find()
    .filter(UserColumn::Id.eq(user_id))
    .one(exec)?
    .map(|e| e.delete(exec));
```

### 4.4 Current Controller Status

| Service | Implemented (audit stubs) | Need real logic | Total |
|---------|--------------------------|-----------------|-------|
| identity-login-service | 11 | 9 | 20 |
| identity-session-service | 12 | 4 | 16 |
| identity-user-mgmt-service | 10 | 18 | 28 |
| authz-core | 13 | 2 | 15 |
| api-keys | 9 | 2 | 11 |
| org-mgmt | 0 | 43 | 43 |
| **Total** | **55** | **78** | **133** |

---

## 5. Service Layer (Optional but Recommended)

Hauliage keeps most logic in controllers (no separate service layer for simple CRUD). For complex flows (multi-step auth, transactional updates), extract to services:

```rust
// impl/src/services/auth_service.rs
use lifeguard::LifeExecutor;
use crate::models::user::{UserEntity, UserRecord};
use crate::models::session::{SessionEntity};

pub async fn create_session(
    exec: &impl LifeExecutor,
    user_id: uuid::Uuid,
    ip: String,
    user_agent: String,
) -> Result<String, String> {
    // 1. Generate JWT
    // 2. Create session record
    // 3. Store in Redis (for session lookup/revocation)
    // 4. Return token
}
```

---

## 6. Middleware Chain

BRRTRouter middleware order (executed in registration order):

1. **MetricsMiddleware** — Prometheus metrics
2. **MemoryMiddleware** — Memory monitoring
3. **CorsMiddleware** — CORS handling (config.yaml origins + OpenAPI x-cors)
4. **Security middleware** — JWT/API key validation (registered per scheme)
5. **Tenant context extraction** — Extracts `X-Tenant-ID` into request context

### Tenant Context

The `X-Tenant-ID` header is extracted by BRRTRouter middleware and available in controllers via:

```rust
let tenant_id = req.tenant_context.tenant_id;
```

**Public endpoints** (excluded from tenant requirement):
- `/.well-known/openid-configuration` — OIDC metadata
- `/.well-known/jwks.json` — Public signing keys

---

## 7. Security Configuration

### 7.1 Security Providers (per OpenAPI scheme)

| Scheme Type | Provider | Config |
|-------------|----------|--------|
| `http/bearer` | `JwksBearerProvider` | JWKS URL, issuer, audience, leeway, cache TTL |
| `http/bearer` | `BearerJwtProvider` | Static signature (dev/test) |
| `apiKey` | `RemoteApiKeyProvider` | Verify URL, timeout, cache TTL |
| `apiKey` | `StaticApiKeyProvider` | Static key |
| OAuth2 | `JwksBearerProvider` | Via PropelAuth or per-scheme JWKS |

### 7.2 Production Flow

1. Client sends JWT in `Authorization: Bearer <token>` header
2. `JwksBearerProvider` validates signature against JWKS endpoint
3. If valid, BRRTRouter extracts claims and populates `tenant_context`
4. Controller receives typed request with tenant context

---

## 8. OpenAPI Spec Architecture

### 8.1 Per-Spec Design

Each service has a **self-contained** OpenAPI spec:

```
openapi/idam/
├── identity-login-service/openapi.yaml    (20 endpoints, 29 schemas)
├── identity-session-service/openapi.yaml  (16 endpoints, 60 schemas)
├── identity-user-mgmt-service/openapi.yaml (28 endpoints, 28 schemas)
├── authz-core/openapi.yaml                (15 endpoints, 19 schemas)
├── api-keys/openapi.yaml                  (11 endpoints, 17 schemas)
└── org-mgmt/openapi.yaml                  (43 endpoints, 45 schemas)
```

### 8.2 Schema Duplication

Shared schemas (User, UserProfile, Org) are **duplicated** in each consuming spec's `components/schemas`. This is intentional — each spec must be self-contained for codegen.

### 8.3 X-Tenant-ID Header

All operational endpoints require `X-Tenant-ID` header:

```yaml
security:
  - TenantContext: []

components:
  securitySchemes:
    TenantContext:
      type: apiKey
      in: header
      name: X-Tenant-ID
```

---

## 9. Deployment Configuration

### 9.1 BRRTRouter Runtime Config

```rust
let config = RuntimeConfig::from_env();
may::config().set_stack_size(config.stack_size);      // Default: 65536
may::config().set_workers(config.may_workers);          // Default: number of CPUs
```

### 9.2 Configuration Loading Order

1. **CLI args** — `--spec`, `--config`, `--test-api-key`
2. **Environment variables** — `DB_*`, `BRRTR_API_KEY`, `BRRTR_BEARER_SIGNATURE`
3. **Config file** — `config.yaml` (CORS, security providers, HTTP settings)
4. **Defaults** — Built-in fallbacks for everything

### 9.3 K8s Deployment

In Kubernetes, `DB_*` and `DB_PASS` are injected via ConfigMap and Secret. The `hauliage_database` pattern auto-detects K8s service hosts:

```rust
let db_host = std::env::var("DB_HOST")
    .ok()
    .or_else(|| std::env::var("KUBERNETES_SERVICE_HOST")
        .map(|_| "postgres.data.svc.cluster.local".into()))
    .unwrap_or_else(|| "localhost".into());
```

---

## 10. Migration Strategy

Lifeguard migrations are generated from entity structs:

```bash
# In impl/Cargo.toml, add build dependency:
# lifeguard-migrate = { workspace = true }

# In build.rs (or Cargo.toml build-dependencies):
# lifeguard-migrate processes models/*.rs and generates migrations

# Run migrations:
# The binary connects to PostgreSQL and applies pending migrations on startup
```

Entity workflow:
1. Edit `models/*.rs` (add/remove/modify fields)
2. Run `lifeguard_migrator` to generate migration SQL
3. Migration is applied on service startup

---

## 11. Integration Between Services

### 11.1 Login → Authz-Core Flow

The only cross-service call is at login:

```
Client → identity-login-service (login)
          ↓
        authz-core (principal/effective)
          ↓
        JWT issued with enriched claims
```

After login, the JWT is self-contained — all other services validate it locally via JWKS.

### 11.2 Session Service as Central Hub

The session service is the most complex — it integrates with:
- **Login service** — receives tokens from login flow
- **Authz-core** — validates roles for session operations
- **User-mgmt** — reads user profile data
- **External OIDC providers** — for social login callbacks

---

## 12. Error Handling Convention

All controllers follow this pattern:

```rust
match entity::find().filter(...).one(exec) {
    Ok(Some(entity)) => { /* success */ },
    Ok(None) => Response { /* empty/default — 404 equivalent */ },
    Err(lifeguard::LifeError::Other(ref s))
        if s.contains("Expected exactly one row, got 0") => {
            Response { /* 404 handling */ }
        },
    Err(e) => {
        eprintln!("DB Error: {}", e);
        Response { /* 500 handling */ }
    }
}
```

For HTTP error responses (non-200), use `HttpJson`:

```rust
return HttpJson::new(404, serde_json::json!({ "errors": ["Resource not found"] }));
return HttpJson::new(500, serde_json::json!({ "errors": [format!("DB error: {}", e)] }));
```

---

## 13. Audit Event Emission

All controllers emit audit events via `crate::audit::EMITTER`:

```rust
use crate::audit::EMITTER;
use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};

EMITTER.emit(AuditEvent::new(
    AuditEventType::SessionManagement,
    "user_logged_in",
    AuditSeverity::Info,
    AuditActor::User { id: user_id.to_string() },
    Some(serde_json::json!({
        "email": email,
        "ip": ip,
    })),
));
```

Audit event types: `Authentication`, `Authorization`, `SessionManagement`, `UserManagement`, `APIKey`, `System`

---

## Appendix A: Comparison — Hauliage vs Sesame-IDAM

| Aspect | Hauliage | Sesame-IDAM |
|--------|----------|-------------|
| DB Pool | `hauliage_database` crate | **Missing** — needs `sesame_idam_database` |
| ORM | Lifeguard (`LifeModel` + `LifeRecord`) | **Same** — can use identical pattern |
| Entities | Per-service model files | **Need creation** — 60+ entities across 6 services |
| Controllers | Mixed: some real, some stubs | **55 audit stubs, 78 need logic** |
| Services | Minimal (controllers do direct DB calls) | **Recommended** for complex auth flows |
| Config | `config.yaml` via CLI arg | **Same pattern** |
| Security | JWKS + static keys | **Same** |
| Tenant isolation | `X-Tenant-ID` header | **Same** |
| Codegen | `brrtrouter-gen` | **Same** |
| Migration | `lifeguard-migrate` | **Missing** — needs integration |

The key gap: sesame-idam has no `database` crate, no entity models, and no migration tooling wired up. The controllers are functional stubs but can't query anything.
