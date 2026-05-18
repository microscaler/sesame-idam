---
title: Webhook Subscription Entity
status: verified
updated: 2026-05-16
sources: [openapi/org-mgmt/openapi.yaml, microservices/org-mgmt/impl/src/models/webhook_subscription.rs]
---

# Entity: Webhook Subscription

Owned by: **org-mgmt**

## Description

Webhook subscription model for organizations. Subscribes an organization to receive HTTP callbacks for events. No delivery tracking table exists — the impl uses a simplified model.

## Schema (from impl/ crate — org-mgmt)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| org_id | uuid (FK -> orgs) | Webhook belongs to an org |
| url | text | Callback URL |
| events | text | Event types (stored as text, NOT text[] array) |
| secret | text (nullable) | HMAC signing key |
| active | boolean | Whether the webhook is enabled |
| created_at | timestamptz | |
| updated_at | timestamptz | |

## API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/organizations/{org_id}/webhooks` | GET | List webhook subscriptions |
| `/organizations/{org_id}/webhooks/{subscription_id}` | DELETE | Delete webhook subscription |
| `/organizations/{org_id}/webhooks/{subscription_id}/test` | POST | Test webhook delivery |

## Key Design Decisions

1. **Single table.** The wiki previously documented two tables (`WebhookEndpoint` and `WebhookDelivery`), but the impl has only one: `webhook_subscriptions`.
2. **No delivery tracking.** The `WebhookDelivery` table does NOT exist. The impl stores `failed_deliveries`, `last_delivery_at`, `last_delivery_status`, `total_deliveries` as columns on the subscription itself — not as a separate table.
3. **Events as text.** The `events` column is VARCHAR (text), NOT a PostgreSQL `text[]` array.
4. **`active` not `is_active`.** The impl uses `active` boolean.

## Drift Found (verified 2026-05-16)

| Wiki Claim | Actual Impl | Impact |
|------------|-------------|--------|--------|
| Two tables (WebhookEndpoint + WebhookDelivery) | Single table: `webhook_subscriptions` | High — WebhookDelivery doesn't exist |
| `events` is `text[]` | `events` is VARCHAR/text | Medium — array type wrong |
| `is_active` | Actual column is `active` | Low — naming mismatch |
| `status`, `attempts`, `last_attempt_at`, `next_retry_at` | NOT in impl (no delivery tracking table) | High — delivery tracking is conceptual |
| `payload` jsonb | NOT in impl | High |
| `response_status`, `response_body` | NOT in impl | High |
| `subscription_id` as path param | Actual param is `subscription_id` (correct) | Low |
