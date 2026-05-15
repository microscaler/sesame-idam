# Story 4.5: Implement RFC 7662 Introspection Endpoint

## Epic

[04-hybrid-authz-model](../hybrid.md)

## Parent Epic Story

Story 4.5

## Summary

Implement an RFC 7662-compatible introspection endpoint that provides a standards-based fallback for token validation. This is a standards-compliant way for resource servers to validate tokens online when JWT claims are insufficient or when immediate revocation awareness is needed.

## Why This Story Exists

The JWT document mentions RFC 7662 introspection as an optional future enhancement: "Not currently visible in public API. Can be added as a future enhancement." This story implements it as a standards-based fallback path.

## Design Context

### Current State

- No introspection endpoint exists
- No RFC 7662 compliance
- Online fallback is ad-hoc (authz-core `/authorize` endpoint)
- No standards-based token introspection

### RFC 7662 Introspection

RFC 7662 defines a standard introspection endpoint. The response format is:

```
POST /auth/introspect
Content-Type: application/x-www-form-urlencoded

token=<access_token>
token_type_hint=access_token    # optional
```

Response:

```json
{
  "active": true,
  "scope": "profile:read orders:write",
  "client_id": "web-portal",
  "username": "alice@example.com",
  "token_type": "Bearer",
  "exp": 1715003600,
  "iat": 1715000000,
  "sub": "user_abc123",
  "aud": ["myapp.com"],
  "iss": "https://idam.example.com",
  "jti": "tok_abc123"
}
```

### Introspection vs Direct JWT Validation

| Aspect | JWT Validation (Direct) | Introspection (RFC 7662) |
|--------|------------------------|-------------------------|
| Latency | Fast (signature check only) | Slow (database lookup) |
| Freshness | Bounded by token TTL | Real-time (checks revocation) |
| Scalability | High (stateless) | Low (per-token database call) |
| Use case | Common path (95%+ of requests) | Edge cases, high-risk, admin |

### Introspection Use Cases

1. **Resource servers without JWT validation**: Legacy services or third-party integrations that can't validate JWTs can call introspection instead
2. **Immediate revocation check**: When a resource server needs to know if a token is revoked RIGHT NOW (not just until it expires)
3. **Token exchange result validation**: After a token exchange, validate the new token
4. **Debugging**: When JWT validation fails, introspection can provide detailed rejection reasons

## Implementation Notes

### Introspection Endpoint

```yaml
# openapi/idam/identity-session-service/openapi.yaml
paths:
  /auth/introspect:
    post:
      summary: Token Introspection (RFC 7662)
      operationId: introspectToken
      description: |
        Introspect a token to determine its validity and claims.
        This is a standards-compliant fallback for resource servers
        that cannot validate JWTs directly.
      security:
        - ApiKeyHeader: []  # Introspection requires API key (client credentials)
      requestBody:
        required: true
        content:
          application/x-www-form-urlencoded:
            schema:
              type: object
              required: [token]
              properties:
                token:
                  type: string
                  description: The access token to introspect
                token_type_hint:
                  type: string
                  enum: [access_token, refresh_token]
                  description: Hint about the token type
      responses:
        '200':
          description: Token introspection result
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/IntrospectionResponse'
        '401':
          description: Invalid introspection credentials
        '400':
          description: Invalid request
```

### Introspection Implementation

```rust
async fn handle_introspect(
    token: String,
    token_type_hint: Option<String>,
) -> Result<IntrospectionResponse, AuthError> {
    // 1. Try JWT validation first (fast path)
    let claims = match jwks_client.validate(&token).await {
        Ok(claims) => claims,
        Err(JwtError::Revoked) => {
            // Token was revoked -- still return active=false
            return Ok(IntrospectionResponse { active: false });
        }
        Err(_) => {
            // JWT validation failed -- fall back to database lookup
            // This handles tokens signed with a different algorithm
            // or tokens from a different issuer
        }
    };
    
    // 2. Check revocation status (always fresh)
    let is_revoked = revocation_cache.is_revoked(&claims.jti).await?;
    if is_revoked {
        return Ok(IntrospectionResponse { active: false });
    }
    
    // 3. Return introspection response
    Ok(IntrospectionResponse {
        active: true,
        scope: Some(claims.scope.clone()),
        client_id: Some(claims.client_id.clone()),
        username: None,  // Email is not in the token (PII removed)
        token_type: Some("Bearer".to_string()),
        exp: Some(claims.exp),
        iat: Some(claims.iat),
        sub: Some(claims.sub.clone()),
        aud: Some(claims.aud.clone()),
        iss: Some(claims.iss.clone()),
        jti: Some(claims.jti.clone()),
    })
}
```

### Security

- Introspection requires API key authentication (client credentials)
- Not accessible with Bearer tokens (introspection is a server-to-server endpoint)
- Rate limited to prevent abuse
- All introspection requests are logged (who introspected which token)

## Mermaid Diagrams

### Introspection Flow

```mermaid
sequenceDiagram
    participant ResourceServer
    participant Introspect as /auth/introspect
    participant JWKS as JWKS Client
    participant Cache as Revocation Cache
    participant DB as Database (fallback)

    ResourceServer->>Introspect: POST /auth/introspect<br/>token=<access_token><br/>API key auth
    Introspect->>JWKS: Validate JWT
    alt JWT valid
        JWKS-->>Introspect: AccessClaims
        Introspect->>Cache: Is jti revoked?
        Cache-->>Introspect: No
        Introspect-->>ResourceServer: {active: true, sub, aud, ...}
    else JWT invalid (revoked)
        JWKS-->>Introspect: Revoked
        Introspect-->>ResourceServer: {active: false}
    else JWT invalid (unknown)
        JWKS-->>Introspect: Not recognized
        Introspect->>DB: Token lookup (fallback)
        alt Token found
            DB-->>Introspect: Token data
            Introspect-->>ResourceServer: {active: true/false, ...}
        else Token not found
            DB-->>Introspect: Not found
            Introspect-->>ResourceServer: {active: false}
        end
    end
```

### JWT Validation vs Introspection

```mermaid
flowchart TD
    A[Resource server validates token] --> B{Can validate JWT?}
    B -->|Yes| C[Direct JWT validation]
    C --> D{Valid?}
    D -->|Yes| E[Allow]
    D -->|No| F[Check revocation cache]
    F -->|Revoked| G[Deny]
    F -->|Not revoked| E
    
    B -->|No| H[Call introspection endpoint]
    H --> I{Token active?}
    I -->|Yes| E
    I -->|No| G
```

## OpenAPI Changes

- Add `/auth/introspect` endpoint to identity-session-service spec
- Add `IntrospectionResponse` schema
- Add `token_type_hint` parameter to request

```yaml
components:
  schemas:
    IntrospectionResponse:
      type: object
      required: [active]
      properties:
        active:
          type: boolean
          description: Whether the token is active
        scope:
          type: string
          description: Scope of the token
        client_id:
          type: string
          description: Client ID that issued the token
        username:
          type: string
          description: Username (may be null)
        token_type:
          type: string
          example: Bearer
        exp:
          type: integer
          format: int64
          description: Expiration time
        iat:
          type: integer
          format: int64
          description: Issued at time
        sub:
          type: string
          description: Subject (user ID)
        aud:
          type: array
          items:
            type: string
          description: Audience
        iss:
          type: string
          description: Issuer
        jti:
          type: string
          description: JWT ID
```

## Design Doc References

- `design-doc.md` section 10.3: Hybrid Authorization Model -- RFC 7662 introspection (optional)
- `design-doc.md` section 10.1: Token Security -- introspection as a fallback
- `topics/topic-hybrid-authz.md`: Document introspection as an optional enhancement
- `topics/topic-token-lifecycle.md`: Document introspection in token lifecycle

## Wiki Pages to Update/Create

- `topics/topic-hybrid-authz.md`: (new) Document introspection endpoint
- `topics/topic-token-lifecycle.md`: Document introspection in token lifecycle

## Acceptance Criteria

- [ ] `/auth/introspect` endpoint is implemented per RFC 7662
- [ ] Response includes `active` boolean (required by RFC 7662)
- [ ] Response includes `sub`, `aud`, `iss`, `exp`, `iat`, `jti` (when active)
- [ ] Response includes `scope`, `client_id`, `token_type` (when available)
- [ ] Introspection requires API key authentication (not Bearer tokens)
- [ ] Introspection returns `active: false` for revoked tokens
- [ ] Introspection returns `active: false` for expired tokens
- [ ] Introspection returns `active: false` for invalid signatures
- [ ] Introspection is rate limited (e.g., 100 requests per minute per client)
- [ ] Metrics: `introspect_total{result: "active", "inactive"}` is emitted
- [ ] All introspection requests are logged (who introspected which token)

## Dependencies

- Depends on Story 1.3 (JWKS validation infrastructure)
- Optional enhancement -- can be implemented after the core hybrid model (Stories 4.1-4.4)

## Risk / Trade-offs

- **Introspection defeats the purpose of JWT common path**: If every resource server calls introspection instead of validating JWTs, the load reduction benefit is lost. Introspection should only be used for edge cases where JWT validation is not possible or immediate revocation is needed.
- **API key requirement**: Introspection requires API key authentication. This means the resource server must have an API key registered with Sesame. This adds onboarding complexity but is necessary to prevent unauthorized token introspection.
- **Rate limiting**: Without rate limiting, introspection can be abused (e.g., a malicious resource server introspecting millions of tokens). The rate limit (100 req/min per client) is a starting point that can be adjusted based on actual usage patterns.

## Tests

### Unit Tests

- [ ] **Introspection response includes active=true for valid JWT**: Given a valid, non-expired, non-revoked JWT, assert `handle_introspect()` returns `IntrospectionResponse { active: true, sub: Some(...), aud: Some(...), iss: Some(...), exp: Some(...), iat: Some(...), jti: Some(...), scope: Some(...), client_id: Some(...), token_type: Some("Bearer") }`
- [ ] **Introspection response includes active=false for expired JWT**: Given a JWT where `exp < current_time`, assert `handle_introspect()` returns `IntrospectionResponse { active: false }`
- [ ] **Introspection response includes active=false for revoked JWT**: Given a JWT whose `jti` is in the revocation cache, assert `handle_introspect()` returns `IntrospectionResponse { active: false }`
- [ ] **Introspection response includes active=false for invalid signature**: Given a JWT with a tampered signature (e.g., `claims.sub` modified after signing), assert `handle_introspect()` returns `IntrospectionResponse { active: false }`
- [ ] **Introspection response omits username for PII protection**: Given a valid JWT for a user with email `alice@example.com`, assert the `username` field in the introspection response is `None` (PII is not included in token introspection)
- [ ] **Introspection requires API key authentication**: Given a request without an API key, assert the endpoint returns 401 Unauthorized before any JWT validation occurs
- [ ] **Introspection rejects requests without token parameter**: Given a request with no `token` field in the body, assert the endpoint returns 400 Bad Request with a clear error message
- [ ] **Introspection rejects unknown token_type_hint values**: Given `token_type_hint: "refresh_token"` (or any invalid value), assert the endpoint either returns 400 or accepts it gracefully (RFC 7662 says unknown hints should be ignored)
- [ ] **Rate limiter rejects excess introspections**: Given 101 introspection requests from the same client within one minute, assert the 101st request returns 429 Too Many Requests
- [ ] **Fast path uses JWT validation**: Given a valid JWT, assert the introspection handler validates via JWKS (fast path) and does NOT fall back to database lookup
- [ ] **Slow path falls back to database for unrecognized tokens**: Given a JWT signed by an unknown issuer (not in JWKS), assert the handler falls back to database token lookup rather than immediately denying

### Integration Tests (BDD-style with `rstest_bdd`)

- [ ] **Scenario: Introspect active valid token**: `given` a valid access token issued to user alice with scope `profile:read` → `when` a resource server calls `POST /auth/introspect` with Alice's token and a valid API key → `then` the response is `{ active: true, scope: "profile:read", sub: "alice", aud: [...], ... }`
- [ ] **Scenario: Introspect expired token**: `given` an access token with `exp` in the past → `when` a resource server calls `POST /auth/introspect` with the expired token → `then` the response is `{ active: false }`
- [ ] **Scenario: Introspect revoked token**: `given` an access token that has been explicitly revoked via `jti` denylisting → `when` a resource server calls `POST /auth/introspect` → `then` the response is `{ active: false }` (immediate revocation awareness)
- [ ] **Scenario: Introspect token signed by unknown issuer**: `given` a JWT signed with a JWKS key that is not registered → `when` a resource server calls introspection → `then` the handler falls back to database lookup and returns `{ active: false }` if the token is not found in the database
- [ ] **Scenario: Introspection with valid token_type_hint**: `given` a valid access token → `when` introspection is called with `token_type_hint=access_token` → `then` the token is validated and `{ active: true, ... }` is returned
- [ ] **Scenario: Introspection without token_type_hint**: `given` a valid access token → `when` introspection is called without `token_type_hint` → `then` the handler validates the token anyway and returns the full response (hint is optional)
- [ ] **Scenario: Introspection rate limiting kicks in**: `given` a resource server client → `when` the client sends 101 introspection requests within 60 seconds → `then` the first 100 are processed normally and the 101st returns 429 Too Many Requests with a retry-after header
- [ ] **Scenario: Introspection is server-to-server only**: `given` a client with only a Bearer token (no API key) → `when` the client calls `POST /auth/introspect` → `then` the endpoint returns 401 Unauthorized (introspection is not accessible with user Bearer tokens)
- [ ] **Scenario: Introspection logs all requests**: `given` a resource server calls introspection with a valid token → `then` the system logs include the resource server's client ID, the introspected token's `jti`, the result (active/inactive), and a timestamp

### Security Regression Tests

- [ ] **Introspection does not leak PII**: Assert that the introspection response NEVER includes `email`, `phone_number`, `first_name`, `last_name`, or any other PII field from the JWT claims — only `sub`, `aud`, `iss`, `exp`, `iat`, `jti`, `scope`, `client_id`, and `token_type` are returned
- [ ] **Introspection requires API key, not Bearer token**: Assert that a request with only a Bearer token (no API key) is rejected at the authentication layer before any JWT validation or token lookup occurs
- [ ] **Introspection cannot enumerate valid tokens**: Assert that an attacker cannot use introspection to enumerate which tokens are valid — the response for active vs inactive tokens should have the same timing characteristics (no timing side-channel)
- [ ] **Introspection cannot bypass revocation**: Assert that a token whose `jti` has been denylisted returns `{ active: false }` even if the JWT signature is valid and the token has not expired (revocation takes precedence over validity)
- [ ] **Introspection rate limit cannot be bypassed**: Assert that rate limiting is applied per client API key, not per introspected token — an attacker cannot bypass the limit by introspecting different tokens from the same client
- [ ] **Introspection response does not include internal error details**: Assert that when introspection encounters an error (e.g., database connection failure, JWKS cache miss), the response is `{ active: false }` or a generic 500 error without leaking internal state, stack traces, or query details

### Edge Cases

- [ ] **Introspect with empty token string**: Given `token=""` in the request body, assert the handler returns 400 Bad Request with a clear message ("token parameter is required" or "token cannot be empty")
- [ ] **Introspect with extremely long token (>64KB)**: Given a JWT-like token string exceeding 64,000 characters, assert the handler rejects with 400 Bad Request without consuming excessive memory or CPU
- [ ] **Introspect with malformed JOSE header**: Given a token where the first base64url segment decodes to invalid JSON, assert the handler returns `{ active: false }` (not a 500 panic or crash)
- [ ] **Concurrent introspection of same token**: 100 concurrent requests to introspect the same valid token — assert all 100 return `{ active: true, ... }` without race conditions or inconsistent results
- [ ] **Introspect with token from expired JWKS key**: Given a JWT signed with a JWKS key that has since been rotated out of the JWKS cache, assert the handler either (a) uses a cached JWKS entry if the cache TTL hasn't expired, or (b) falls back to database lookup if the token is recognized
- [ ] **Introspect with zero-value timestamps**: Given a JWT with `exp: 0` (epoch) or `iat: 0`, assert the handler correctly interprets this as an already-expired token and returns `{ active: false }`
- [ ] **Introspect with aud audience mismatch**: Given a JWT issued for `aud: "other-app"` introspected by a resource server expecting `aud: "sesame-api"`, assert the handler still returns `{ active: true }` (the introspection endpoint does not enforce audience matching — it reports the token's state regardless of which service is asking)

### Cleanup

- Redis revocation cache must be cleaned between tests — use `FLUSHDB` or a unique prefix per test run to prevent stale revocation entries from affecting subsequent tests
- JWKS cache used in tests must be reset between scenarios — use a fresh `JwksClient` or clear the cache between test runs
- Rate limiter state must be cleared between tests — if using an in-memory rate limiter, use a fresh instance per test scenario
- Mock database state (tokens stored for fallback lookup) must be cleared between tests — use a test transaction rollback or drop/recreate test tables
- Metrics registry must be reset between test scenarios using `prometheus::Registry::new()` to prevent cross-test metric contamination
- Log output from tests should be isolated per test run — use a test-specific logger or capture logs in-memory rather than writing to a shared file
- API keys used for introspection authentication must be unique per test to prevent key collisions between concurrent test scenarios
