# Audit & Logging

> **Component:** Security event tracking, compliance reporting, tenant audit trails
> **Priority:** P2 — Required for regulated industries (healthcare, finance, government)
> **Services:** All 6 services (audit events generated across authn, authz, user-mgmt, session, org)

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

## Entity Model

### Audit Event Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Event identifier |
| `event_type` | Enum: [authn, authz, user, session, org, api_key] | Yes | Event category |
| `event_action` | String (255) | Yes | Specific action (login, revoke, create) |
| `tenant_id` | UUID | Yes | Tenant isolation scope |
| `org_id` | UUID | No | Organization scope |
| `user_id` | UUID | No | Associated user |
| `actor_id` | UUID | No | Principal performing the action |
| `actor_type` | Enum: [user, system, admin] | No | Actor type |
| `target_id` | UUID | No | Target entity identifier |
| `target_type` | String (255) | No | Target entity type |
| `severity` | Enum: [info, warning, error, critical] | No | Event severity |
| `metadata` | JSON | No | Event-specific details |
| `ip_address` | String (45) | No | Source IP address |
| `user_agent` | String (512) | No | Source user agent |
| `session_id` | UUID | No | Associated session |
| `hmac_signature` | String (255) | No | HMAC for integrity verification |
| `created_at` | DateTime | Yes | Event timestamp (UTC) |

### Compliance Report Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Report identifier |
| `tenant_id` | UUID | Yes | Tenant scope |
| `report_type` | Enum: [gdpr_access, audit_export, retention] | Yes | Report category |
| `status` | Enum: [pending, generating, complete, failed] | Yes | Report status |
| `generated_at` | DateTime | Yes | Generation timestamp |
| `expires_at` | DateTime | Yes | Report download expiration |
| `download_url` | String (1024) | No | Secure download URL |
| `event_count` | Integer | No | Number of events included |
| `created_by` | UUID | No | Requesting principal |

### Retention Policy Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Policy identifier |
| `tenant_id` | UUID | Yes | Tenant scope |
| `event_type` | String (255) | Yes | Event type to apply to |
| `retention_days` | Integer | Yes | Days to retain events |
| `archive_after_days` | Integer | No | Days before archival to cold storage |
| `delete_after_days` | Integer | No | Days before permanent deletion |
| `created_at` | DateTime | Yes | Policy creation timestamp |

---

## Entity Relationships

```
AuditEvent ───┬── User (via user_id)           ← Event subject
              ├── Principal (via actor_id)      ← Event actor
              ├── Session (via session_id)      ← Event session context
              └── ComplianceReport (via tenant_id) ← Audit report output
```

---

## Required API Endpoints

### Event Querying

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/audit/events` | List audit events with filters |
| `GET` | `/api/v1/audit/events/{id}` | Get specific audit event |
| `POST` | `/api/v1/audit/events/search` | Advanced audit event search |
| `GET` | `/api/v1/audit/events/stats` | Get event statistics |

### Event Filtering

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/audit/events/by-user` | Events by user ID |
| `GET` | `/api/v1/audit/events/by-tenant` | Events by tenant ID |
| `GET` | `/api/v1/audit/events/by-org` | Events by organization |
| `GET` | `/api/v1/audit/events/by-type` | Events by event type |
| `GET` | `/api/v1/audit/events/by-date` | Events by date range |
| `GET` | `/api/v1/audit/events/by-severity` | Events by severity level |

### Compliance Reporting

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/audit/reports/gdpr-access` | Generate GDPR data access report |
| `POST` | `/api/v1/audit/reports/audit-export` | Generate audit event export |
| `GET` | `/api/v1/audit/reports/{id}` | Get report status and download |
| `DELETE` | `/api/v1/audit/reports/{id}` | Cancel report generation |

### Retention Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/audit/retention` | List retention policies |
| `POST` | `/api/v1/audit/retention` | Create retention policy |
| `POST` | `/api/v1/audit/retention/{id}` | Update retention policy |
| `DELETE` | `/api/v1/audit/retention/{id}` | Delete retention policy |

---

## Competitive Positioning

### Where Sesame-IDAM Lags
- **No built-in audit logging** — This is the biggest gap. Currently, no audit event capture exists in any service.
- **No compliance reports** — No GDPR access reports or audit exports.
- **No event retention policies** — No automatic event archival or deletion.
- **No tamper detection** — No HMAC signing for audit event integrity.

### Future Competitive Advantages
- **Tenant-isolated audit trails** — Each tenant gets its own audit log by default.
- **Rust-native event processing** — High-throughput event capture and indexing.
- **API-first audit queries** — Full audit search via REST, not just dashboard UIs.

---

## Competitive Intelligence Deep Dive

### Okta: Audit Logs
Okta's Audit Logs capture every user action, system event, and configuration change. Logs are queryable via API, exportable to CSV/SIEM, and retained for 365 days on Enterprise plans. **Sesame Gap:** Zero audit logging currently implemented.

### Auth0: Audit Logs
Auth0's audit logs include sign-in events, token issuance, rule executions, and dashboard activity. Free tier has 24h retention; paid tiers up to 30 days. **Sesame Gap:** No event capture at all currently.

### Microsoft Entra: Sign-In Logs
Entra ID provides comprehensive sign-in logs, non-interactive logs, and protection logs. Integration with Microsoft Sentinel for SIEM. **Sesame Gap:** No SIEM integration points.

### PingIdentity: Secure Logging
Ping logs all authentication, authorization, and administration events with tamper-evident logging. Supports SIEM integration via syslog, Kafka, and HTTP. **Sesame Gap:** No logging infrastructure exists.

---

## Implementation Roadmap

### Phase 1: Core Audit Events (Not Implemented) — P1
1. Define AuditEvent schema across all 6 services
2. Instrument login/logout events in identity-login-service
3. Instrument permission changes in authz-core
4. Instrument user profile changes in identity-user-mgmt-service
5. Instrument session events in identity-session-service
6. Instrument org/membership events in org-mgmt

### Phase 2: Audit Querying (Not Implemented) — P2
1. Audit event search API with filters
2. Event statistics and analytics
3. Event export (CSV/JSON)
4. Retention policy management

### Phase 3: Compliance (Not Implemented) — P2
1. GDPR data access report generation
2. Tamper-evident event signing (HMAC)
3. SIEM integration (syslog, Kafka, HTTP)
4. Compliance-ready audit exports (SOC 2, ISO 27001)

---

## Key Takeaway for Buyers

Audit logging is the **biggest single gap** in Sesame-IDAM's current feature set. No competitor can be seriously evaluated for regulated industries without audit trails. This should be **P1 priority** — implement basic event capture across all services within 4-6 weeks.

**For startups and SMBs** that don't need compliance reporting, this is a lower priority. **For healthcare, finance, and government buyers**, lack of audit logging is a disqualifier.
