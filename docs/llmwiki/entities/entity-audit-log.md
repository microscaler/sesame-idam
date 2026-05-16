---
title: Audit Event Entity
status: verified
updated: 2026-05-16
sources: [openapi/*/openapi.yaml, microservices/*/impl/src/models/]
---

# Entity: Audit Event

Owned by: **authz-core** AND **identity-user-mgmt-service** (two separate impl models)

## Description

Immutable audit trail for all identity and access management operations. **Important:** There are TWO separate `audit_events` tables in the system — one in authz-core and one in identity-user-mgmt-service. They have similar but not identical schemas.

## Schema: authz-core/impl/src/models/audit_event.rs

|| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| tenant_id | varchar(255) | Tenant scope |
| event_type | varchar(64) | e.g., "AUTH_SUCCESS", "AUTH_FAIL" |
| severity | varchar(32) | INFO, WARN, ERROR, CRITICAL |
| actor | varchar(32) | Actor identifier |
| data | text (nullable) | Event payload |
| ip | varchar(64, nullable) | Source IP |
| created_at | timestamptz | |

## Schema: identity-user-mgmt-service/impl/src/models/audit_event.rs

|| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| tenant_id | varchar(255) | Tenant scope |
| user_id | uuid (FK -> users, nullable) | Who performed the action |
| event_type | varchar(64) | e.g., "user.login", "org.create" |
| severity | varchar(32) | |
| actor | varchar(32) | Actor identifier |
| data | text (nullable) | Event payload |
| ip | varchar(64, nullable) | Source IP |
| user_agent | varchar(255, nullable) | Client user agent |
| created_at | timestamptz | |

## Key Design Decisions

1. **Two separate audit event models.** authz-core uses a lightweight model (no user_id FK); identity-user-mgmt-service has a richer model with user_id FK, org_id, user_agent.
2. **No org-mgmt audit events.** The old wiki pointed to org-mgmt for audit — but authz-core is the actual home for audit events.
3. **Event types use dot notation.** Examples: "user.login", "org.create", "token.refresh".
4. **Severity levels.** INFO, WARN, ERROR, CRITICAL.
5. **Data stored as text.** The `data` field is text (JSON string), not jsonb.

## API Endpoints (Audit)

|| Service | Endpoint | Purpose |
|---------|----------|---------|
| authz-core | `GET /authz/audit/events` | List audit events |
| authz-core | `POST /authz/audit/events` | Search audit events |
| authz-core | `GET /authz/audit/events/{id}` | Get audit event by ID |
| authz-core | `POST /authz/audit/events/stats` | Get audit event statistics |
| authz-core | `POST /authz/audit/export` | Export audit events |
| authz-core | `GET /authz/audit/export/{export_id}` | Check export status |
| authz-core | `GET /authz/audit/retention` | List retention policies |
| authz-core | `POST /authz/audit/retention` | Create retention policy |
| authz-core | `PATCH /authz/audit/retention/{id}` | Update retention policy |
| authz-core | `DELETE /authz/audit/retention/{id}` | Delete retention policy |
| identity-user-mgmt-service | `POST /admin/audit/events` | Get user-specific audit events |
| identity-user-mgmt-service | `POST /admin/audit/users/{user_id}/events/compliance-export` | Export user's audit events (GDPR) |
| identity-user-mgmt-service | `GET /admin/audit/users/{user_id}/events/count` | Get user event count |

## Drift Found (verified 2026-05-16)

|| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|--------|
| Single audit table in org-mgmt | TWO audit tables: authz-core + identity-user-mgmt-service | Critical — wiki pointed to wrong service |
| `user_id` uuid (FK) | authz-core: NO user_id FK; identity-user-mgmt-service: has user_id FK | Medium — only one model has FK |
| `org_id` column | ONLY in identity-user-mgmt-service; NOT in authz-core | Medium |
| `action` column | Actual field is `event_type` | Low — naming mismatch |
| `resource_type` column | NOT in either impl | Medium — wiki overstates |
| `resource_id` column | NOT in either impl | Medium — wiki overstates |
| `metadata` jsonb | Actual field is `data` (text, not jsonb) | Medium |
| `ip_address` inet | Actual field is `ip` varchar(64) | Low |
| No `user_agent` in authz-core | Only in identity-user-mgmt-service | Medium |
| No `deleted_at` | Not applicable — audit events are immutable | N/A |
