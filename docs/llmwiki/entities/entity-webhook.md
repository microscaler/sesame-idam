---
title: Webhook Entity
status: partially-verified
updated: 2026-01-22
sources: [openapi/org-mgmt/openapi.yaml]
---

# Entity: Webhook

Owned by: **org-mgmt**

## Description

Webhook endpoints per organization for real-time event delivery. Includes delivery tracking with retry logic.

## Schema: WebhookEndpoint

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| org_id | uuid (FK) | |
| url | text | Callback URL |
| secret | text | HMAC signing key |
| events | text[] | Event types to subscribe to |
| is_active | boolean | |
| created_at | timestamptz | |
| updated_at | timestamptz | |

## Schema: WebhookDelivery

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| webhook_endpoint_id | uuid (FK) | |
| event_type | text | |
| payload | jsonb | |
| status | text | `pending` | `success` | `failed` |
| attempts | integer | Retry count |
| last_attempt_at | timestamptz | |
| next_retry_at | timestamptz | Exponential backoff |
| response_status | integer | HTTP status |
| response_body | text | Last response |
| created_at | timestamptz | |

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/` — Lifeguard entity definition
- `openapi/org-mgmt/openapi.yaml` — Webhook CRUD API

## Gaps / Drift

> **Open:** Verify actual Lifeguard model against OpenAPI spec.
