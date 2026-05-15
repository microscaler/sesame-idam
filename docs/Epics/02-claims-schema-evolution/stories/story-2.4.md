# Story 2.4: Add Tenant to JWT Claims

## Epic

[02-claims-schema-evolution](../claims.md)

## Parent Epic Story

Story 2.4

## Summary

Add the `tenant_id` claim (UUID) to every access token in both the top-level and the namespaced claims structure. This enables downstream services to validate tenant context without a database call, enforcing the hard-segment multi-tenant isolation boundary defined in the Tenancy Model.

## Why This Story Exists

The JWT document identifies "No tenant claim (despite multi-tenancy being core to Sesame)" as a gap. The Tenancy Model wiki states that `tenant_id` is the hard-segment isolation boundary with "zero bleed" between tenants. Every API request includes `X-Tenant-ID`, but the JWT does not carry this claim. Embedding it in the token enables downstream services to validate tenant context without a database call.

## Design Context

### Current State

- Tenants are identified by the `X-Tenant-ID` header in API requests
- `tenant_id` is not in the JWT claims
- The design doc mentions `tenant_id` in the JWT payload example (section 6.2) but the current implementation does not include it
- The Tenancy Model wiki states: "Do include `tenant_id` in JWT claims (`access_token` payload)" -- this is a documented requirement, not yet implemented

### Tenancy Model Requirements

From `topics/topic-tenancy-model.md`:
- **Tenant = Hard-Segment isolation boundary**
- **Same email on different tenants = unrelated users**
- **UNIQUE(tenant_id, email)** prevents duplicate emails within a tenant
- **No cross-tenant identity**: Users, orgs, keys, sessions NEVER cross tenant boundaries
- **Do**: include `tenant_id` in JWT claims
- **Do not**: use `tenant_id` in user-facing URLs

### Claim Placement

`tenant_id` appears in two places:
1. **Top-level**: `tenant_id` at the root of the JWT payload (convenience, backward compat with design doc examples)
2. **Namespaced**: `https://sesame-idam.dev/claims.tenant` in the namespaced authz claims

Both contain the same UUID value. This redundancy ensures that:
- Code that reads the top-level `tenant_id` (from the design doc) still works
- New code reads from the namespaced `sx.tenant` (collision-resistant)

## Implementation Notes

### Claim Population

In the login handler:

```
1. Extract tenant_id from X-Tenant-ID header (or API key context)
2. Set claims.tenant_id = tenant_id
3. Set claims.sx.tenant = tenant_id
4. Validate that the user's tenant matches the request tenant_id
```

### Validation in Downstream Services

```rust
impl AccessClaims {
    pub fn validate_tenant(&self, expected_tenant: &str) -> Result<(), JwtError> {
        if self.tenant_id != expected_tenant {
            return Err(JwtError::TenantMismatch {
                expected: expected_tenant.to_string(),
                actual: self.tenant_id.clone(),
            });
        }
        if self.sx.tenant != expected_tenant {
            return Err(JwtError::TenantMismatch {
                expected: expected_tenant.to_string(),
                actual: self.sx.tenant.clone(),
            });
        }
        Ok(())
    }
}
```

### Tenancy in Different User Types

| User Type | tenant_id in JWT | Notes |
|-----------|-----------------|-------|
| `customer` | Same as request `X-Tenant-ID` | User belongs to this tenant |
| `platform` | Same as request `X-Tenant-ID` | Platform user operates within this tenant |
| `platform_admin` (global) | May be null or "all" | Platform admins may operate across tenants; this is an enterprise feature |

Platform admin tokens that operate across tenants are an **Enterprise opt-in** feature per the Tenancy Model. For the base implementation, all tokens have a single `tenant_id` matching the request tenant.

## Mermaid Diagrams

### Tenant Claim Flow

```mermaid
sequenceDiagram
    participant Client as Consumer App
    participant Gateway as API Gateway
    participant Sesame as Sesame-IDAM
    participant Service as Downstream Service

    Client->>Gateway: POST /api/resource<br/>X-Tenant-ID: tenant_A
    Gateway->>Gateway: Extract tenant_id from X-Tenant-ID
    Gateway->>Service: POST /api/resource<br/>X-Tenant-ID: tenant_A<br/>Authorization: Bearer ***
    Service->>Service: Validate JWT
    Service->>Service: Extract claims.tenant_id from JWT
    Service->>Service: Compare JWT tenant_id == request tenant_id
    alt Match
        Service->>Service: Process request with tenant context
    else Mismatch
        Service->>Client: 401 Tenant Mismatch
    end
```

### Multi-Tenant Isolation in JWT

```mermaid
graph TB
    subgraph "Tenant A (hauliage)"
        A1["User: alice@hauliage.com"]
        A2[JWT: tenant_id = tenant_hauliage]
        A1 --> A2
    end
    subgraph "Tenant B (rerp)"
        B1["User: alice@rerp.com"]
        B2[JWT: tenant_id = tenant_rerp]
        B1 --> B2
    end
    
    Note: alice@hauliage.com and alice@rerp.com are COMPLETELY unrelated users
    Note: Their JWTs have DIFFERENT tenant_id claims
    Note: No cross-tenant identity exists
```

### Downstream Tenant Validation

```mermaid
flowchart TD
    A[Downstream service receives request] --> B[Extract JWT]
    B --> C{Validate JWT signature}
    C -->|Fail| Z[Reject 401]
    C -->|Pass| D{Extract tenant_id from JWT}
    D --> E{Extract tenant_id from X-Tenant-ID header}
    E --> F{Do they match?}
    F -->|No| Z[Reject 401 Tenant Mismatch]
    F -->|Yes| G{Is tenant_id valid?}
    G -->|No| Z[Reject 401 Invalid Tenant]
    G -->|Yes| H[Process request with tenant context]
```

## OpenAPI Changes

- `LoginResponse` schema: Add `tenant_id` field (string, UUID format)
- No changes to request schemas needed (tenant_id is derived from `X-Tenant-ID` header)

```yaml
components:
  schemas:
    LoginResponse:
      type: object
      properties:
        tenant_id:
          type: string
          format: uuid
          description: Tenant UUID (hard-segment isolation boundary)
```

## Design Doc References

- `design-doc.md` section 6.2: JWT Schema -- `tenant_id` field already present in the design doc example
- `design-doc.md` section 5.0: Multi-Tenant Partitioning -- tenant_id column on every major entity
- `topics/topic-tenancy-model.md`: "Do include `tenant_id` in JWT claims (`access_token` payload)"
- `topics/topic-tenancy-model.md`: Tenancy Model Core Rules -- tenant_id maps to isolation boundary

## Wiki Pages to Update/Create

- `topics/topic-tenancy-model.md`: Verify that JWT tenant_id is documented as implemented
- `topics/topic-jwt-schema.md`: Document tenant_id in JWT claims

## Acceptance Criteria

- [ ] `tenant_id` (UUID) is included in every access token at the top level
- [ ] `tenant_id` is included in the namespaced claims: `https://sesame-idam.dev/claims.tenant`
- [ ] Both top-level and namespaced `tenant_id` contain the same UUID value
- [ ] The `tenant_id` in the JWT matches the `X-Tenant-ID` header from the login request
- [ ] Downstream services can validate `claims.tenant_id` against the request's `X-Tenant-ID`
- [ ] Platform admin tokens (if implemented) have the correct `tenant_id`
- [ ] Unit tests verify: tenant_id matches request, tenant_id is valid UUID, tenant_id is present in both locations

## Dependencies

- Depends on Story 2.2 (claims struct implementation includes `tenant_id` field)
- Intersects with Epic 4 (hybrid authz) for tenant-scoped route classification

## Risk / Trade-offs

- **Tenant ID in JWT is not encrypted**: The JWT is signed, not encrypted. Any service that can decode the JWT can see the `tenant_id`. This is intentional -- `tenant_id` is not a secret, it is an authorization boundary. The risk is that a stolen JWT reveals the tenant, which is already visible from the `X-Tenant-ID` header.
- **Platform admin cross-tenant access**: Platform admins (e.g., Sesame operators) may need to operate across tenants. This is an Enterprise opt-in feature. For the base implementation, all tokens have a single `tenant_id`. Cross-tenant admin tokens would use a special `tenant_id` value (e.g., "all" or null) with elevated risk level.
- **No tenant change after token issue**: A token's `tenant_id` is fixed at issue time. If a user moves between tenants (unlikely, as users are strictly scoped to one tenant), they must re-authenticate to get a new token. This is correct -- there is no cross-tenant identity.

## Tests

### Unit Tests

- [ ] **`tenant_id` matches between top-level and namespaced claims**: Create an `AccessClaims` with `tenant_id = "tenant-abc"` and `sx.tenant = "tenant-abc"`, assert both fields are identical (consistency invariant)
- [ ] **`tenant_id` is a valid UUID format**: Generate a `tenant_id` string and assert it matches the UUID regex pattern (`[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}`)
- [ ] **`validate_tenant()` rejects mismatch**: Create an `AccessClaims` with `tenant_id = "tenant-abc"`, call `validate_tenant("tenant-def")`, assert it returns `JwtError::TenantMismatch { expected: "tenant-def", actual: "tenant-abc" }`
- [ ] **`validate_tenant()` accepts match**: Create an `AccessClaims` with `tenant_id = "tenant-abc"`, call `validate_tenant("tenant-abc")`, assert it returns `Ok(())`
- [ ] **Top-level `tenant_id` populates on login**: Given a login request with `X-Tenant-ID: tenant-abc`, assert the issued JWT payload contains `tenant_id: "tenant-abc"` at the root level
- [ ] **Namespaced `sx.tenant` populates on login**: Same login request — assert the decoded JWT payload contains `"https://sesame-idam.dev/claims": { "tenant": "tenant-abc", ... }`
- [ ] **User's actual tenant matches request tenant**: In the login handler, given a user record with `tenant_id = "tenant-abc"` and a login request with `X-Tenant-ID: tenant-def`, assert the login is rejected (401) — users cannot authenticate as a different tenant

### Integration Tests (BDD-style with `rstest_bdd`)

- [ ] **Scenario: Tenant ID flows from login to token**: `given` a user belonging to tenant `hauliage` (UUID: `abc-123`) → `when` the user logs in with `X-Tenant-ID: abc-123` → `then` the access token's `tenant_id` field equals `abc-123` in both top-level and `sx.tenant`
- [ ] **Scenario: Cross-tenant login is rejected**: `given` a user registered under tenant `hauliage` → `when` the user attempts to login with `X-Tenant-ID: rerp` → `then` the login returns 401 with a tenant-mismatch error (not a password error — prevents tenant enumeration)
- [ ] **Scenario: Downstream service validates tenant**: `given` a JWT with `tenant_id = "hauliage"` → `when` a downstream service receives the request with `X-Tenant-ID: rerp` → `then` the service rejects the request with 401 Tenant Mismatch (the JWT tenant and header tenant don't match)
- [ ] **Scenario: Tenant ID present in LoginResponse**: `given` a successful login → `when` the `LoginResponse` is returned → `then` it includes a `tenant_id` field matching the tenant of the authenticated user
- [ ] **Scenario: Different users on different tenants have different JWT tenants**: `given` user alice on tenant `hauliage` and user alice on tenant `rerp` → `when` both login → `then` alice@hauliage's JWT has `tenant_id = hauliage_uuid` and alice@rerp's JWT has `tenant_id = rerp_uuid` — confirming zero cross-tenant identity

### Security Regression Tests

- [ ] **Tenant ID cannot be forged in token**: If a client modifies the `tenant_id` claim in a validly-signed token, assert that the JWT signature verification fails (the token cannot be tampered with — only the issuer can set the tenant)
- [ ] **Tenant ID matches request header**: For every login request, assert `claims.tenant_id == X-Tenant-ID` header value — never allow the token's tenant to differ from the request's tenant
- [ ] **No tenant_id leakage across login sessions**: Assert that a login to tenant A never results in a JWT containing tenant B's UUID (test with sequential logins to different tenants using the same client)

### Edge Cases

- [ ] **Malformed tenant_id UUID**: If a request sends `X-Tenant-ID: not-a-uuid`, assert the login fails before token generation (tenant validation happens before JWT issuance)
- [ ] **Null tenant_id**: If the user's database record has no `tenant_id` (data corruption edge case), assert the login handler rejects it (500 error, not a token issued with null tenant)
- [ ] **Tenant ID with special characters in UUID**: UUIDs should only contain hex characters and hyphens — assert any `tenant_id` containing non-hex characters is rejected

### Cleanup

- Tenant data in the test database must be isolated per test — use unique UUIDs for each test's tenant to prevent cross-test leakage
- Redis cache keys prefixed with tenant_id must be cleaned between test scenarios (use `DEL hauliage:*` or a per-test Redis namespace)
- Integration tests must not leave tokens in a "stale" state — tokens are 5-minute TTL, so they expire naturally; no explicit cleanup needed for JWT tokens
