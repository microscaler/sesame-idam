# Story 8.3: Implement Security Audit Logging

## Epic

[08-security-hardening](../security.md)

## Parent Epic Story

Story 8.3

## Summary

Implement comprehensive security audit logging for all JWT operations: issuance, validation, version bumps, revocations, and delegation events. Logs must include sufficient detail for security incident investigation and compliance reporting.

## Why This Story Exists

The JWT document identifies security audit logging as critical: "Log all JWT issuance, validation failures, version bumps, revocations, and delegation events. Include issuer, subject, actor, scopes, decision_source in every log entry." Without audit logging, security incidents cannot be investigated and compliance requirements cannot be met.

## Design Context

### Current State

- No security audit logging
- JWT operations are not logged (issuance, validation, revocation)
- No audit trail for delegation events
- No log format standardization

### Audit Log Format

Every security event MUST include:

| Field | Description | Example |
|-------|-------------|---------|
| `event` | Event type | `jwt_issued`, `validation_failed`, `version_bump` |
| `timestamp` | ISO 8601 UTC | `2026-05-15T22:30:00Z` |
| `service` | Service name | `identity-login-service` |
| `tenant_id` | Tenant context | `tenant_abc` |
| `user_id` | Subject | `user_123` |
| `actor_id` | Actor (for delegation) | `support_agent_456` |
| `scopes` | Requested scopes | `profile:read,orders:write` |
| `decision_source` | How authorization was decided | `jwt_claims`, `authz_core`, `cached` |
| `result` | Success or failure | `allowed`, `denied`, `revoked` |

### Example Audit Entries

#### JWT Issuance

```json
{
  "event": "jwt_issued",
  "timestamp": "2026-05-15T22:30:00Z",
  "service": "identity-login-service",
  "tenant_id": "tenant_abc",
  "user_id": "user_123",
  "actor_id": null,
  "scopes": "profile:read orders:write",
  "decision_source": "jwt_claims",
  "result": "allowed",
  "token_version": 42,
  "ttl": 300,
  "algorithm": "ES256"
}
```

#### JWT Validation Failure

```json
{
  "event": "validation_failed",
  "timestamp": "2026-05-15T22:30:01Z",
  "service": "identity-user-mgmt-service",
  "tenant_id": "tenant_abc",
  "user_id": "user_123",
  "actor_id": null,
  "scopes": "profile:read",
  "decision_source": "jwt_claims",
  "result": "denied",
  "error": "stale_auth_token",
  "reason": "claims.ver (41) < cached_ver (42)"
}
```

#### Delegation Event

```json
{
  "event": "delegation",
  "timestamp": "2026-05-15T22:30:02Z",
  "service": "identity-login-service",
  "tenant_id": "tenant_abc",
  "user_id": "user_123",
  "actor_id": "support_agent_456",
  "scopes": "profile:read",
  "decision_source": "jwt_claims",
  "result": "allowed",
  "delegation_type": "support_impersonation",
  "actor_roles": ["support_agent"],
  "act_claim_present": true
}
```

### Logging Levels

| Event | Level | Rationale |
|-------|-------|-----------|
| JWT issued | INFO | Normal operation |
| JWT validated (allowed) | DEBUG | High volume, normal operation |
| JWT validated (denied) | WARN | Potential security issue |
| Token binding mismatch | ERROR | Active attack indicator |
| Version bump | INFO | Authorization change |
| Revocation | WARN | Security-relevant event |
| Delegation | INFO | Auditable action |
| Validation failure (stale token) | WARN | Security-relevant event |

## Mermaid Diagrams

### Audit Log Flow

```mermaid
flowchart TD
    A[JWT Operation] --> B{Operation type?}
    B -->|Issued| C[Log: jwt_issued]
    B -->|Validated - allowed| D[Log: jwt_validated]
    B -->|Validated - denied| E[Log: validation_denied]
    B -->|Revoked| F[Log: token_revoked]
    B -->|Delegated| G[Log: delegation]
    
    C --> H[Write to audit log]
    D --> H
    E --> H
    F --> H
    G --> H
    
    H --> I[Async to log aggregator]
    I --> J[Structured log format]
    J --> K[SIEM / compliance tool]
```

### Audit Log Structure

```mermaid
graph TB
    subgraph "Audit Log Entry"
        A[event: jwt_issued]
        B[timestamp: 2026-05-15T...]
        C[service: identity-login-service]
        D[tenant_id: tenant_abc]
        E[user_id: user_123]
        F[actor_id: support_agent_456]
        G[scopes: profile:read]
        H[decision_source: jwt_claims]
        I[result: allowed]
        J[token_version: 42]
    end
```

### Event Hierarchy

```mermaid
flowchart TD
    A[JWT Operations] --> B[Issuance]
    A --> C[Validation]
    A --> D[Revocation]
    A --> E[Delegation]
    A --> F[Version Management]
    
    B --> B1[jwt_issued]
    C --> C1[jwt_validated]
    C --> C2[validation_failed]
    D --> D1[token_revoked]
    D --> D2[family_revoked]
    E --> E1[delegation]
    F --> F1[version_bump]
    F --> F2[version_mismatch]
```

## OpenAPI Changes

No OpenAPI changes. Audit logging is internal -- no API surface is exposed.

## Design Doc References

- `design-doc.md` section 10.12: Observability -- Security audit logging
- `design-doc.md` section 10.5: Delegation & Actor Claims -- delegation audit
- `design-doc.md` section 10.4: Token Versioning & Revocation -- revocation audit

## Wiki Pages to Update/Create

- `topics/topic-security.md`: (new) Document audit logging requirements
- `topics/topic-token-lifecycle.md`: Document lifecycle audit events

## Acceptance Criteria

- [ ] All JWT operations are logged: issuance, validation, revocation, delegation, version bump
- [ ] Log format includes: event, timestamp, service, tenant_id, user_id, actor_id, scopes, decision_source, result
- [ ] Failed validations logged at WARN level
- [ ] Token binding mismatches logged at ERROR level
- [ ] Delegation events include actor_id and delegation_type
- [ ] Version bump events include old_ver, new_ver, reason
- [ ] All logs are structured JSON for machine parsing
- [ ] Logs are async (non-blocking) to avoid impacting request latency
- [ ] Metrics: `security_audit_log_total{event: "jwt_issued", "validation_failed", ...}` is emitted

## Dependencies

- Depends on Story 4.2 (JWT middleware -- where validation happens)
- Depends on Story 5.1 (version bump events)
- Depends on Story 6.1 (delegation events)

## Risk / Trade-offs

- **Log volume**: JWT validation happens on every request (133 endpoints across 6 services). At 10,000 RPS, this generates millions of log entries per hour. Mitigation: use DEBUG level for successful validations (low volume), INFO/WARN/ERROR for security-relevant events only.
- **PII in logs**: The log format includes user_id and tenant_id but NOT PII fields (email, phone). This is intentional -- PII must never be in audit logs. The JWT claims themselves do not include PII (Story 2.3), so this is naturally enforced.
- **Async logging**: To avoid impacting request latency, audit logging should be async (batched writes). This means logs may be delayed, but security events are captured even if the logging pipeline is temporarily unavailable.
- **Log storage retention**: Security logs must be retained for compliance (typically 90 days to 7 years depending on jurisdiction). The async write pipeline must handle storage backpressure — if the log aggregator is down, logs should be buffered (with a bounded buffer) rather than lost or blocking the request handler.

## Tests

### Unit Tests

- [ ] **JWT issuance log entry has correct event and fields**: Given a JWT is issued, assert the log entry contains `event: "jwt_issued"`, `service: "identity-login-service"`, a valid ISO 8601 timestamp, `user_id`, `tenant_id`, `scopes`, `decision_source: "jwt_claims"`, `result: "allowed"`, `token_version`, `ttl`, and `algorithm`
- [ ] **JWT validation success logged at DEBUG**: Given a JWT is validated successfully, assert the log level is `DEBUG` (not INFO or WARN) — high-volume normal operations should not flood logs
- [ ] **JWT validation denial logged at WARN**: Given a JWT is validated but denied (e.g., insufficient permissions), assert the log level is `WARN` — potential security issue
- [ ] **Token binding mismatch logged at ERROR**: Given a JWT with a valid signature but a binding mismatch, assert the log level is `ERROR` — active attack indicator
- [ ] **Version bump log includes old_ver and new_ver**: Given a token version is bumped from 41 to 42, assert the log entry contains `old_ver: 41`, `new_ver: 42`, and a `reason` field explaining the bump
- [ ] **Delegation log includes actor_id and delegation_type**: Given a delegation event for support impersonation, assert the log contains `actor_id: "support_agent_456"`, `delegation_type: "support_impersonation"`, `actor_roles: ["support_agent"]`, and `act_claim_present: true`
- [ ] **Token revocation logged**: Given a token is revoked (jti denylisted), assert the log entry contains `event: "token_revoked"`, `result: "revoked"`, and the `jti` of the revoked token
- [ ] **PII fields are never in log entries**: Assert that no log entry contains `email`, `phone`, `name`, or other PII fields — even when the JWT payload or request body includes them
- [ ] **Structured JSON format is valid**: Given any log entry, assert it is valid JSON that can be parsed by a structured log parser (e.g., `serde_json`)
- [ ] **Async log write does not block request handler**: Given a log write to the aggregator is slow (500ms), assert the request handler completes in <10ms — the log write must be fire-and-forget or batched
- [ ] **Log entry includes request ID for correlation**: Given an incoming request with a unique request ID, assert the request ID is included in all log entries for that request, enabling end-to-end tracing
- [ ] **Event type is one of the defined set**: Assert that every log entry's `event` field is one of: `jwt_issued`, `jwt_validated`, `validation_failed`, `token_revoked`, `family_revoked`, `delegation`, `version_bump`, `version_mismatch` — no other values allowed
- [ ] **Log buffer does not overflow**: Given a burst of 10,000 log entries in 1 second, assert the bounded buffer handles overflow gracefully (drops oldest or waits) without blocking the request handler

### Integration Tests (BDD-style with `rstest_bdd`)

- [ ] **Scenario: Full login flow triggers all expected audit events**: `given` a user logs in → `when` the login completes → `then` the following events are logged in order: `jwt_issued` (INFO) with `token_version` → verify event count = 1
- [ ] **Scenario: JWT validation success path**: `given` a request with a valid JWT arrives → `when` the JWT middleware validates it → `then` a `jwt_validated` log entry is written at DEBUG level with `decision_source` matching the route category
- [ ] **Scenario: JWT validation failure triggers security log**: `given` a request with an expired JWT arrives → `when` the JWT middleware validates it → `then` a `validation_failed` log entry is written at WARN level with `error: "token_expired"` and the `reason` field
- [ ] **Scenario: Version bump triggers audit log**: `given` a role change occurs → `when` the version is bumped in Redis → `then` a `version_bump` log entry is written at INFO level with `old_ver`, `new_ver`, and the `reason` for the change
- [ ] **Scenario: Delegation event is logged with actor details**: `given` a support agent impersonates a user → `when` the delegation token is issued → `then` a `delegation` log entry is written with `actor_id`, `delegation_type: "support_impersonation"`, `actor_roles`, and `act_claim_present: true`
- [ ] **Scenario: Token revocation is logged**: `given` a token is revoked (jti denylisted) → `when` the revocation completes → `then` a `token_revoked` log entry is written with the `jti`, `user_id`, and `reason` for revocation
- [ ] **Scenario: All 6 services emit security audit logs**: `given` security events occur across all 6 services → `when` the events are processed → `then` each service writes its own log entries with the correct `service` field matching the service name
- [ ] **Scenario: Log buffer overflow during burst**: `given` 10,000 JWT validations arrive in 1 second → `when` the logs are processed → `then` all 10,000 entries are captured (none dropped), the buffer handled the burst, and request latency was not impacted

### Security Regression Tests

- [ ] **No PII leaks in audit logs under any circumstance**: Given a JWT contains `email: "alice@corp.com"` in the payload, and a validation failure occurs, assert the log entry does NOT contain `alice@corp.com` — only `user_id` is logged
- [ ] **Raw access token is never in audit logs**: Given a JWT validation fails, assert the raw access token string (the full `eyJ...` base64url content) is NOT written to any log entry — only metadata like `jti` and `user_id` are logged
- [ ] **Log entry cannot be forged by a client**: Assert that log entries are written server-side and cannot be influenced by client input — the `service`, `timestamp`, `decision_source`, and `event` fields are all set by the server, not the client
- [ ] **Denylisted jti cannot suppress audit log**: Given a revocation is triggered, assert the `token_revoked` log entry is written even if the token's jti is already in the denylist — the act of revocation itself must be logged regardless of state
- [ ] **High-volume logging does not hide security events**: Given 1 million DEBUG-level validation logs are generated in 1 hour, assert that WARN and ERROR entries are still written and visible — log level filtering does not suppress higher-priority events
- [ ] **Async log loss on service crash**: Given a service crashes while log entries are in the async buffer, assert that the buffer is flushed on graceful shutdown (if possible) or that the lost entries are acceptable (known limitation of async logging)

### Edge Cases

- [ ] **Log entry with null actor_id for non-delegated events**: Given a JWT issuance for a regular user (no delegation), assert the `actor_id` field is `null` in the log entry (not missing, not empty string)
- [ ] **Log entry with empty scopes array**: Given a JWT issued with no scopes, assert the `scopes` field is an empty array `[]` or empty string `""` (document which)
- [ ] **Log entry with very long user_id (100 chars)**: Given a user_id of 100 characters, assert the log entry is still valid JSON and no truncation occurs
- [ ] **Log entry with ISO 8601 timestamp in UTC**: Assert every log entry's `timestamp` is in UTC (ends with `Z` or `+00:00`) — never local time
- [ ] **Log entry when log aggregator is down**: Given the log aggregator is unreachable (connection refused), assert the log entry is buffered and not lost — the request handler is not blocked
- [ ] **Log entry with Unicode in user_id or tenant_id**: Given a user_id containing Unicode (e.g., `"usr_caf\u00e9"`), assert the JSON log entry is valid and the Unicode is preserved (not mangled)
- [ ] **Log entry when tenant_id is unknown**: Given a JWT validation with no `tenant_id` in the claims, assert the log entry contains `tenant_id: null` (not missing, not a default value)
- [ ] **Log entry when error message is very long**: Given a validation failure with a 5KB error message, assert the log entry is still valid JSON — the error field should be truncated to a reasonable length (e.g., 1KB max)

### Cleanup

- [ ] Audit log buffer must be flushed between test scenarios — use a flush method or await buffer drain to ensure all entries are written before verifying log content
- [ ] Metrics registry must be reset between test scenarios using `prometheus::Registry::new()` to prevent cross-test metric contamination
- [ ] Log level configuration must be explicit per test — do not rely on global log level state; set the log level for each test explicitly (e.g., `tracing_subscriber::fmt::layer().with_max_level(Level::DEBUG)`)
- [ ] No audit log files should be left in the filesystem after tests — use an in-memory log writer (e.g., `tracing_appender::non_blocking` to a buffer) during tests
- [ ] If tests use a real log aggregator endpoint, isolate it per test — use different log streams or indices to prevent cross-test entry contamination
- [ ] Async log task must be cleaned up between tests — ensure spawned log writer tasks are dropped or cancelled to prevent them from writing to subsequent tests
