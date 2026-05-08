---
title: Audit Log Entity
status: partially-verified
updated: 2026-01-22
sources: [openapi/org-mgmt/openapi.yaml]
---

# Entity: Audit Log

Owned by: **org-mgmt** (but logged by all services)

## Description

Immutable audit trail for all identity and access management operations.

## Schema (from OpenAPI)

|| Column | Type | Notes |
||--------|------|-------|
|| id | uuid (PK) | |
|| user_id | uuid | Who performed the action |
|| org_id | uuid | Which org was affected |
|| tenant_id | uuid (FK) | **REQUIRED** — audit logs scoped to platform |
|| action | text | e.g., "user.login", "org.create" |
|| resource_type | text | e.g., "user", "organization" |
|| resource_id | text | ID of the affected resource |
|| metadata | jsonb | Additional context |
|| ip_address | inet | Source IP |
|| user_agent | text | Client user agent |
|| created_at | timestamptz | |

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition
- `openapi/org-mgmt/openapi.yaml` — Audit log API (if exposed)

## Gaps / Drift

> **Open:** Verify actual Lifeguard model against OpenAPI spec.
