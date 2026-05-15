# Story 2.2: Implement New TokenClaims Rust Structs

## Epic

[02-claims-schema-evolution](../claims.md)

## Parent Epic Story

Story 2.2

## Summary

Implement the Rust structs that represent the new JWT claim structure: `ActorClaim` for RFC 8693 `act`, `SesameAuthzClaims` for the namespaced authz data, and `AccessClaims` as the top-level structure. Ensure backward-compatible deserialization during migration and validate required claims on parse.

## Why This Story Exists

The new claim structure defined in Story 2.1 needs Rust implementations that can serialize/deserialize JWT payloads, validate required claims, and provide typed access to each claim. This story implements those Rust types.

## Design Context

### Current Rust Types (from JWT document)

The JWT document references `TokenClaims` structure with registered claims (`sub`, `iss`, `aud`, `exp`, `iat`, `jti`) plus namespaced custom claims for `email`, `org_id`, `portal_type`, and `roles`. The current code signs tokens with HS256.

### Target Rust Types

```rust
// ActorClaim: RFC 8693 delegation actor
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActorClaim {
    pub sub: String,
    // Optional: tenant, portal, roles can be added later
}

// SesameAuthzClaims: namespaced authorization data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SesameAuthzClaims {
    pub tenant: String,
    pub portal: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub entitlements_ref: Option<String>,
    pub entitlements_hash: Option<String>,
    pub risk: Option<String>,
}

// AccessClaims: top-level JWT claim structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessClaims {
    // Standard claims
    pub iss: String,
    pub sub: String,
    pub aud: Vec<String>,
    pub client_id: String,
    pub scope: String,
    pub exp: i64,
    pub nbf: i64,
    pub iat: i64,
    pub jti: String,
    // Version claims
    pub ver: u64,
    pub sid: String,
    // Tenancy
    pub tenant_id: String,
    pub user_id: String,
    pub user_type: String,
    // Namespaced authz claims
    #[serde(rename = "https://sesame-idam.dev/claims")]
    pub sx: SesameAuthzClaims,
    // Optional delegation
    pub act: Option<ActorClaim>,
}
```

### Serialization Considerations

1. **URI key serialization**: The `https://sesame-idam.dev/claims` key must be serialized as a string, not a nested object. The `serde(rename = "...")` attribute handles this.

2. **Deserialization order**: When deserializing, JWT libraries typically deserialize the JOSE header first, then the payload. The payload is deserialized into `AccessClaims`. If the JWT was signed with an old schema (without `ver` or namespaced claims), deserialization fails or produces `None` for required fields.

3. **Backward-compatible deserialization**: Use `#[serde(default)]` on optional fields. However, `ver`, `sid`, and `sx` are required for the new schema -- old JWTs without them should be rejected.

### Validation on Parse

```rust
impl AccessClaims {
    pub fn validate(&self) -> Result<(), JwtValidationError> {
        // Issuer validation
        if !ALLOWED_ISSUERS.contains(&self.iss.as_str()) {
            return Err(JwtValidationError::InvalidIssuer);
        }
        // Audience validation
        if !self.aud.iter().any(|a| EXPECTED_AUDIENCE.contains(a)) {
            return Err(JwtValidationError::InvalidAudience);
        }
        // Token version must be present
        if self.ver == 0 {
            return Err(JwtValidationError::MissingVersion);
        }
        // Tenant must be present
        if self.tenant_id.is_empty() {
            return Err(JwtValidationError::MissingTenant);
        }
        // Authz claims namespace must be present
        if self.sx.tenant.is_empty() {
            return Err(JwtValidationError::MissingAuthzClaims);
        }
        // Risk claim must be valid if present
        if let Some(risk) = &self.sx.risk {
            if !["normal", "elevated", "critical"].contains(&risk.as_str()) {
                return Err(JwtValidationError::InvalidRisk);
            }
        }
        Ok(())
    }
}
```

## Implementation Notes

### Error Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum JwtValidationError {
    InvalidIssuer,
    InvalidAudience,
    MissingVersion,
    MissingTenant,
    MissingAuthzClaims,
    InvalidRisk,
    InvalidTokenVersion,
    Expired,
    NotYetValid,
    SignatureInvalid,
}
```

### JWT Claims Builder

```rust
impl AccessClaims {
    pub fn builder() -> AccessClaimsBuilder {
        AccessClaimsBuilder::new()
    }
}

pub struct AccessClaimsBuilder {
    claims: PartialAccessClaims,
}

impl AccessClaimsBuilder {
    pub fn new() -> Self { ... }
    pub fn iss(mut self, iss: String) -> Self { ... }
    pub fn sub(mut self, sub: String) -> Self { ... }
    pub fn aud(mut self, aud: Vec<String>) -> Self { ... }
    pub fn client_id(mut self, client_id: String) -> Self { ... }
    pub fn scope(mut self, scope: String) -> Self { ... }
    pub fn exp(mut self, exp: i64) -> Self { ... }
    pub fn nbf(mut self, nbf: i64) -> Self { ... }
    pub fn iat(mut self, iat: i64) -> Self { ... }
    pub fn jti(mut self, jti: String) -> Self { ... }
    pub fn ver(mut self, ver: u64) -> Self { ... }
    pub fn sid(mut self, sid: String) -> Self { ... }
    pub fn tenant_id(mut self, tenant_id: String) -> Self { ... }
    pub fn user_id(mut self, user_id: String) -> Self { ... }
    pub fn user_type(mut self, user_type: String) -> Self { ... }
    pub fn sx(mut self, sx: SesameAuthzClaims) -> Self { ... }
    pub fn act(mut self, act: ActorClaim) -> Self { ... }
    pub fn build(self) -> Result<AccessClaims, JwtError> { ... }
}
```

### Location in Codebase

The new types go in `common/src/jwt.rs` (or `common/src/token_claims.rs` as a new module). They must be part of the shared crate that all 6 services depend on.

## Mermaid Diagrams

### Claim Structure in Memory

```mermaid
classDiagram
    class AccessClaims {
        +String iss
        +String sub
        +Vec~String~ aud
        +String client_id
        +String scope
        +i64 exp
        +i64 nbf
        +i64 iat
        +String jti
        +u64 ver
        +String sid
        +String tenant_id
        +String user_id
        +String user_type
        +SesameAuthzClaims sx
        +Option~ActorClaim~ act
        +validate() Result
    }
    class SesameAuthzClaims {
        +String tenant
        +String portal
        +Vec~String~ roles
        +Vec~String~ permissions
        +Option~String~ entitlements_ref
        +Option~String~ entitlements_hash
        +Option~String~ risk
    }
    class ActorClaim {
        +String sub
    }
    AccessClaims --> SesameAuthzClaims : contains
    AccessClaims --> ActorClaim : contains (optional)
```

### Claim Construction Flow

```mermaid
sequenceDiagram
    participant Login as identity-login-service
    participant Authz as authz-core
    participant JWT as JWT claims builder

    Login->>Login: Authenticate user, verify password
    Login->>Authz: POST /principal/effective {user_id, org_id}
    Authz-->>Login: {roles, permissions, tenant_id, user_type}
    Login->>JWT: builder()
    JWT->>JWT: iss = "https://idam.example.com"
    JWT->>JWT: sub = user_id
    JWT->>JWT: aud = [audiences]
    JWT->>JWT: scope = "profile:read ..."
    JWT->>JWT: ver = increment_version(user_id)
    JWT->>JWT: sid = generate_session_id()
    JWT->>JWT: sx = SesameAuthzClaims {tenant, portal, roles, permissions}
    JWT->>JWT: build() -> AccessClaims
    JWT-->>Login: AccessClaims (validated)
    Login->>Login: sign(AccessClaims) -> JWT string
    Login-->>Client: {access_token, refresh_token}
```

### Validation Flow

```mermaid
flowchart TD
    A[JWT payload] --> B[Deserialize to AccessClaims]
    B --> C{Required fields present?}
    C -->|No| Z[Reject: MissingVersion / MissingTenant]
    C -->|Yes| D{iss in allow-list?}
    D -->|No| Z
    D -->|Yes| E{aud intersects expected?}
    E -->|No| Z
    E -->|Yes| F{ver >= 1?}
    F -->|No| Z
    F -->|Yes| G{sx.tenant not empty?}
    G -->|No| Z
    G -->|Yes| H{risk valid if present?}
    H -->|No| Z
    H -->|Yes| I[AccessClaims validated]
```

## OpenAPI Changes

No OpenAPI changes needed for Rust struct implementation (internal code). The OpenAPI schema changes are covered in Story 2.1.

## Design Doc References

- `design-doc.md` section 6.2: JWT Schema -- new namespaced structure
- `design-doc.md` section 10.1: Token Security -- claim structure
- `design-doc.md` section 10.4: Token Versioning -- `ver` claim implementation
- `design-doc.md` section 10.5: Delegation -- `ActorClaim` structure

## Wiki Pages to Update/Create

- `topics/topic-jwt-schema.md`: Document the Rust struct definitions
- `topics/topic-token-lifecycle.md`: Document claims builder pattern
- `topics/topic-claims-schema.md`: (new) Rust type specification

## Acceptance Criteria

- [ ] `ActorClaim` struct is implemented with `sub` field (RFC 8693)
- [ ] `SesameAuthzClaims` struct is implemented with all authz fields
- [ ] `AccessClaims` struct is implemented with all standard, version, tenancy, and authz fields
- [ ] The `https://sesame-idam.dev/claims` key is correctly serialized/deserialized using serde rename
- [ ] `AccessClaims::validate()` checks all required fields: `iss`, `aud`, `ver`, `tenant_id`, `sx.tenant`
- [ ] `AccessClaims::validate()` rejects invalid `risk` values
- [ ] All structs implement `Serialize`, `Deserialize`, `Clone`, `Debug`, `PartialEq`
- [ ] A JWT claims builder pattern is implemented for token construction
- [ ] The types are in the shared crate accessible to all 6 services
- [ ] Unit tests cover: valid claims, missing `ver`, missing `tenant_id`, missing `sx.tenant`, invalid `risk`, valid `act` claim

## Dependencies

- Depends on Story 2.1 (claim structure defined)
- Required by Story 2.2 (claims implementation), Story 3.1 (token construction in refresh), Story 4.2 (JWT middleware), Story 5.1 (version claim access)

## Risk / Trade-offs

- **Required fields**: `ver`, `sid`, and `sx` are required in the new schema. Old JWTs (signed during HS256 transition) will fail deserialization or validation. This is acceptable -- tokens have 5-minute TTLs, so old tokens expire quickly.
- **Serde rename**: The URI key `https://sesame-idam.dev/claims` requires `#[serde(rename = "...")]`. This works but is less ergonomic than a regular Rust field name. It is necessary per RFC 7519 for collision-resistant custom claims.
- **Builder pattern overhead**: The builder pattern adds code volume but provides clarity and validation at construction time. It is the right trade for an IAM system where token correctness is critical.

## Tests

### Unit Tests

- [ ] **`ActorClaim` round-trip**: Create `ActorClaim { sub: "user-123" }`, serialize to JSON, deserialize back, assert `sub` is identical
- [ ] **`SesameAuthzClaims` full struct round-trip**: Create a `SesameAuthzClaims` with all fields populated (tenant, portal, roles, permissions, entitlements_ref, entitlements_hash, risk), serialize and deserialize — assert all fields match
- [ ] **`SesameAuthzClaims` optional fields absent**: Create a `SesameAuthzClaims` with `entitlements_ref=None`, `entitlements_hash=None`, `risk=None` — serialize and assert the JSON does NOT contain these keys (not `null`, omitted entirely)
- [ ] **`AccessClaims` required field validation rejects missing `ver`**: Create a claims struct with `ver: 0` (or test deserialization of a payload missing `ver`), assert `validate()` returns `JwtValidationError::MissingVersion`
- [ ] **`AccessClaims` required field validation rejects missing `tenant_id`**: Create a claims struct with `tenant_id: ""` (empty string), assert `validate()` returns `JwtValidationError::MissingTenant`
- [ ] **`AccessClaims` required field validation rejects missing `sx.tenant`**: Create a claims struct with `SesameAuthzClaims { tenant: "", ... }`, assert `validate()` returns `JwtValidationError::MissingAuthzClaims`
- [ ] **`AccessClaims` validation rejects invalid `risk`**: Create a claims struct with `sx.risk = Some("unknown")`, assert `validate()` returns `JwtValidationError::InvalidRisk`
- [ ] **`AccessClaims` validation accepts valid `risk` values**: For each of `"normal"`, `"elevated"`, `"critical"`, assert `validate()` returns `Ok(())`
- [ ] **`AccessClaims` validation accepts valid claims**: Create a fully-populated, well-formed `AccessClaims` and assert `validate()` returns `Ok(())`
- [ ] **Builder pattern constructs valid claims**: Use `AccessClaims::builder()` to set all required fields and call `build()`, assert the result is `Ok(AccessClaims)` with all fields populated
- [ ] **Builder pattern rejects missing required fields**: Call `build()` on a builder missing `ver`, assert it returns a `JwtError` variant indicating a missing required field
- [ ] **All structs derive required traits**: Assert `ActorClaim`, `SesameAuthzClaims`, and `AccessClaims` all implement `Clone`, `Debug`, `PartialEq` (compile-time check — if they don't, the code won't compile)
- [ ] **`AccessClaims` contains `ActorClaim` optional**: Assert that `AccessClaims { act: None, ... }` serializes without the `act` key, and `AccessClaims { act: Some(ActorClaim{sub:"..."}), ... }` serializes with the `act` object

### Integration Tests (BDD-style with `rstest_bdd`)

- [ ] **Scenario: Claims deserialized from real JWT**: `given` a JWT issued by identity-login-service during a login flow → `when` the claims are decoded and deserialized into `AccessClaims` → `then` `validate()` returns `Ok(())` and all fields are populated correctly
- [ ] **Scenario: Legacy JWT fails validation**: `given` an old-format JWT (HS256, flat claims, no `ver`) → `when` it is deserialized into `AccessClaims` and `validate()` is called → `then` it returns `JwtValidationError::MissingVersion` (not a deserialization panic)
- [ ] **Scenario: Builder produces serializable claims**: `given` a claims builder with all fields set → `when` `build()` succeeds → `then` the resulting `AccessClaims` serializes to valid JSON matching the target schema from Story 2.1
- [ ] **Scenario: Shared crate types accessible from all services**: `given` the `common` crate with the new types → `when` any of the 6 services import `AccessClaims`, `SesameAuthzClaims`, or `ActorClaim` → `then` compilation succeeds (compile-time verification)

### Security Regression Tests

- [ ] **`validate()` rejects `iss` from non-trusted issuer**: Create an `AccessClaims` with `iss: "https://evil-issuer.example.com"` (not in `ALLOWED_ISSUERS`), assert `validate()` returns `JwtValidationError::InvalidIssuer`
- [ ] **`validate()` rejects `aud` with no match**: Create an `AccessClaims` with `aud: ["unknown-service"]`, assert `validate()` returns `JwtValidationError::InvalidAudience`
- [ ] **Token builder cannot set `ver=0`**: Attempt to build a token with `ver=0` — assert the builder rejects it (enforced at construction time, not just validation time)

### Edge Cases

- [ ] **Very long `tenant_id`**: Inject a 1000-character `tenant_id`, assert serialization succeeds and the token stays under the size budget
- [ ] **Very large `roles` array**: Inject 500 roles, assert serialization succeeds but the token size is flagged (test fails if over size budget — this is intentional to catch entitlement inflation)
- [ ] **`aud` vector empty**: Create `AccessClaims { aud: vec![] }`, assert `validate()` returns `InvalidAudience`
- [ ] **`scope` empty string**: Create `AccessClaims { scope: "" }`, assert the struct accepts it (empty scope is valid — it means no permissions, which is enforced by policy evaluation, not schema validation)

### Cleanup

- No state cleanup required — these are pure data type tests
- Integration tests must not leave partially-constructed `AccessClaims` in shared caches (if claims are cached, clear the cache between test scenarios)
