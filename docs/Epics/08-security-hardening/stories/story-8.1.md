# Story 8.1: Enforce JWT typ Claim (at+jwt)

## Epic

[08-security-hardening](../security.md)

## Parent Epic Story

Story 8.1

## Summary

Enforce the JWT `typ` (type) claim as `at+jwt` (access token) in all JWT validation logic. This prevents type confusion attacks where a refresh token or API key is mistakenly accepted as an access token. This is a baseline security requirement that should be implemented first.

## Why This Story Exists

RFC 7519 defines the `typ` header parameter for JWTs. Sesame specifies `typ: at+jwt` for access tokens. Without type enforcement, a service might accidentally accept a different token type (refresh token, self-issued ID token) as an access token, bypassing authorization checks. The JWT document explicitly states: "Enforce `typ` claim in all services -- this is a baseline security requirement."

## Design Context

### Current State

- No `typ` claim enforcement in any service
- All JWT validation only checks signature, exp, iss, aud
- No differentiation between token types in validation

### typ Enforcement

Every access token MUST include:

```json
{
  "typ": "at+jwt"
}
```

The JOSE header:

```json
{
  "alg": "ES256",
  "typ": "at+jwt",
  "kid": "key_1"
}
```

### Validation Logic

```rust
pub fn validate_typ(claims: &AccessClaims) -> Result<(), AuthError> {
    if claims.typ != Some("at+jwt".to_string()) {
        return Err(AuthError::InvalidTokenType {
            expected: "at+jwt".to_string(),
            actual: claims.typ.unwrap_or_default(),
        });
    }
    Ok(())
}
```

### Token Type Differentiation

| Token Type | typ claim | Use Case |
|------------|-----------|----------|
| Access token | `at+jwt` | API requests (Bearer token) |
| Refresh token | Not a JWT | Opaque string (stored in Redis) |
| Self-issued ID token | `id+at+jwt` (future) | Client-side identity (not current) |

## Mermaid Diagrams

### typ Enforcement Flow

```mermaid
sequenceDiagram
    participant Service
    participant JWT as JWT Parser
    participant Cache as JWKS Cache

    Service->>JWT: Parse JWT
    JWT->>JWT: Extract typ from JOSE header
    JWT->>Service: typ = "at+jwt"
    Service->>Service: typ == "at+jwt"?
    Service-->>Service: YES -> proceed with validation
    Service-->>Client: 401 if typ != "at+jwt"
```

### Token Type Comparison

```mermaid
flowchart TD
    A[JWT received] --> B{Check typ}
    B -->|typ = at+jwt| C[Access token - Proceed]
    B -->|typ = id+at+jwt| D[Reject: wrong type]
    B -->|typ = jwt| E[Reject: wrong type]
    B -->|no typ| F[Reject: missing typ]
    B -->|typ = ""| G[Reject: empty typ]
    
    C --> H[Validate signature]
    D --> I[Return 401 InvalidTokenType]
    E --> I
    F --> I
    G --> I
```

## OpenAPI Changes

- No OpenAPI changes. `typ` is a JOSE header field, not part of the API schema.

## Design Doc References

- `design-doc.md` section 6.2: JWT Schema -- `typ: at+jwt` in standard claims table
- `design-doc.md` section 10.1: Token Security -- "Enforce typ claim in all services"

## Wiki Pages to Update/Create

- `topics/topic-jwt-schema.md`: Document typ requirement
- `topics/topic-token-security.md`: Document type enforcement

## Acceptance Criteria

- [ ] All 6 services enforce `typ == "at+jwt"` on JWT validation
- [ ] Tokens without `typ` claim are rejected
- [ ] Tokens with wrong `typ` value are rejected
- [ ] Rejection returns 401 with error "invalid_token_type"
- [ ] Unit tests verify: correct typ accepted, missing typ rejected, wrong typ rejected

## Dependencies

- Depends on Story 1.1 (asymmetric key generation)
- This is a foundational story -- implement first

## Risk / Trade-offs

- **Breaking change**: If any current services issue JWTs without `typ`, enforcing this will break them. However, this is a security requirement that must be implemented regardless. Any services without `typ` must be updated before this story is considered complete.
- **No operational impact**: Enforcing `typ` does not change the token format -- it only adds a validation check. The token format already includes `typ: at+jwt` (see design-doc.md), so this is a validation improvement, not a format change.
- **Future token type extensibility**: If future token types are introduced (e.g., `id+at+jwt` for self-issued ID tokens), the typ check must be service-context-aware — an endpoint expecting access tokens rejects `id+at+jwt`, but a client-identity endpoint might accept it. The typ enforcement in the JWT middleware applies to Bearer-token API access only.

## Tests

### Unit Tests

- [ ] **Valid typ at+jwt accepted**: Given a JWT with JOSE header `typ = "at+jwt"`, assert `validate_typ()` returns `Ok(())` — no error
- [ ] **Missing typ claim rejected**: Given a JWT with no `typ` in the JOSE header, assert `validate_typ()` returns `AuthError::InvalidTokenType { expected: "at+jwt", actual: "" }`
- [ ] **Wrong typ rejected (typ=jwt)**: Given a JWT with `typ = "jwt"`, assert validation returns `AuthError::InvalidTokenType { expected: "at+jwt", actual: "jwt" }`
- [ ] **Wrong typ rejected (typ=id+at+jwt)**: Given a JWT with `typ = "id+at+jwt"`, assert validation rejects it for API access (wrong type)
- [ ] **Empty typ rejected**: Given a JWT with `typ = ""` (empty string), assert validation rejects it — empty typ is not valid
- [ ] **typ is case-sensitive**: Given a JWT with `typ = "AT+JWT"` (uppercase), assert validation rejects it — typ comparison is case-sensitive
- [ ] **typ rejects whitespace**: Given a JWT with `typ = " at+jwt"` (leading space) or `typ = "at+jwt "` (trailing space), assert validation rejects it — no trimming
- [ ] **typ rejected when set to refresh token identifier**: Given a JWT where typ = "refresh" (even if it looks like a valid JWT), assert it is rejected as an access token
- [ ] **Error message includes expected and actual typ**: Given a JWT with `typ = "self-issued"`, assert the error message is `"Invalid token type: expected at+jwt, got self-issued"` for clear debugging
- [ ] **typ check occurs before signature verification**: Assert the typ check happens in the JWT validation pipeline BEFORE signature verification — a token with wrong typ is rejected immediately without computing or checking the signature (defense in depth, prevents unnecessary crypto work)
- [ ] **typ check occurs after header parsing**: Assert the JOSE header is successfully parsed and the `typ` field is extracted before validation — a malformed JOSE header causes a parse error, not a typ error

### Integration Tests (BDD-style with `rstest_bdd`)

- [ ] **Scenario: Login service issues typ at+jwt**: `given` a successful login flow → `when` the access token is parsed → `then` the JOSE header contains `typ: "at+jwt"` and the payload is correctly decoded
- [ ] **Scenario: Service rejects token without typ**: `given` a client sends a JWT with no `typ` in the JOSE header → `when` the request reaches the JWT middleware → `then` the response is 401 with error code "invalid_token_type"
- [ ] **Scenario: Service rejects token with wrong typ**: `given` a client sends a JWT with `typ: "jwt"` → `when` the request reaches the JWT middleware → `then` the response is 401 with error code "invalid_token_type" and message "expected at+jwt, got jwt"
- [ ] **Scenario: All 6 services enforce typ**: `given` a token with missing `typ` → `when` the token is sent to each of the 6 services → `then` all 6 services reject it with 401 invalid_token_type (no service accepts it)
- [ ] **Scenario: typ enforcement works with JWKS validation**: `given` a JWT with valid signature, valid exp, valid iss, valid aud, but `typ: "jwt"` → `when` the service validates → `then` the typ check fails BEFORE signature verification completes (or fails regardless)
- [ ] **Scenario: typ enforcement works with HS256 tokens**: `given` a JWT signed with HS256 and `typ: "at+jwt"` → `when` the service validates → `then` it is accepted (typ enforcement is algorithm-independent)
- [ ] **Scenario: typ enforcement works with ES256 tokens**: `given` a JWT signed with ES256 and `typ: "at+jwt"` → `when` the service validates → `then` it is accepted

### Security Regression Tests

- [ ] **Refresh token cannot be used as access token**: Given a refresh token (opaque string, not a JWT) is sent as a Bearer token, assert it is rejected — either it fails JWT parsing (not a valid JWT) or it fails typ enforcement if it happens to be a JWT
- [ ] **Self-issued ID token cannot bypass authz**: Given a self-issued JWT with `typ: "id+at+jwt"` is sent as a Bearer token to an API endpoint, assert it is rejected by typ enforcement — it cannot bypass authorization
- [ ] **Typ claim cannot be used to confuse the validator**: Given an attacker crafts a JWT with `typ: "at+jwt"` but with an invalid signature or expired timestamp, assert the typ check passes but signature/exp checks still reject it — typ alone does not grant access
- [ ] **No information leakage through typ error message**: Assert the error message for wrong typ does not leak internal token processing details — it should say "invalid_token_type" or "expected at+jwt, got X" without revealing the validation pipeline order or internal structures
- [ ] **Typ enforcement does not create a side-channel**: Assert that the time-to-reject for a wrong-typ token is approximately the same as for a wrong-signature token — reject at typ parse time so timing-based attacks cannot distinguish "missing typ" from "valid typ + bad signature"

### Edge Cases

- [ ] **JWT with typ but no header (JWS compact format)**: Given a JWT in compact serialization where the header base64url-decodes correctly and contains `typ`, assert the typ is extracted and validated
- [ ] **JWT header with typ as non-string (JSON number)**: Given a JWT where the header has `typ: 123` (number instead of string), assert the handler rejects it — typ must be a string per JWT spec, not a number
- [ ] **JWT header with typ as JSON object**: Given a JWT where the header has `typ: {"value": "at+jwt"}`, assert the handler rejects it — typ must be a plain string, not a complex type
- [ ] **JWT with typ containing null bytes**: Given a JWT header with `typ: "at+\u0000jwt"`, assert the handler rejects it — typ must be ASCII alphanumeric plus + and . characters only
- [ ] **Extremely long typ value**: Given a JWT with `typ` set to a 10KB string, assert the handler rejects it — typ should be bounded to a reasonable length (e.g., 64 chars max)
- [ ] **JWT with multiple typ values (JSON array)**: Given a malformed header where typ is an array `[\"at+jwt\", \"id+at+jwt\"]`, assert the handler rejects it — typ must be a single string

### Cleanup

- [ ] No state changes are needed — typ enforcement is a read-only validation check with no cache or database writes
- [ ] Metrics registry must be reset between test scenarios using `prometheus::Registry::new()` to prevent cross-test metric contamination
- [ ] Test JWT fixtures must be isolated — each test should generate its own JWT or use a unique `jti` to prevent key collisions between concurrent tests
- [ ] No temporary files should be left in the filesystem after test runs
- [ ] If tests use a shared JWT signing key, ensure the key is not persisted between tests — use fresh keys per test or a test-specific key store
