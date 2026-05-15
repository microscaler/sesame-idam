---
title: Authorization Flow
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, service-topology-design.md]
---

# Authorization Flow

## Hybrid Authorization Model (Epic 4)

> **Updated:** 2026-01-22 — Epic 4 introduces a hybrid model: JWT claims handle the common path, with lightweight online fallback for high-risk, dynamic, or high-cardinality decisions. authz-core is **no longer called on every request**.

### Three Route Categories

| Category | Description | Online Fallback | Examples |
|----------|-------------|----------------|----------|
| `jwt-only` | All authz decisions from JWT claims | No | Self-service reads (users/me GET, preferences GET) |
| `jwt-with-fallback` | JWT handles common path, fallback for edge cases | Yes, cached 5-30s | Self-service low-risk writes (preferences PUT) |
| `online-only` | All decisions require online evaluation | Yes, no cache | API key lifecycle, delegated/admin actions, authz-core endpoints |

### Route Classification (Story 4.1)

| Path Pattern | Methods | Category | Service | Rationale |
|--------------|---------|----------|---------|-----------|
| `/api/v1/identity/users/me` | GET | jwt-only | identity-user-mgmt | Self-service read, ownership from JWT |
| `/api/v1/identity/preferences` | GET | jwt-only | identity-user-mgmt | Self-service read |
| `/api/v1/identity/preferences` | PUT, PATCH | jwt-with-fallback | identity-user-mgmt | Low-risk write, business validation online |
| `/api/v1/identity/users/me` | PUT, PATCH | jwt-with-fallback | identity-user-mgmt | Low-risk update, ownership from JWT |
| `/api/v1/identity/email/upsert` | PUT | jwt-with-fallback | identity-user-mgmt | Data-integrity check needs freshness |
| `/api/v1/identity/users/query` | POST | jwt-with-fallback | identity-user-mgmt | Admin query, ownership + tenant context |
| `/api/v1/identity/users/{id}` | GET | jwt-with-fallback | identity-user-mgmt | Identity resolution, needs freshness |
| `/api/v1/am/authorize` | POST | online-only | authz-core | Fine-grained resource check always online |
| `/api/v1/am/principal/effective` | POST | online-only | authz-core | JWT claim enrichment, always online |
| `/api/v1/am/api-keys/validate` | POST | online-only | api-keys | Key validation needs freshness for revocation |
| `/api/v1/am/api-keys` (CRUD) | POST/PUT/DELETE | online-only | api-keys | Key creation/revocation always fresh |
| `/orgs` (CRUD) | POST/PUT/DELETE | online-only | org-mgmt | Org lifecycle, always online |
| `/orgs/{id}/members` (CRUD) | POST/PUT/DELETE | online-only | org-mgmt | Membership changes, always online |
| All SSO/SCIM/webhooks | POST/PUT/DELETE | online-only | org-mgmt | Sensitive org config, always online |

**Approximate split:** 40 jwt-only + 50 jwt-with-fallback + 43 online-only = ~133 endpoints total.

## JWT Common-Path Middleware (Story 4.2)

Implemented as BRRTRouter middleware. Validates JWT (typ, iss, aud, exp, signature) and evaluates local policy from claims.

```
Client Request
  -> BRRTRouter Router (path matching)
    -> JWT Common-Path Middleware  <-- NEW
      -> If jwt-only: evaluate claims, return allow/deny
      -> If jwt-with-fallback or online-only: continue to handler
    -> Handler (business logic)
```

For `jwt-only` routes, the middleware:
1. Validates JWT signature, typ, iss, aud, exp, nbf
2. Looks up RoutePolicy by path + method
3. Evaluates local policy from claims:
   - Tenant validation: `claims.tenant_id == X-Tenant-ID`
   - Role check: `claims.sx.roles` evaluated against route requirements
   - Permission check: `claims.sx.permissions` evaluated against route requirements
4. Returns `AuthDecision::Allowed` or `AuthDecision::Denied` without calling authz-core

## Per-Request Authorization (Fallback Path)

For `jwt-with-fallback` routes, the handler decides whether to call authz-core:

```
Consumer App → POST /api/v1/identity/preferences {preferences_data} →
  Handler:
    1. JWT middleware already validated token and returned claims
    2. Check: does JWT cover this decision?
       a. JWT covers → return 200 OK (common path optimization)
       b. JWT doesn't cover →
            i. Redis cache lookup (authz_fallback:{hash})
            ii. Cache HIT → return cached result
            iii. Cache MISS →
                   a. Call authz-core POST /authorize {org_id, action}
                   b. Write result to Redis (per-route TTL 5-30s)
                   c. Return {allowed: true/false}
```

## Route-Specific Authorization Decisions (Story 4.4)

### Six Route Types with Distinct Strategies

| Route Type | Strategy | Decision Logic |
|------------|----------|---------------|
| **Login, callback, OTP** | Server-side/session logic | Not JWT common-path — these routes CREATE trust, don't evaluate it |
| **Self-service reads** | JWT common path | Ownership check: `claims.sub == request.user_id` |
| **Self-service low-risk writes** | JWT + optional fallback | Ownership from JWT, business validation via online fallback |
| **Identity resolution** | Hybrid | Validate tenant from JWT, call authz-core for data-integrity |
| **API key lifecycle** | Hybrid, leaning central | Validate tenant from JWT, call authz-core for revocation freshness |
| **Delegated/admin** | Hybrid with `act`, step-up, version | Validate `act` claim, check version, call authz-core |

### Login Routes (Not Authz-Protected)

Login routes are **not protected by JWT common-path authz** because they CREATE trust:

```rust
async fn handle_login(request: LoginRequest) -> Result<LoginResponse, AuthError> {
    // 1. Authenticate user (password, MFA, OAuth)
    // 2. Call authz-core /principal/effective for JWT claim enrichment
    // 3. Sign and return JWT
    // 4. Store session in Redis + PG
    // NOTE: No authorization check needed — authentication IS the authorization.
}
```

### Self-Service Reads (jwt-only)

```rust
async fn handle_get_users_me(claims: AccessClaims) -> Result<UserProfile, AuthError> {
    // Ownership: claims.sub == request.user_id (checked in JWT middleware)
    let profile = user_repo.find_by_id(request.user_id).await?;
    Ok(profile)
}
```

### Self-Service Low-Risk Writes (jwt-with-fallback)

```rust
async fn handle_put_preferences(claims: AccessClaims, body: PreferencesUpdate) -> Result<(), AuthError> {
    // 1. Ownership from JWT
    if claims.sub != body.user_id { return Err(AuthError::Forbidden); }
    // 2. Business validation via online fallback (if needed)
    if requires_business_validation(&body) {
        let auth_result = authz_client.authorize(AuthorizeRequest {
            user_id: claims.sub, org_id: claims.sx.tenant,
            action: "preferences:update", resource: body.resource_id,
        }).await?;
        if !auth_result.allowed { return Err(AuthError::Forbidden); }
    }
    user_repo.update_preferences(body).await?;
    Ok(())
}
```

### Identity Resolution (Hybrid)

```rust
async fn handle_email_upsert(claims: AccessClaims, body: EmailUpsert) -> Result<EmailInfo, AuthError> {
    // 1. Tenant from JWT (fast)
    validate_tenant(&claims)?;
    // 2. Permission from JWT claims (fast)
    if !claims.sx.permissions.contains(&"email:write".to_string()) {
        return Err(AuthError::Forbidden);
    }
    // 3. Data-integrity via authz-core (always online, not cached)
    let auth_result = authz_client.authorize(AuthorizeRequest {
        user_id: claims.sub, org_id: claims.sx.tenant,
        action: "email:upsert", resource: body.email,
    }).await?;
    if !auth_result.allowed { return Err(AuthError::Forbidden); }
    email_repo.upsert(body).await?;
    Ok(email_repo.find_by_address(body.email).await?)
}
```

### Delegated/Admin Actions (Hybrid with act, step-up, version)

```rust
async fn handle_admin_action(claims: AccessClaims, body: AdminAction) -> Result<AdminActionResult, AuthError> {
    // 1. Extract actor (act claim or user claim)
    let actor = match &claims.act {
        Some(act) => act,
        None => ActorClaim { sub: claims.sub.clone() },
    };
    // 2. Version check for elevated risk
    if claims.sx.risk == Some("elevated".to_string()) {
        let current_ver = version_cache.get(actor.sub).await?;
        if claims.ver < current_ver { return Err(AuthError::StaleAuthToken); }
    }
    // 3. Admin permission via authz-core (always online)
    let auth_result = authz_client.authorize(AuthorizeRequest {
        user_id: actor.sub, org_id: actor.tenant,
        action: body.action, resource: body.resource_id,
    }).await?;
    if !auth_result.allowed { return Err(AuthError::Forbidden); }
    admin_repo.execute_action(body).await?;
    Ok(AdminActionResult { success: true })
}
```

## Selective Online Fallback with Caching (Story 4.3)

For `jwt-with-fallback` routes, if JWT claims don't cover the decision, call authz-core with cached result:

### Cache Key Generation

```rust
fn generate_fallback_cache_key(request: &AuthorizeRequest) -> String {
    // CRITICAL: tenant_id is included to prevent cache collision between tenants
    let key_data = format!("{}:{}:{}:{}:{}",
        request.tenant_id, request.sub, request.org_id,
        request.action, request.resource_id
    );
    format!("authz_fallback:{}", blake3::hash(key_data.as_bytes()))
}
```

### Cache TTL per Route

| Route | Cache TTL | Rationale |
|-------|-----------|-----------|
| `/api/v1/identity/preferences` PUT | 30s | Low-risk write, stale results acceptable |
| `/api/v1/identity/email/upsert` PUT | 15s | Data integrity needs more freshness |
| `/api/v1/identity/users/me` PUT | 30s | User update, ownership from JWT |
| `/api/v1/identity/users/query` POST | 15s | Admin query, tenant-scoped |

### Cache Miss Storm Mitigation (Single-Flight)

When cache expires, use single-flight pattern: only one request hits authz-core, others wait for result.

### Fallback Ratio Economics

```
baseline_authz_qps = R (all requests)
hybrid_authz_qps = (R × f) + T  (f = fallback rate, T = issuance/refresh traffic)
reduction = 1 - hybrid_authz_qps / baseline_authz_qps
```

Examples:
- R = 10,000 rps, f = 0.5%, T = 20 → 99.3% reduction (70 rps)
- R = 10,000 rps, f = 2% → 97.8% reduction (220 rps)

### Fallback Ratio Monitoring

- Alert on fallback ratio > 5% (means JWT common path is not working)
- Track per-route: `authz_fallback_total{route}`, `authz_fallback_ratio`
- Track latency: `authz_fallback_latency_ms`

## RFC 7662 Introspection Endpoint (Story 4.5 — Optional)

Standards-based token introspection for resource servers that cannot validate JWTs directly:

```
POST /auth/introspect
Content-Type: application/x-www-form-urlencoded
Authorization: ApiKeyHeader  # Server-to-server only

token=<access_token>
token_type_hint=access_token  # optional
```

Response:
```json
{
  "active": true,
  "scope": "profile:read orders:write",
  "client_id": "web-portal",
  "token_type": "Bearer",
  "exp": 1715003600,
  "iat": 1715000000,
  "sub": "user_abc123",
  "aud": ["myapp.com"],
  "iss": "https://idam.example.com",
  "jti": "tok_abc123"
}
```

- Requires API key authentication (not Bearer tokens)
- Not accessible with user Bearer tokens (server-to-server endpoint only)
- Rate limited (100 req/min per client)
- Fast path: JWT validation via JWKS
- Slow path: Database fallback for unrecognized tokens
- PII fields (email, name, phone) are NEVER included in introspection response

## Cache Strategy

### Fallback Result Cache

- **TTL:** Per-route configurable (5-30 seconds, defined in RoutePolicy)
- **Target hit ratio:** >99%
- **Sharding:** Shard by `tenant_id` (permissions are tenant-scoped)
- **Key format:** `authz_fallback:{blake3_hash}`

### Other Caches

| Cache | TTL | Why |
|-------|-----|-----|
| JWKS cache | 5 minutes | Low churn, avoids repeated discovery |
| Version cache | 15-60 seconds | Limits central lookups without slow revocation |
| Fallback result cache | 5-30 seconds per route | Cuts repeated fallback chatter |
| Denylist cache | Until token exp | Needed only for urgent revocations |
| Entitlement snapshot cache | 30-300 seconds | Avoids embedding large ACLs in tokens |

## Performance Impact

### Analytical Load Reduction

| Scenario | Baseline | Hybrid | Reduction |
|----------|----------|--------|-----------|
| 10,000 rps, 0.5% fallback | 10,000 rps | 70 rps | 99.3% |
| 10,000 rps, 2% fallback | 10,000 rps | 220 rps | 97.8% |

### Key Design Rule

**The common path must stay local.** Online fallback is only for the small set of routes where policy is too dynamic, too sensitive, or too large to encode safely in a token.

## Principal/Effective Flow

Called once at login time from identity-login-service:

```
identity-login-service → POST /api/v1/am/principal/effective {user_id, org_id} →
  authz-core:
    1. Resolve user's roles in this org
    2. Walk role inheritance chain (parent_role_id)
    3. Collect all permissions
    4. Return effective claims for JWT signing
```

**This is the ONLY time authz-core is called in the login flow.** All subsequent per-request authorization uses the hybrid model above.

## Code Anchors

- `microservices/idam/authz-core/impl/src/` — Authorization handler logic, principal/effective
- `microservices/idam/identity-login-service/impl/src/` — Login handler, JWT signing
- `openapi/authz-core/openapi.yaml` — authorize + principal/effective endpoints
- `openapi/identity-login-service/openapi.yaml` — Login response schema with JWT claims
- `openapi/identity-session-service/openapi.yaml` — Introspection endpoint (Story 4.5)

## Gaps / Drift

> **Open:** Verify cache implementation, TTL values, and hit ratio targets against source code.
> **Open:** Route classification YAML file needs to be generated from OpenAPI specs (Story 4.1).
> **Open:** Introspection endpoint (Story 4.5) is optional — only needed for legacy/third-party integrations.
