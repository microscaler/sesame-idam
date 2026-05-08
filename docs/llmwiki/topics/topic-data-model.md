---
title: Data Model
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, sesame-idam-complete.md]
---

# Data Model

## Core Entities

The data model consists of these primary entities:

### Identity Layer
- **User** — Single table, two types (customer/platform)
- **Session** — Per-user AND per-application, tokens hashed
- **MFADevice** — TOTP, WebAuthn, SMS support

### Access Management Layer
- **Organization** — Multi-tenant, per-platform scoping
- **Application** — Consuming app registration
- **Role** — Per-application, with inheritance (parent_role_id)
- **Permission** — Per-application, named (resource:action convention)
- **RolePermission** — Many-to-many bridge table

### M2M Layer
- **APIKey** — User-scoped or org-scoped, SHA-256 hash only

### Observability Layer
- **AuditLog** — All operations across all services
- **WebhookEndpoint** / **WebhookDelivery** — Event delivery with retry

## Key Design Decisions

1. **One user table, two user types.** `user_type` distinguishes customer vs platform.
2. **Soft deletes everywhere.** `deleted_at` columns on user, org, application.
3. **Sessions are per-user AND per-application.** Separate sessions per app per user.
4. **Refresh token rotation.** Old tokens revoked on each refresh.
5. **API keys per-application.** Specific set of permissions per app.
6. **Role inheritance.** `parent_role_id` creates hierarchy; effective permissions resolved by walking chain.
7. **Orgs per-platform.** Same org name can exist across different apps via `platform` column.

## ERD (from design-doc.md)

```
User 1──* OrganizationMember *──1 Organization
User 1──* UserRole *──1 Role
Role 1──* RolePermission *──1 Permission
User 1──* Session *──1 Application
User 1──* APIKey
User 1──* MFADevice
User 1──* AuditLog
Application 1──* WebhookEndpoint 1──* WebhookDelivery
```

## Code Anchors

- `docs/design-doc.md:330-500` — Full ERD and entity definitions
- `docs/sesame-idam-complete.md:152-362` — Data model section
- `microservices/idam/*/impl/src/models/` — Lifeguard entity definitions

## Gaps / Drift

> **Open:** Verify actual Lifeguard models in impl crates match the design doc. Need source code verification.
