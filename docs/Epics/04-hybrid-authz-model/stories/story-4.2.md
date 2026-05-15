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
