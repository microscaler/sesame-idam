# Story 9.6: Implement Structured JWT Logging

## Epic

[09-observability](../observability.md)

## Parent Epic Story

Story 9.6

## Summary

Implement per-request structured JWT logging with standard fields: issuer, subject, client_id, session_id, token_id (jti), token_version, route, decision_source, actor subject (when act is present). NEVER log raw access tokens or refresh tokens. Log at INFO level for audit trail, WARN for mismatches, ERROR for validation failures.

## Why This Story Exists

The JWT document requires: "Per-request structured log fields: issuer, subject, client_id, session_id, token_id, token_version, route, decision_source (jwt/fallback/denylist/version_mismatch), actor subject when act is present. NEVER log raw access tokens or refresh tokens." Structured logging enables programmatic analysis of JWT decisions, incident investigation, and compliance reporting.

## Design Context

### Current State

- No structured JWT logging
- JWT validation failures appear in service logs but without standard fields
- No decision_source field (can't tell if JWT or authz-core made the decision)
- No actor subject in logs (delegation events are untraceable)

### Structured Log Format

```json
{
  "timestamp": "2026-05-15T22:30:00Z",
  "level": "WARN",
  "service": "identity-user-mgmt-service",
  "event": "jwt_validation",
  "issuer": "https://idam.example.com",
  "subject": "user_123",
  "client_id": "web-portal",
  "session_id": "ses_01JV8W...",
  "token_id": "tok_abc123",
  "token_version": 42,
  "route": "/api/v1/identity/users/me",
  "decision_source": "jwt_claims",
  "actor_subject": null,
  "result": "allowed",
  "method": "GET"
}
```

### Decision Source Values

| Value | When Used |
|-------|-----------|
| `jwt_claims` | JWT common path evaluated and decided |
| `fallback_cached` | Online fallback result came from cache |
| `fallback_online` | Online fallback called authz-core |
| `denylist` | Token was in jti denylist |
| `version_mismatch` | claims.ver < cached_ver |
| `online_only` | Route was online-only, always called authz-core |

### Actor Subject

The actor subject is populated when the JWT contains an `act` claim (delegation):

```json
{
  "actor_subject": "support_agent_456"
}
```

When there is no act claim, it is null:

```json
{
  "actor_subject": null
}
```

### Security: Never Log Tokens

Raw access tokens and refresh tokens MUST NEVER appear in logs. This is a hard requirement -- tokens are secrets that should never be persisted in log files, SIEMs, or any persistent storage.

```rust
// WRONG: logs the raw token
error!("Invalid token: {}", token);  // NEVER DO THIS

// CORRECT: logs the token ID (jti)
error!("Invalid token: jti={}", claims.jti);
```

### Logging Levels

| Event | Level | Fields Logged |
|-------|-------|--------------|
| JWT allowed | INFO | All standard fields |
| JWT denied | WARN | All standard fields + error_reason |
| Validation failure | ERROR | All standard fields + error_details |
| Version mismatch | WARN | All standard fields + expected_ver, actual_ver |
| Token revocation | WARN | All standard fields + revocation_reason |
| Delegation | INFO | All standard fields + actor_subject + delegation_type |

## Mermaid Diagrams

### Structured Log Flow

```mermaid
sequenceDiagram
    participant JWT as JWT Middleware
    participant Logger as Structured Logger
    participant SIEM as SIEM / Log Aggregator
    participant Grafana

    JWT->>Logger: Log structured JSON with JWT fields
    Logger->>Logger: Validate: no raw tokens in fields
    Logger->>SIEM: Async write (non-blocking)
    SIEM->>Grafana: Index for query
    Grafana->>Grafana: Dashboard: JWT decisions by route
    
    alt JWT allowed
        Logger->>Logger: level=INFO
    else JWT denied
        Logger->>Logger: level=WARN
    else Validation failure
        Logger->>Logger: level=ERROR
    end
```

### Log Field Completeness

```mermaid
flowchart TD
    A[JWT Request] --> B{Structured log fields}
    B --> C[timestamp]
    B --> D[level]
    B --> E[service]
    B --> F[event]
    B --> G[issuer]
    B --> H[subject]
    B --> I[client_id]
    B --> J[session_id]
    B --> K[token_id / jti]
    B --> L[token_version]
    B --> M[route]
    B --> N[decision_source]
    B --> O[actor_subject]
    B --> P[result]
    B --> Q[method]
    
    B --> R[NEVER: raw_token]
    R -.->|Security requirement| S[Validate: no PII in logs]
```

### Decision Source Flow

```mermaid
flowchart TD
    A[Request] --> B{Decision source?}
    B -->|jwt_claims| C[LOG: decision_source=jwt_claims]
    B -->|fallback_cached| D[LOG: decision_source=fallback_cached]
    B -->|fallback_online| E[LOG: decision_source=fallback_online]
    B -->|denylist| F[LOG: decision_source=denylist]
    B -->|version_mismatch| G[LOG: decision_source=version_mismatch]
    B -->|online_only| H[LOG: decision_source=online_only]
    
    C --> I[Result: allowed/denied]
    D --> I
    E --> I
    F --> I
    G --> I
    H --> I
```

## OpenAPI Changes

No OpenAPI changes. Logging is internal to the service.

## Design Doc References

- `design-doc.md` section 10.12: Observability -- structured JWT logging format
- `design-doc.md` section 10.5: Delegation & Actor Claims -- actor subject in logs

## Wiki Pages to Update/Create

- `topics/topic-observability.md`: Document structured log format
- `topics/topic-token-security.md`: Document security: never log tokens

## Acceptance Criteria

- [ ] All JWT validation requests produce structured JSON log entries
- [ ] All required fields are present: timestamp, level, service, event, issuer, subject, client_id, session_id, token_id, token_version, route, decision_source, actor_subject, result, method
- [ ] decision_source is one of: jwt_claims, fallback_cached, fallback_online, denylist, version_mismatch, online_only
- [ ] actor_subject is populated when act claim is present, null otherwise
- [ ] Raw tokens are NEVER in log entries (verified by unit test)
- [ ] PII fields (email, phone) are NEVER in log entries
- [ ] Logging is async (non-blocking) -- does not add latency to request
- [ ] Log levels: INFO for allowed, WARN for denied, ERROR for validation failures
- [ ] Unit tests verify: structured log format, no raw tokens, required fields present

## Dependencies

- Depends on Story 4.2 (JWT middleware -- where logging happens)
- Depends on Story 6.1 (act claim -- for actor_subject field)
- Can be implemented in parallel with other epics

## Risk / Trade-offs

- **Log volume**: Structured JSON logging is more verbose than plain text. At 10,000 RPS, this generates ~600,000 log lines per minute. Mitigation: use DEBUG level for successful JWT validations (low-value logs), INFO/WARN/ERROR for security-relevant events. The `event: "jwt_validation"` field allows filtering to only security-relevant events.
- **Subject privacy**: The subject (user_id) is logged in every request. This is necessary for audit trail but could be considered PII in some jurisdictions. Mitigation: user_id is an opaque identifier (not email or phone), so it's not PII under GDPR. If user_id needs to be hashed for privacy, add a `hash_subject` flag in the logging configuration.
- **Token_id vs raw token**: The `token_id` (jti) is logged, not the raw token. The jti is a UUID that identifies the token but cannot be used as a token. This is the correct approach -- the jti is metadata, the raw token is a secret.
