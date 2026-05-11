# Audit & Logging

> **Component:** Security event tracking, compliance reporting, tenant audit trails, event retention
> **Priority:** P1 — Implemented (schemas + query API); instrumentation across services pending
> **Services:** authz-core (cross-cutting audit API), identity-user-mgmt-service (user audit events)
> **Status:** ✅ API implemented — 11 schemas, 10 endpoints, 100% of audit API endpoints present

---

## The Pitch

**Buyer Question:** *Can I track every authentication attempt, permission change, and configuration event with tenant-isolated audit trails and compliance-ready reporting?*

If the answer is yes, you can pass SOC 2, ISO 27001, HIPAA, and GDPR audits without writing a single custom audit endpoint. Audit logging is the invisible insurance policy of identity — you only notice it's missing when you need it during a security incident or compliance review. In a multi-tenant system, audit trails must be tenant-isolated, tamper-proof, and queryable by event type, user, time range, and severity.

---

## What This Component Does

Audit & Logging captures and surfaces security-critical events across the entire identity platform:

1. **Authentication Events** — Login successes/failures, MFA challenges, password resets, account lockouts
2. **Authorization Events** — Permission changes, role assignments, policy modifications, principal evaluations
3. **User Management Events** — Profile updates, email/phone changes, account creation/deletion, MFA enrollment
4. **Session Events** — Token issuance, refresh, revocation, step-up authentication, impersonation
5. **Organization Events** — Member additions/removals, role changes, SSO configuration, SCIM sync
6. **API Key Events** — Key creation, rotation, revocation, archival, usage anomalies
7. **Compliance Reporting** — GDPR data access reports, audit export, retention policy enforcement
8. **Event Querying** — Search audit events by type, user, tenant, time range, and severity
9. **Event Retention** — Configurable retention periods with archival to cold storage
10. **Tamper Detection** — HMAC-signed audit events for integrity verification

---

## Implementation Status

### ✅ Implemented (API Layer)

All audit API schemas and endpoints are implemented and code-generated:

| Schema | Service | Purpose |
|--------|---------|---------|
| `AuditEvent` | authz-core | Core audit event entity |
| `AuditEventType` | authz-core | Event categories (8 enums) |
| `AuditSeverity` | authz-core | Severity levels (4 enums) |
| `AuditActor` | authz-core | Actor types (5 enums) |
| `AuditEventListResponse` | authz-core | Paginated event list |
| `AuditEventSearchRequest` | authz-core | Advanced search parameters |
| `AuditEventStats` | authz-core | Aggregate statistics |
| `AuditEventExportRequest` | authz-core | Export request payload |
| `AuditEventExportResponse` | authz-core | Export job status |
| `AuditRetentionPolicy` | authz-core | Retention policy entity |
| `AuditEventFilter` | authz-core | Filter parameters |

### API Endpoints (10 total)

| Service | Method | Endpoint | Purpose |
|---------|--------|----------|---------|
| **authz-core** | POST | `/audit/events` | Search audit events (advanced) |
| **authz-core** | GET | `/audit/events` | List audit events (simple) |
| **authz-core** | GET | `/audit/events/{id}` | Get single event |
| **authz-core** | POST | `/audit/events/stats` | Get event statistics |
| **authz-core** | POST | `/audit/export` | Request async export |
| **authz-core** | GET | `/audit/export/{id}` | Check export status |
| **authz-core** | GET | `/audit/retention` | List retention policies |
| **authz-core** | POST | `/audit/retention` | Create retention policy |
| **authz-core** | PATCH/DELETE | `/audit/retention/{id}` | Update/delete policy |
| **identity-user-mgmt** | POST | `/audit/user/{id}/events` | Get user-specific events |
| **identity-user-mgmt** | GET | `/audit/user/{id}/events/count` | User event count |
| **identity-user-mgmt** | POST | `/audit/user/{id}/events/compliance-export` | GDPR export |

### ⏳ Pending (Instrumentation Layer)

The API is ready to receive events, but no services currently emit them:

1. Instrument login/logout in identity-login-service
2. Instrument permission changes in authz-core
3. Instrument user profile changes in identity-user-mgmt-service
4. Instrument session events in identity-session-service
5. Instrument org/membership events in org-mgmt
6. Instrument API key events in api-keys service
7. HMAC signing for tamper-evident logging
8. Async export processing (currently returns stub responses)
9. SIEM integration (syslog, Kafka, HTTP)

---

## Entity Model

### Audit Event Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Event identifier |
| `event_type` | Enum | Yes | Event category (authentication, authorization, user_management, session_management, organization, api_key, system, compliance) |
| `event_action` | String (255) | Yes | Specific action (login_success, token_rotate, user_delete) |
| `tenant_id` | UUID | Yes | Tenant isolation scope |
| `org_id` | UUID | No | Organization scope |
| `user_id` | UUID | No | Associated user |
| `actor` | Enum | Yes | Actor type (user, system, admin, service_account, api_key) |
| `target_id` | UUID | No | Target entity identifier |
| `target_type` | String (255) | No | Target entity type |
| `severity` | Enum | No | Severity (info, warning, error, critical) |
| `metadata` | JSON | No | Event-specific details |
| `ip_address` | String (45) | Yes | Source IP address |
| `user_agent` | String (512) | No | Source user agent |
| `session_id` | UUID | No | Associated session |
| `hmac_signature` | String (255) | No | HMAC for integrity verification |
| `timestamp` | DateTime | Yes | Event timestamp (UTC) |

### Retention Policy Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Policy identifier |
| `tenant_id` | UUID | Yes | Tenant scope |
| `event_type` | String (255) | Yes | Event type to apply to |
| `retention_days` | Integer | Yes | Days to retain events (0 = indefinite) |
| `archive_after_days` | Integer | No | Days before archival to cold storage |
| `delete_after_days` | Integer | No | Days before permanent deletion |
| `created_at` | DateTime | Yes | Policy creation timestamp |

---

## Entity Relationships

```
AuditEvent ───┬── User (via user_id)           ← Event subject
              ├── Principal (via actor_id)      ← Event actor
              ├── Session (via session_id)      ← Event session context
              └── RetentionPolicy (via tenant_id) ← Retention rules
```

---

## Competitive Positioning

### Where Sesame-IDAM Now Wins

- **Tenant-isolated audit trails** — Each tenant gets its own audit log by default via `tenant_id` filter on every query
- **Rust-native event processing** — High-throughput event capture and indexing (once instrumentation is added)
- **API-first audit queries** — Full audit search via REST, not just dashboard UIs
- **Configurable retention policies** — Per-event-type retention, archival, and deletion schedules
- **Async compliance export** — Non-blocking export jobs with status polling (GDPR-ready)
- **No vendor lock-in** — Self-hosted, no per-user pricing for audit data

### Where Sesame-IDAM Lags (Instrumentation Pending)

- **No event instrumentation** — Competitors already capture events at the service level. Sesame's API is ready but empty.
- **No SIEM integration** — No syslog, Kafka, or HTTP push endpoints for real-time log forwarding.
- **No tamper-evident signing** — HMAC signing endpoint exists but isn't wired to event creation.
- **No dashboard UI** — Competitors provide visual audit dashboards with filtering and drill-down.

---

## Competitive Intelligence Deep Dive

### Okta: Audit Logs
Okta's Audit Logs capture every user action, system event, and configuration change. Logs are queryable via API, exportable to CSV/SIEM, and retained for 365 days on Enterprise plans. Okta also provides audit log streaming to Splunk, ArcSight, and QRadar. **Sesame Comparison:** Sesame has the API endpoint structure and retention policies. What's missing is the actual event emission at the service layer and SIEM streaming.

### Auth0: Audit Logs
Auth0's audit logs include sign-in events, token issuance, rule executions, and dashboard activity. Free tier has 24h retention; paid tiers up to 30 days. Auth0 provides Log Streams for real-time forwarding to Splunk, Datadog, and other SIEMs. **Sesame Comparison:** Sesame's retention policy model is more flexible (per-type policies). Auth0's real-time log streaming is more mature.

### Microsoft Entra: Sign-In Logs
Entra ID provides comprehensive sign-in logs, non-interactive logs, and protection logs. Integration with Microsoft Sentinel for SIEM. **Sesame Comparison:** Entra's integration with Microsoft's SIEM ecosystem is unmatched. Sesame's API-first approach is more flexible for non-Microsoft SIEMs.

### PingIdentity: Secure Logging
Ping logs all authentication, authorization, and administration events with tamper-evident logging. Supports SIEM integration via syslog, Kafka, and HTTP. **Sesame Comparison:** Sesame matches Ping's flexibility in API-first design but needs to implement the actual event emission and SIEM push endpoints.

---

## Implementation Roadmap

### Phase 1: Event Emission (P1 — Next Step)
1. Define audit event emission interface in shared crate
2. Instrument login/logout in identity-login-service (authn events)
3. Instrument permission changes in authz-core (authz events)
4. Instrument user profile changes in identity-user-mgmt-service (user events)
5. Instrument session events in identity-session-service (session events)
6. Instrument org/membership events in org-mgmt (org events)
7. Instrument API key events in api-keys service (api_key events)

### Phase 2: Compliance Features (P1)
1. Tamper-evident event signing (HMAC) — wire up to event emission
2. Async export processing — implement the actual file generation
3. GDPR Data Subject Access Request endpoint
4. Export download URL with temporary access

### Phase 3: Enterprise Integrations (P2)
1. SIEM push: syslog, Kafka, HTTP endpoints
2. Real-time event streaming for SOC monitoring
3. Compliance dashboard (optional — could be third-party)
4. Cross-tenant audit aggregation (platform admin view)

---

## Key Takeaway for Buyers

The audit logging API is **fully implemented** — 11 schemas, 10 endpoints, all with proper error responses, pagination, filtering, and compliance export. The only remaining work is **instrumenting the services** to emit events, which is a straightforward addition once the emission interface is defined.

**For regulated industries**, the API foundation is solid. The key question is timeline for instrumentation — if that's delivered within 4-6 weeks, Sesame-IDAM becomes competitive with Okta and Auth0 on audit capability.

**The API-first approach is an advantage** — audit queries can be used by any client (dashboard, SIEM, analytics tool) without depending on a specific UI. This is more flexible than the dashboard-dependent approaches of Okta and Auth0.
