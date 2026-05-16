# Story 4.2: Implement JWT Common-Path Authorization Middleware

## Epic

[04-hybrid-authz-model](../hybrid.md)

## Parent Epic Story

Story 4.2

## Summary

Implement a gateway-level middleware that validates the JWT (typ, iss, aud, exp, signature) and evaluates local policy from claims. For `jwt-only` routes, this middleware returns allow/deny without calling authz-core. This is the primary mechanism for reducing online authz load.

## Why This Story Exists

The JWT document's core thesis: JWT claims handle the common path, with online fallback for high-risk decisions. This story implements the JWT common-path evaluation that replaces the current per-request authz-core call for `jwt-only` routes.

## Design Context

### Current State

- All requests currently go through the existing BRRTRouter middleware
- The middleware validates JWTs (or API keys) but doesn't evaluate authorization claims
- Fine-grained authorization requires calling authz-core `/authorize` endpoint
- No JWT common-path middleware exists

### Middleware Placement

```
Client Request
  -> BRRTRouter Router (path matching)
    -> JWT Common-Path Middleware  <-- NEW
      -> If jwt-only: evaluate claims, return allow/deny
      -> If jwt-with-fallback or online-only: continue to handler
    -> Handler (business logic)
```

### Middleware Implementation

```rust
pub struct JwtAuthMiddleware {
    route_policies: Arc<RoutePolicyStore>,
    // JWKS client for token validation (from Epic 1)
    jwks_client: Arc<JwksClient>,
}

impl JwtAuthMiddleware {
    pub async fn validate_and_authorize(
        &self,
        request: &HttpRequest,
    ) -> Result<AuthDecision, AuthError> {
        // 1. Extract Bearer token
        let token = extract_bearer_token(request)?;
        
        // 2. Validate JWT (typ, iss, aud, exp, nbf, signature)
        let claims: AccessClaims = self.jwks_client.validate(token)?;
        
        // 3. Look up route policy
        let path = request.path();
        let method = request.method().to_string();
        let policy = self.route_policies.get_policy(path, &method)
            .ok_or(AuthError::PolicyNotFound)?;
        
        // 4. Evaluate local policy from claims
        match &policy.category {
            RouteAuthCategory::JwtOnly => {
                self.evaluate_jwt_only(&claims, policy)
            }
            RouteAuthCategory::JwtWithFallback { .. } => {
                Ok(AuthDecision::JwtCommonPath { claims })
            }
            RouteAuthCategory::OnlineOnly => {
                Ok(AuthDecision::JwtCommonPath { claims })
            }
        }
    }
    
    fn evaluate_jwt_only(
        &self,
        claims: &AccessClaims,
        policy: &RoutePolicy,
    ) -> Result<AuthDecision, AuthError> {
        // 5. Evaluate local policy
        // - Check tenant_id matches request X-Tenant-ID
        // - Check roles/permissions in claims.sx
        // - Check user_type (customer vs platform)
        // - Check risk level if present
        
        let allowed = self.evaluate_local_policy(claims, policy);
        
        if allowed {
            Ok(AuthDecision::Allowed { claims })
        } else {
            Ok(AuthDecision::Denied { reason: "jwt_only_policy_violation" })
        }
    }
    
    fn evaluate_local_policy(&self, claims: &AccessClaims, policy: &RoutePolicy) -> bool {
        // 1. Tenant validation
        // 2. Role/permission check
        // 3. User type check
        // 4. Risk level check
        true // implementation placeholder
    }
}

// AuthDecision is the result of middleware evaluation
pub enum AuthDecision {
    Allowed { claims: AccessClaims },
    Denied { reason: String },
    JwtCommonPath { claims: AccessClaims },  // Continue to handler (not jwt-only)
}
```

### Local Policy Evaluation

For `jwt-only` routes, the middleware evaluates:

```
1. Tenant validation: claims.tenant_id == request X-Tenant-ID
2. User type: claims.user_type matches expected type for this route
3. Role check: claims.sx.roles contains required role (if any)
4. Permission check: claims.sx.permissions contains required permission (if any)
5. Risk check: claims.sx.risk == "normal" (if elevated/critical routes)
```

### Tenant Validation (Critical)

The tenant validation is the most critical check -- if this fails, the request must be rejected immediately:

```rust
fn validate_tenant(&self, claims: &AccessClaims, request: &HttpRequest) -> Result<(), AuthError> {
    let request_tenant = request
        .headers()
        .get("X-Tenant-ID")
        .and_then(|h| h.to_str().ok())
        .ok_or(AuthError::MissingTenantId)?;
    
    if claims.tenant_id != request_tenant {
        return Err(AuthError::TenantMismatch {
            expected: request_tenant.to_string(),
            actual: claims.tenant_id.clone(),
        });
    }
    
    Ok(())
}
```

## Mermaid Diagrams

### Middleware Flow

```mermaid
sequenceDiagram
    participant Client
    participant Router as BRRTRouter
    participant JwtMid as JWT Middleware
    participant JWKS as JWKS Client
    participant Handler as Route Handler

    Client->>Router: POST /api/v1/identity/users/me
    Router->>JwtMid: Process request
    JwtMid->>JwtMid: Extract Bearer token
    JwtMid->>JWKS: Validate JWT
    JWKS-->>JwtMid: AccessClaims (validated)
    JwtMid->>JwtMid: Look up RoutePolicy
    JwtMid->>JwtMid: Category: jwt-only
    JwtMid->>JwtMid: Evaluate local policy
    alt Policy allowed
        JwtMid-->>Handler: AuthDecision::Allowed { claims }
        Handler->>Handler: Process with tenant context
    else Policy denied
        JwtMid-->>Client: 403 Forbidden
    end
```

### JWT Common-Path vs Online Fallback

```mermaid
flowchart TD
    A[Request] --> B[JWT Middleware]
    B --> C{Category?}
    
    C -->|jwt-only| D[Evaluate JWT claims]
    D --> E{Tenant match?}
    E -->|No| F[401 Tenant Mismatch]
    E -->|Yes| G{Roles/permissions allow?}
    G -->|No| H[403 Forbidden]
    G -->|Yes| I[200 OK]
    
    C -->|jwt-with-fallback| J[Validate JWT]
    J --> K{Claims cover decision?}
    K -->|Yes| I
    K -->|No| L[Call authz-core /authorize]
    L --> M{Cached?}
    M -->|Yes| I
    M -->|No| N[DB evaluation]
    N --> I
    
    C -->|online-only| O[Validate JWT only]
    O --> P[Call authz-core /authorize]
    P --> I
```

### Middleware Position in BRRTRouter Pipeline

```mermaid
graph TB
    A[Client Request] --> B[BRRTRouter Router]
    B --> C[JWT Common-Path Middleware]
    C -->|jwt-only: allow| D[Handler]
    C -->|jwt-only: deny| E[403 Forbidden]
    C -->|jwt-with-fallback/online-only| D
    D --> F[Response]
    
    subgraph "JWT Middleware"
        C --> G[Validate JWT]
        G --> H[Extract claims]
        H --> I[Look up RoutePolicy]
        I --> J[Evaluate local policy]
    end
```

## Malicious Hacker Gotchas (Must Be Addressed During Implementation)

> **Source:** `docs/PRS_SECURITY_HARDENING.md` — Security threat model analysis

These are specific attack vectors identified during threat modeling. Each must be considered and mitigated during implementation. If a gotcha cannot be fully mitigated, document the residual risk.

### HACK-401: Tenant Validation Not Enforced BEFORE Handler (CRITICAL — Hole #5 from PRS)

**Risk:** Cross-tenant data exfiltration

The `validate_tenant()` function in Story 4.2 checks `claims.tenant_id != request_tenant` and returns `AuthError::TenantMismatch`. BUT this only applies to `jwt-only` routes. For `jwt-with-fallback` and `online-only` routes, the middleware returns `AuthDecision::JwtCommonPath { claims }` and passes control to the handler — the handler is responsible for tenant validation.

**Exploit path:**
1. Attacker has a valid JWT from Tenant A
2. Attacker sends `POST /api/v1/identity/users/me` (jwt-with-fallback route)
3. Middleware validates JWT, extracts claims, returns `JwtCommonPath`
4. Handler does NOT check tenant — it queries by `claims.sub` only
5. Result: attacker accesses user data from Tenant A

**The real attack is more subtle:** If the BRRTRouter framework itself extracts `tenant_id` from `X-Tenant-ID` header (not from JWT claims), the middleware is safe. But if any handler falls back to JWT `claims.tenant_id` without checking the header, the tenant can be spoofed.

**Implementation requirement:**
- Ensure the BRRTRouter framework ALWAYS uses `X-Tenant-ID` header (not JWT claims) for tenant context extraction
- Document this as an architectural requirement: "All handlers MUST use the tenant context injected by the BRRTRouter framework from `X-Tenant-ID` header — never from JWT claims directly"
- The JWT claims `tenant_id` should ONLY be used for VALIDATION (compare against header), not for authorization decisions

### HACK-402: JWT Signature Validation Is the Only Defense (CRITICAL — Hole #7 from PRS)

**Risk:** If JWKS cache is poisoned or signing key compromised, ALL routes are bypassed

The middleware's ONLY security check is the JWT signature. Once the signature validates, ALL claims (roles, permissions, risk level) are trusted. There is no secondary verification for any claims.

**Exploit path:**
1. Attacker obtains the private signing key (memory dump, insider threat, backup leak)
2. Attacker crafts JWT with any roles, permissions, risk levels
3. The JWT signature is valid
4. ALL routes are accessible — jwt-only, jwt-with-fallback, online-only
5. Result: full system compromise

**Why this is the most critical hole:** The JWT claims system is the ENTIRE security model. Without a valid signature, nothing works. With a valid signature, everything works. There is no defense-in-depth at the claims level.

**Implementation requirement:**
- Implement JWKS cache poisoning protection (HACK-421)
- Implement token binding (Story 8.2) to limit the impact of a stolen token
- Implement entitlements hash verification (HACK-101) as a secondary check
- Consider per-route canonical verification for high-consequence actions (HACK-102)

### HACK-403: Missing X-Tenant-ID Header Not Rejected for jwt-with-fallback (HIGH)

**Risk:** Tenant context missing → queries run without tenant isolation

The `validate_tenant()` function returns `AuthError::MissingTenantId` if the header is missing. But this function is ONLY called for `jwt-only` routes (inside `evaluate_jwt_only()`). For `jwt-with-fallback` and `online-only` routes, the middleware returns `JwtCommonPath { claims }` WITHOUT calling `validate_tenant()`.

**Exploit path:**
1. Attacker sends a request to `POST /api/v1/identity/preferences` (jwt-with-fallback)
2. Attacker includes a valid JWT from Tenant A
3. Attacker omits the `X-Tenant-ID` header
4. Middleware passes the request to handler (no tenant check for jwt-with-fallback)
5. If the handler doesn't also check the tenant, queries run without tenant isolation
6. Result: potential tenant bleed if database lacks tenant_id column or RLS

**Implementation requirement:**
- ALL routes (including jwt-with-fallback and online-only) must validate `X-Tenant-ID` is present
- The tenant validation should happen at the BRRTRouter framework level (before any middleware or handler), NOT in the JWT middleware
- Document this as: "Tenant context MUST be extracted from `X-Tenant-ID` header by the framework layer. JWT middleware validates consistency but does not enforce it"

### HACK-404: JWKS Cache Poisoning (CRITICAL — Hole #16 from PRS)

**Risk:** If JWKS cache is poisoned, attacker's forged tokens are accepted

The middleware uses a JWKS cache with a 5-minute TTL. If an attacker can poison this cache (e.g., by sending a malicious JWKS endpoint URL, or by exploiting a bug in the JWKS parsing logic), all requests with tokens signed by the attacker's key will be accepted.

**Exploit path:**
1. Attacker controls the JWKS endpoint (or exploits a deserialization bug in JWKS parsing)
2. Attacker injects their public key into the JWKS cache
3. Attacker signs forged JWTs with their private key
4. The middleware loads the attacker's key from cache
5. All forged tokens are accepted as valid

**Implementation requirement:**
- Validate JWKS keys have correct key type (RSA/EC) and algorithm (RS256/ES256)
- Reject keys with insecure parameters (e.g., small modulus < 2048 bits for RSA)
- Implement JWKS cache poisoning detection (compare key fingerprints)
- Log JWKS cache updates (new keys added/removed) for auditing

### HACK-405: Middleware Is a Single Point of Failure (HIGH — documented but underestimated)

**Risk:** If the middleware fails, ALL requests are blocked or bypassed

The Risk/Trade-offs section mentions this as a trade-off, but doesn't specify the failure behavior. What happens when:
- JWKS endpoint is unreachable → cache miss → validation fails → 503 or fail-open?
- RoutePolicyStore is not initialized → no policy found → 503 or fail-open?
- Memory exhaustion → middleware process dies → all requests drop?

**Implementation requirement:**
- Document EXPLICIT failure behavior for each scenario:
  - JWKS cache miss with no cached key → fail CLOSED (reject with 503)
  - JWKS endpoint unreachable → use cached keys if available, fail closed if not
  - RoutePolicyStore not initialized → fail closed (reject with 503)
  - Any unexpected error → fail closed (reject with 503)
- NEVER fail open — a middleware failure must always reject the request

### HACK-406: No Rate Limiting on JWT Validation (MEDIUM)

**Risk:** Attacker can perform JWT signature validation DoS

Each request to a jwt-only route triggers JWT signature validation (JWKS lookup + crypto operation). An attacker can send millions of requests with different JWTs to cause CPU exhaustion from cryptographic operations.

**Implementation requirement:**
- Add rate limiting at the framework level (before JWT validation)
- Implement per-IP rate limiting for requests with Authorization headers
- Reject requests that exceed the rate limit with 429 Too Many Requests

### HACK-407: Token Expired but Still Processed (MEDIUM — Hole #4 from PRS)

**Risk:** Expired tokens used before middleware rejects them

The middleware validates `exp` and `nbf` claims. But what if the token is expired and the client is on a slow network? The client sends the token, the middleware validates it, rejects it as expired — but by then, the request has already consumed server resources.

**Implementation requirement:**
- Add token expiry check before any expensive validation (JWKS lookup, signature verification)
- For expired tokens, reject immediately without processing

### HACK-408: Claims Can Be Tampered if Signature Validation Is Skipped (LOW — but documented)

**Risk:** If `extract_jti` or any helper function skips signature validation, tampered claims are accepted

The security assessment (F-002) notes: "`extract_jti` disables signature validation, allowing tampered tokens." While this may be an issue in a different story, Story 4.2 should ensure that ALL code paths validate signatures — NEVER skip signature validation for any reason.

**Implementation requirement:**
- Ensure that the `jwks_client.validate()` method ALWAYS validates the signature
- Document this as an invariant: "JWT signature validation is NEVER skipped — it is the foundational security check"

---

## OpenAPI Changes

No OpenAPI changes. The middleware is internal to the routing layer. The OpenAPI spec documents the API surface -- the authorization mechanism is an implementation detail.

## Design Doc References

- `design-doc.md` section 10.3: Hybrid Authorization Model -- JWT common-path middleware
- `design-doc.md` section 6.2: JWT Schema -- claims available for local policy evaluation
- `design-doc.md` section 10.1: Token Security -- tenant validation
- `design-doc.md` section 10.9: RLS Security Model -- middleware + SesameExecutor flow
- `topics/topic-hybrid-authz.md`: (new) Document middleware position

## Wiki Pages to Update/Create

- `topics/topic-hybrid-authz.md`: (new) Document middleware implementation
- `topics/topic-login-flow.md`: Note JWT common-path middleware
- `topics/topic-authorization-flow.md`: Update with hybrid model

## Acceptance Criteria

- [ ] JWT middleware is implemented as a BRRTRouter middleware component
- [ ] Middleware extracts Bearer token from request
- [ ] JWT is validated: typ=at+jwt, iss, aud, exp, nbf, signature (via JWKS)
- [ ] RoutePolicy is looked up by path + method
- [ ] For `jwt-only` routes: local policy is evaluated from JWT claims
- [ ] Tenant validation: claims.tenant_id matches request X-Tenant-ID
- [ ] Role check: claims.sx.roles evaluated against route requirements
- [ ] Permission check: claims.sx.permissions evaluated against route requirements
- [ ] User type check: claims.user_type validated for route
- [ ] For `jwt-with-fallback` and `online-only`: JWT is validated but policy evaluation is passed to handler
- [ ] Denied requests return 403 Forbidden with reason
- [ ] Allowed requests pass AccessClaims to the handler context
- [ ] Metrics: `jwt_validation_total{route, result}` is emitted per route
- [ ] Metrics: `jwt_validation_latency_ms` is emitted per route

## Dependencies

- Depends on Story 1.3 (JWKS validation infrastructure)
- Depends on Story 2.2 (AccessClaims struct)
- Depends on Story 4.1 (RoutePolicyStore with classified routes)
- Required by Story 4.3 (online fallback integration)

## Risk / Trade-offs

- **Tenant validation in middleware**: The tenant_id is validated against the request's X-Tenant-ID header in the middleware. If a service receives a request without X-Tenant-ID (e.g., internal service-to-service call), the middleware fails. This is correct -- every request must have a tenant context.
- **Role/permission evaluation in middleware**: The middleware evaluates roles and permissions from the JWT claims. This is correct for coarse-grained checks but may be insufficient for fine-grained resource-level authorization (e.g., "can user edit THIS specific invoice?"). Fine-grained checks are handled by the online fallback (Story 4.3).
- **Middleware as a single point of failure**: If the JWT middleware is down or slow, ALL requests are blocked. This is mitigated by:
  - JWKS cache (5-minute TTL) so validation doesn't depend on network
  - In-memory RoutePolicyStore so classification doesn't depend on external config
  - JWT validation is fast (signature check + claim parsing, <1ms)

## Tests

### Unit Tests

- [ ] **Bearer token extraction works**: Given an HTTP request with `Authorization: Bearer eyJhbG...`, assert `extract_bearer_token()` returns `"eyJhbG..."` (the raw token string)
- [ ] **Bearer token extraction rejects missing header**: Given an HTTP request with no `Authorization` header, assert `extract_bearer_token()` returns `AuthError::MissingAuthorization`
- [ ] **Bearer token extraction rejects non-Bearer scheme**: Given `Authorization: Basic dXNlcjpwYXNz`, assert `extract_bearer_token()` returns an error (only `Bearer` scheme is accepted)
- [ ] **Tenant validation accepts match**: Given `claims.tenant_id = "tenant-abc"` and `X-Tenant-ID: tenant-abc`, assert `validate_tenant()` returns `Ok(())`
- [ ] **Tenant validation rejects mismatch**: Given `claims.tenant_id = "tenant-abc"` and `X-Tenant-ID: tenant-def`, assert `validate_tenant()` returns `AuthError::TenantMismatch { expected: "tenant-def", actual: "tenant-abc" }`
- [ ] **Tenant validation rejects missing header**: Given no `X-Tenant-ID` header, assert `validate_tenant()` returns `AuthError::MissingTenantId`
- [ ] **Local policy allows with matching role**: Given `claims.sx.roles = ["admin"]` and a route requiring `["admin"]`, assert `evaluate_local_policy()` returns `true`
- [ ] **Local policy denies with missing role**: Given `claims.sx.roles = ["customer"]` and a route requiring `["admin"]`, assert `evaluate_local_policy()` returns `false`
- [ ] **Local policy allows with matching permission**: Given `claims.sx.permissions = ["prefs:write"]` and a route requiring `["prefs:write"]`, assert `evaluate_local_policy()` returns `true`
- [ ] **Local policy denies with missing permission**: Given `claims.sx.permissions = ["prefs:read"]` and a route requiring `["prefs:write"]`, assert `evaluate_local_policy()` returns `false`
- [ ] **Local policy allows with normal risk**: Given `claims.sx.risk = Some("normal")` and a route allowing all risk levels, assert `evaluate_local_policy()` returns `true`
- [ ] **Local policy allows without risk claim**: Given `claims.sx.risk = None`, assert `evaluate_local_policy()` returns `true` (absence of risk claim does not cause denial)
- [ ] **jwt-only returns AuthDecision::Allowed**: Given a jwt-only route with all policy checks passing, assert `validate_and_authorize()` returns `AuthDecision::Allowed { claims }`
- [ ] **jwt-only returns AuthDecision::Denied**: Given a jwt-only route with a role check failure, assert `validate_and_authorize()` returns `AuthDecision::Denied { reason: "jwt_only_policy_violation" }`
- [ ] **jwt-with-fallback returns AuthDecision::JwtCommonPath**: Given a jwt-with-fallback route, assert `validate_and_authorize()` returns `AuthDecision::JwtCommonPath { claims }` (continues to handler)
- [ ] **online-only returns AuthDecision::JwtCommonPath**: Given an online-only route, assert `validate_and_authorize()` returns `AuthDecision::JwtCommonPath { claims }` (continues to handler)
- [ ] **PolicyNotFound error for unclassified route**: Given a path+method not in any RoutePolicy, assert `validate_and_authorize()` returns `AuthError::PolicyNotFound`

### Integration Tests (BDD-style with `rstest_bdd`)

- [ ] **Scenario: Valid jwt-only request succeeds**: `given` a user with `org_admin` role → `when` a request to `/api/users/me GET` (jwt-only) is made with a valid JWT and matching X-Tenant-ID → `then` the middleware returns `AuthDecision::Allowed` and the handler processes the request
- [ ] **Scenario: jwt-only denied due to role**: `given` a user with `customer` role → `when` a request to a route requiring `org_admin` is made → `then` the middleware returns `AuthDecision::Denied { reason: "jwt_only_policy_violation" }` and the client receives 403
- [ ] **Scenario: Tenant mismatch rejected**: `given` a valid JWT for tenant A → `when` a request with `X-Tenant-ID: B` is made → `then` the middleware rejects with `TenantMismatch` and the client receives 401
- [ ] **Scenario: jwt-with-fallback continues to handler**: `given` a valid JWT → `when` a request to a `jwt-with-fallback` route is made → `then` the middleware returns `AuthDecision::JwtCommonPath` and the handler proceeds (no 403 from middleware)
- [ ] **Scenario: online-only continues to handler**: `given` a valid JWT → `when` a request to an `online-only` route is made → `then` the middleware returns `AuthDecision::JwtCommonPath` and the handler proceeds to call authz-core
- [ ] **Scenario: Metrics emitted per route**: `given` 10 requests to `/api/users/me GET` → `then` `jwt_validation_total{route: "/api/users/me", result: "allowed"}` is emitted 10 times, and `jwt_validation_latency_ms` histogram records 10 samples
- [ ] **Scenario: Invalid JWT rejected at step 2**: `given` a request with an expired JWT → `when` the middleware processes it → `then` `jwt_validation_total{result: "denied", reason: "token_expired"}` is emitted and the handler is never called

### Security Regression Tests

- [ ] **Tenant bleed prevented**: `given` user alice from tenant A → `when` alice sends a request with `X-Tenant-ID: B` → `then` the middleware rejects with `TenantMismatch` BEFORE the handler is called (tenant validation happens at the middleware layer, not in the handler)
- [ ] **Token tampering detected**: `given` a JWT where the client modifies `sx.roles` from `["customer"]` to `["admin"]` → `then` the signature verification fails (the token is unsigned by the issuer, so it's rejected before policy evaluation)
- [ ] **Missing X-Tenant-ID blocks request**: `given` a request with a valid JWT but no `X-Tenant-ID` header → `then` the middleware rejects with 401 `MissingTenantId` (no request reaches the handler)
- [ ] **jwt-only routes cannot be bypassed**: Assert that `jwt-only` routes NEVER make a call to authz-core — the entire authorization decision is made from JWT claims alone

### Edge Cases

- [ ] **Malformed JWT header (not base64url)**: Send a request with `Authorization: Bearer not-a-jwt` — assert the middleware returns 401 with a clear error message (not a panic or 500)
- [ ] **JWT with no claims body**: Send a JWT where the payload decodes to an empty JSON object — assert deserialization into `AccessClaims` fails and the middleware returns 401
- [ ] **Concurrent requests with same token**: 100 concurrent requests with the same valid JWT — assert all 100 succeed without cache corruption or race conditions (JWKS validation is thread-safe via `Arc`/`RwLock`)
- [ ] **Very large JWT (>750 bytes)**: Send a JWT that exceeds the token size budget — assert the middleware still validates it correctly (the middleware should NOT reject based on size alone; size enforcement is a build-time test per Story 2.5)
- [ ] **JWT with unusual claim values**: A JWT with `sx.roles = []` (empty array), `sx.permissions = []` (empty array), `sx.risk = Some("elevated")` — assert policy evaluation handles empty role/permission arrays gracefully (no panic, returns appropriate allow/deny based on route requirements)

### Cleanup

- No state cleanup required — the middleware is stateless (it reads from in-memory `RoutePolicyStore` and `JwksClient`)
- Integration tests must not leave partially-validated JWTs in caches — use fresh JWTs per test scenario
- If metrics are global singletons, clear the metrics registry between test scenarios using `prometheus::Registry::new()` or similar
