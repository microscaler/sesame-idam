# Story 3.3: Configure Access Token TTL

## Epic

[03-token-lifecycle](../tokens.md)

## Parent Epic Story

Story 3.3

## Summary

Implement configurable access token TTL with role-based tiers: 5 minutes for normal users, 1-5 minutes for admin/high-privilege tokens. TTL is configurable via environment variable and can be adjusted per role tier. This shortens the staleness window for authorization decisions and limits the impact of token theft.

## Why This Story Exists

The JWT document recommends 5-15 minute access token TTL, biasing toward the lower end for authorization-heavy JWTs. Shorter TTLs mean:
- Less stale permissions (authz decisions are more fresh)
- Smaller window for token replay attacks
- More frequent token rotation (better revocation granularity)
- Higher refresh token usage (better family-based detection)

The current design doc states 15 minutes default -- this story updates it to the recommended 5 minutes.

## Design Context

### Current State

- `design-doc.md` section 10.1: Token TTL default is 15 minutes
- No per-role TTL differentiation
- TTL is fixed at compile time or through a single environment variable

### Role-Based TTL Tiers (F-010 Fix)

|| Tier | TTL (minutes) | Use Case | Config Var |
|------|---------------|----------|------------|
| `normal` | 5 | Customer users, standard access | `JWT_ACCESS_TTL_NORMAL` |
| `elevated` | 5 | Users with sensitive permissions | `JWT_ACCESS_TTL_ELEVATED` |
| `admin` | 5 | Platform admins, org admins | `JWT_ACCESS_TTL_ADMIN` |
| `platform` | 5 | Platform users (support, editors) | `JWT_ACCESS_TTL_PLATFORM` |

**F-010 Fix: Admin TTL aligned to 5 minutes.** The original story proposed 1-3 minute admin tokens. This is operationally problematic:

- **Diminishing security return**: An admin with a 3-minute token can still perform any admin action without MFA for 3 minutes. The real security boundary for high-consequence admin actions is step-up MFA (Epic 6), not token frequency.
- **Redis load impact**: 3-minute tokens = 20 refreshes/hour per admin vs 12 refreshes/hour for 5-minute tokens. At 10k admins, this is ~80k additional Redis ops/hr vs ~120k total at 5 min. The 2.5x increase in refresh operations significantly increases Redis load.
- **Operational friction**: Admin tooling that performs batch operations (e.g., bulk org member management) cannot complete within 1-3 minute windows without constant token refresh, creating UX friction that encourages insecure workarounds.
- **Conclusion**: All token types use 5-minute TTL. Step-up MFA (Epic 6) provides the real security boundary for high-consequence admin actions. This simplifies the configuration matrix while maintaining security.

### TTL Configuration

```yaml
# config.yaml
jwt:
  access_token:
    normal_ttl_secs: 300    # 5 minutes
    elevated_ttl_secs: 180  # 3 minutes
    admin_ttl_secs: 180     # 3 minutes
    platform_ttl_secs: 120  # 2 minutes
```

```bash
# Environment variables (override config.yaml)
JWT_ACCESS_TTL_NORMAL=300
JWT_ACCESS_TTL_ELEVATED=180
JWT_ACCESS_TTL_ADMIN=180
JWT_ACCESS_TTL_PLATFORM=120
```

### Token Issuance with TTL

```rust
impl AccessClaims {
    pub fn ttl_for_role(role: &str) -> Duration {
        match role {
            "platform_admin" | "org_admin" => Duration::from_secs(180),  // 3 min
            "elevated" => Duration::from_secs(300),                       // 5 min
            _ => Duration::from_secs(300),                                // 5 min default
        }
    }
}
```

### Refresh Token TTL

| Tier | Refresh Token TTL | Notes |
|------|------------------|-------|
| All tiers | 7-30 days | Configurable via `JWT_REFRESH_TTL_DAYS` |
| Admin tier | 7 days | Shorter refresh window for high-privilege |
| Normal tier | 30 days | Longer refresh window for convenience |

Refresh tokens have longer TTLs because they are:
- Stored hashed in Redis (one-time-use detection)
- Rotated on every use (replay protection)
- Bound to a token family (tear detection)

## Mermaid Diagrams

### Token Lifetime

```mermaid
gantt
    title Access Token Lifetime
    dateFormat X
    axisFormat %M:%S
    section Normal User (5 min)
    Token valid             :0, 300
    Token expired           :300, 0
    
    section Admin User (3 min)
    Token valid             :0, 180
    Token expired           :180, 0
```

### TTL Decision Flow

```mermaid
flowchart TD
    A[Login successful] --> B{Determine user role}
    B -->|customer| C[TTL: 5 minutes]
    B -->|platform| D[TTL: 2 minutes]
    B -->|org_admin| E[TTL: 3 minutes]
    B -->|platform_admin| F[TTL: 3 minutes]
    B -->|elevated| G[TTL: 3 minutes]
    
    C --> H[Issue access token with exp = now + 300]
    D --> H
    E --> H
    F --> H
    G --> H
    
    H --> I[Return access_token + refresh_token]
    I --> J[Client must refresh before expiry]
```

### Refresh Storm Mitigation

```mermaid
flowchart TD
    A[Token expiring] --> B{Is user in multiple sessions?}
    B -->|No| C[Refresh once before expiry]
    B -->|Yes| D[Refresh each session before its expiry]
    
    C --> E[Spread refresh requests]
    D --> E
    
    E --> F{Is this a refresh storm?}
    F -->|No| G[Process normally]
    F -->|Yes| H[Rate limit refresh requests]
    H --> I[Client receives 429 Too Many Requests]
    I --> J[Client backs off and retries]
```

## OpenAPI Changes

- `LoginResponse` schema: Document the token expiry time (exp claim) in description
- No changes to request/response shapes needed -- TTL is an internal implementation detail

```yaml
components:
  schemas:
    LoginResponse:
      properties:
        access_token:
          type: string
          description: JWT access token (ES256-signed). Expires in 5 minutes for normal users, 1-3 minutes for elevated/admin roles.
        refresh_token:
          type: string
          description: Rotating refresh token (7-30 day TTL).
```

## Design Doc References

- `design-doc.md` section 10.1: Token Security -- TTL updated from 15 minutes to 5 minutes normal / 1-3 minutes admin
- `design-doc.md` section 10.4: Token Versioning & Revocation -- Layer 1: short access-token TTLs to cap staleness
- `service-topology-design.md`: identity-session-service handles refresh (EXTREME freq, LOW cost)

## Wiki Pages to Update/Create

- `topics/topic-token-lifecycle.md`: (new) Document TTL tiers
- `topics/topic-login-flow.md`: Update with role-based TTL

## Acceptance Criteria

- [ ] Normal user access tokens expire in 5 minutes (300 seconds)
- [ ] Admin/high-privilege access tokens expire in 1-3 minutes
- [ ] Platform user access tokens expire in 2 minutes
- [ ] TTL is configurable via environment variables (`JWT_ACCESS_TTL_*`)
- [ ] TTL defaults are enforced even when environment variables are not set
- [ ] The `exp` claim in the JWT reflects the correct TTL
- [ ] Expired tokens are rejected with 401 "token expired"
- [ ] Refresh token TTL is longer (7-30 days) than access token TTL
- [ ] Metrics: `token_ttl_seconds` histogram tracks issued token TTLs

## Dependencies

- Depends on Story 2.2 (AccessClaims struct with `exp` field)
- Intersects with Story 3.1 (refresh rotation -- shorter tokens mean more frequent refreshes)

## Risk / Trade-offs

- **Frequent refreshes**: 5-minute tokens mean clients must refresh every 5 minutes. This increases refresh token usage and Redis load. The impact is mitigated by:
  - Refresh is cached in Redis (30s TTL)
  - Refresh tokens are stored hashed (fast lookup)
  - Clients should refresh proactively (e.g., at 4:30 minutes, not at 5:00)
- **Admin token short TTL**: All token types use 5-minute TTL (F-010 fix). Step-up MFA (Epic 6) provides the real security boundary for high-consequence admin actions. This simplifies the configuration matrix while maintaining security.
- **Client-side TTL tracking**: Clients must track token expiry and refresh proactively. If a client sends a request at exactly 5 minutes, the token is expired and the request fails. This is a client-side responsibility -- the backend returns 401 and the client must refresh first.

## Tests

### Unit Tests

- [ ] **Normal user TTL is 300 seconds**: Assert `ttl_for_role("customer")` returns `Duration::from_secs(300)` (5 minutes)
- [ ] **Elevated user TTL is 300 seconds**: Assert `ttl_for_role("elevated")` returns `Duration::from_secs(300)` (F-010: aligned to 5 minutes, same as normal)
- [ ] **Admin user TTL is 300 seconds**: Assert `ttl_for_role("org_admin")` and `ttl_for_role("platform_admin")` return `Duration::from_secs(300)` (F-010: aligned to 5 minutes)
- [ ] **Platform user TTL is 300 seconds**: Assert `ttl_for_role("platform")` returns `Duration::from_secs(300)` (F-010: aligned to 5 minutes)
- [ ] **Unknown role defaults to 300 seconds**: Assert `ttl_for_role("unknown_role")` returns `Duration::from_secs(300)` (the default arm)
- [ ] **All roles produce the same TTL**: Assert `ttl_for_role("customer") == ttl_for_role("org_admin") == ttl_for_role("platform") == Duration::from_secs(300)` — confirming F-010 alignment
- [ ] **`exp` claim is correct**: Given a login at `iat = 1000` with 300s TTL, assert the issued JWT has `exp = 1300`
- [ ] **Refresh token TTL is configurable**: Assert that `JWT_REFRESH_TTL_DAYS` env var, when set to `14`, produces a refresh token with `exp - iat = 14 * 86400` seconds

### Integration Tests (BDD-style with `rstest_bdd`)

- [ ] **Scenario: Normal user gets 5-minute token**: `given` a customer user logs in → `when` the access token is decoded → `then` `exp - iat = 300` seconds
- [ ] **Scenario: Admin user gets 5-minute token**: `given` an org_admin logs in → `when` the access token is decoded → `then` `exp - iat = 300` seconds (same as normal, F-010 fix)
- [ ] **Scenario: Expired token is rejected**: `given` a token with `exp` in the past → `when` a service validates it → `then` the validation returns 401 with `token_expired`
- [ ] **Scenario: Token just before expiry is accepted**: `given` a token with `exp` 1 second in the future → `when` a service validates it → `then` the token is accepted
- [ ] **Scenario: Token 61 seconds past expiry is rejected**: `given` a token with `exp` 61 seconds ago → `when` a service validates it → `then` the token is rejected (past 60-second clock skew tolerance, per Story 1.3)
- [ ] **Scenario: Environment variable overrides default**: `given` `JWT_ACCESS_TTL_NORMAL=600` is set → `when` a normal user logs in → `then` the access token has `exp - iat = 600` seconds
- [ ] **Scenario: Short TTL increases refresh rate**: `given` a client with a 5-minute token → `when` the client makes requests over 30 minutes → `then` the `/auth/refresh` endpoint is called at least 6 times (one per token expiry)
- [ ] **Scenario: Metrics track issued TTLs**: `given` tokens are issued for different user types → `then` `token_ttl_seconds{role: "customer"}`, `token_ttl_seconds{role: "org_admin"}`, etc. are emitted with the correct values

### Security Regression Tests

- [ ] **Admin token cannot get extended TTL via role spoofing**: If a client claims to be an admin, assert the TTL is determined by the user's ACTUAL role in the system (from the authz service), not by any client-supplied role field
- [ ] **TTL cannot be manipulated at token issuance**: Assert that the `exp` claim is set by the server-side TTL function, not by any value from the request body
- [ ] **Refresh token TTL always exceeds access token TTL**: Assert that for every role tier, `refresh_token_ttl > access_token_ttl` — a refresh token should NEVER expire before its associated access token

### Edge Cases

- [ ] **Zero TTL**: If `JWT_ACCESS_TTL_NORMAL=0` is accidentally set, assert the token is issued with `exp = iat` (immediately expired) — this should cause the token to be rejected on first use, serving as a live integration test that the TTL is enforced
- [ ] **Negative TTL**: If a misconfiguration causes `ttl_for_role` to return a negative duration, assert the token issuance fails with a clear error (not a token with `exp < iat` issued to a user)
- [ ] **Maximum TTL**: If `JWT_ACCESS_TTL_NORMAL=3600` (1 hour) is set, assert the token is issued with a 1-hour expiry — confirm the budget test (Story 2.5) still passes with the longer-lived token
- [ ] **Concurrent logins with different roles**: `given` a user who logs in as both a customer and an org_admin at the same time → `then` both tokens are issued with the correct TTL for their respective roles (5 minutes for both, since F-010 aligned them)

### Cleanup

- No state cleanup required — TTL tests are stateless assertions on token claims
- Integration tests that rely on `exp` timing must either use a mocked clock or account for real-time drift between test steps
- Environment variable overrides must be reset between test runs — use `std::env::remove_var` in test teardown to restore the default state
