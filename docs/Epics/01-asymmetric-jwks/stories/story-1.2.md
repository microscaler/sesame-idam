# Story 1.2: Implement JWKS Publication Endpoint

## Epic

[01-asymmetric-jwks](../JWT.md)

## Parent Epic Story

Story 1.2

## Summary

Implement the `/.well-known/jwks.json` endpoint that serves the current set of public signing keys in standard JWKS format (RFC 7517). The endpoint is near-static (cached, NEGLIGIBLE cost per topology design) and includes the `kid` for key identification by validating services.

## Why This Story Exists

The JWT document recommends publishing discovery metadata and a JWKS document so resource servers can validate tokens locally (RFC 8414 + OIDC Discovery). The generated runtime already supports `JwksBearerProvider` with issuer, audience, leeway, and cache TTL configuration. This story wires that runtime support to serve dynamic keys.

## Design Context

### Current State

- `identity-session-service` already declares `/.well-known/jwks.json` in its OpenAPI spec
- The service is classified as EXTREME frequency, NEGLIGIBLE per-request cost
- The generated runtime has `JwksBearerProvider` which serves JWKS for validation
- Currently the endpoint likely serves a static key or the development fallback

### JWKS Format (RFC 7517)

```json
{
  "keys": [
    {
      "kty": "EC",
      "crv": "P-256",
      "kid": "key-2026-05-01",
      "alg": "ES256",
      "x": "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU",
      "y": "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWIPLA"
    }
  ]
}
```

### Key Points

- JWKS is a **set** of keys (not just one) to support overlapping rotation
- `kty` = "EC" for ES256, "OKP" for EdDSA, "RSA" for RS256
- `crv` specifies the curve (P-256, Ed25519)
- `alg` indicates the intended algorithm
- `kid` identifies which key to use for verification (matches `kid` in JWT header)

## Implementation Notes

### Endpoint Path

`GET /.well-known/jwks.json`

### Cache Behavior

The JWKS response is near-static and served from memory:
- The entire JWKS document is built from the current `KeyManager` state
- No database queries required
- Response is served directly from the in-memory key set
- HTTP `Cache-Control: public, max-age=300` (5 minutes, matches JWKS cache TTL from design doc section 10.11)

### Rate Limiting (F-009 Fix)

The JWKS endpoint is public and has no authentication. Without rate limiting, an attacker could:
- Send hundreds of requests/second to exhaust NGINX worker connections
- Force repeated JSON serialization, consuming CPU
- Amplify a DoS against identity-session-service

**Rate limit configuration:**
- 100 requests/second per IP (global, not per-route)
- Return 429 Too Many Requests when exceeded
- Log rate limit violations for security monitoring
- Implement using NGINX `limit_req` or application-level middleware (e.g., `tower_http::limit::RateLimitLayer`)

**NGINX rate limit config:**
```nginx
limit_req_zone $binary_remote_addr zone=jwks_limit:10m rate=100r/s;

location /.well-known/jwks.json {
    limit_req zone=jwks_limit burst=50 nodelay;
    ...
}
```

### Key Set Construction

On each request:
1. Clone the current `KeyManager` state
2. Build JWKS JSON from all keys currently in the manager (current, next, and any in grace period)
3. Return JSON response

This ensures:
- The JWKS always contains at least one valid key
- During rotation, both old and new keys are visible
- After grace period, only the current key is visible

### Response Headers

| Header | Value | Reason |
|--------|-------|--------|
| `Content-Type` | `application/json` | Standard for JWKS |
| `Cache-Control` | `public, max-age=300` | 5-minute cache, matches JWKS cache TTL |
| `X-Content-Type-Options` | `nosniff` | Prevent MIME sniffing |
| `Vary` | `Accept` | Support future content negotiation |

### Content Size

Expected response size: ~500 bytes per key. With 1-2 keys during normal operation and 2-3 during rotation, the response is well under 2KB. This fits comfortably within:
- NGINX default `client_header_buffer_size`: 1KB (JWKS is served, not requested, so this applies to request headers, not response)
- Apache `LimitRequestFieldSize`: 8190 bytes (same, applies to requests)
- The response body is not subject to these limits -- only request headers are

## Mermaid Diagrams

### JWKS Serving Flow

```mermaid
sequenceDiagram
    participant Consumer as Consumer Service
    participant JWKS as /.well-known/jwks.json
    participant KM as KeyManager (in-memory)

    Consumer->>JWKS: GET /.well-known/jwks.json
    Note over JWKS: Cache-Control: max-age=300
    activate JWKS
    JWKS->>KM: Get current key set
    KM-->>JWKS: [current_key, next_key?, grace_keys?]
    JWKS->>JWKS: Build JWKS JSON
    JWKS-->>Consumer: { "keys": [...] }
    Note over JWKS: 200 OK<br/>~500 bytes per key
    deactivate JWKS
```

### Key Publication During Rotation

```mermaid
sequenceDiagram
    participant Tilt as Tilt/Consumer
    participant JWKS as JWKS endpoint
    participant KM as KeyManager

    Tilt->>JWKS: GET jwks.json
    JWKS->>KM: current only
    KM-->>JWKS: [key-2026-04]
    JWKS-->>Tilt: [key-2026-04]

    Note over KM: Rotation triggers
    KM->>KM: Generate key-2026-05
    KM->>JWKS: publish key-2026-05
    JWKS-->>Tilt: next GET: [key-2026-04, key-2026-05]

    Note over KM: key-2026-05 becomes current
    JWKS-->>Tilt: next GET: [key-2026-04, key-2026-05]

    Note over KM: grace period ends
    JWKS-->>Tilt: GET after grace: [key-2026-05]
    Note over KM: key-2026-04 dropped
```

## OpenAPI Changes

Add to `openapi/idam/identity-session-service/openapi.yaml`:

```yaml
paths:
  /.well-known/jwks.json:
    get:
      summary: JSON Web Key Set
      operationId: getJwks
      description: |
        Returns the current set of public signing keys in JWKS format (RFC 7517).
        Use this endpoint to validate JWT signatures. Keys may include current,
        next (preparing for rotation), and grace-period keys.
      responses:
        '200':
          description: JWKS document
          content:
            application/json:
              schema:
                type: object
                required: [keys]
                properties:
                  keys:
                    type: array
                    items:
                      $ref: '#/components/schemas/JsonWebKey'
```

Add new schema:

```yaml
components:
  schemas:
    JsonWebKey:
      type: object
      required: [kty, kid, alg, crv, x, y]
      properties:
        kty:
          type: string
          description: Key type (EC, OKP, RSA)
        kid:
          type: string
          description: Key identifier
        alg:
          type: string
          description: Intended algorithm (ES256, EdDSA, RS256)
        crv:
          type: string
          description: Curve (P-256, Ed25519)
        x:
          type: string
          description: X coordinate (base64url-encoded)
        y:
          type: string
          description: Y coordinate (base64url-encoded, EC only)
```

## Design Doc References

- `design-doc.md` section 10.2: Asymmetric Signing & JWKS
- `design-doc.md` section 10.11: Caching Strategy -- JWKS cache 5-minute TTL
- `service-topology-design.md`: identity-session-service serves `/.well-known/jwks.json` (EXTREME freq, NEGLIGIBLE cost)
- `design-doc.md` section 10.1: Token Security -- Key management property
- `design-doc.md` section 6.2: JWT Schema -- `kid` field in JWT header

## Wiki Pages to Update/Create

- `topics/topic-jwt-schema.md`: Add JWKS endpoint reference
- `topics/topic-token-lifecycle.md`: (new) Document key rotation lifecycle

## Acceptance Criteria

- [ ] `GET /.well-known/jwks.json` returns valid JWKS JSON (RFC 7517)
- [ ] Response includes all current and grace-period keys
- [ ] Each key includes `kty`, `kid`, `alg`, `crv`, and coordinate fields
- [ ] Response includes `Cache-Control: public, max-age=300`
- [ ] During key rotation, both old and new keys appear in the JWKS
- [ ] After the grace period, the old key is removed from the JWKS
- [ ] Response size is under 2KB (1-2 keys normal, 2-3 during rotation)
- [ ] Response is served from in-memory key set (no database queries)
- [ ] The endpoint has NEGLIGIBLE per-request cost (served from memory)

## Dependencies

- Depends on Story 1.1 (key generation and KeyManager)
- Required by Story 1.3 (other services fetch JWKS to validate tokens)

## Risk / Trade-offs

- **JWKS endpoint is public**: It does not require authentication. This is by design -- public key material should be discoverable. The risk is minimal since it only contains public keys.
- **Response is unversioned**: The JWKS doesn't include a `version` or `updated_at` field. Consumers must rely on `Cache-Control` headers. A `jku` (JWK Set URL) claim in the JWT itself could provide the authoritative source, but this adds complexity.
- **Single endpoint**: All services share one JWKS endpoint. If identity-session-service is down, no service can validate new tokens. This is acceptable because identity-session-service is classified as HIGH frequency and should be highly available.
