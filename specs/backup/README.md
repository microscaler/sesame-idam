# Backup: Original Sesame OpenAPI spec

**File:** `openapi-original-sesame.yaml`

This is a backup of the **original** seasame-idam OpenAPI spec (pre-pivot, single “Sesame Auth Service” monolith). It is kept for reference when implementing the rich functionality listed in the main README **Features** table.

The current architecture splits IDAM into **Authentication** and **Authorization** microservices; canonical specs for those are in `BRRTRouter/docs/SPIFFY_mTLS/openapi/` (`identity-openapi.yaml`, `access-management-openapi.yaml`). When building out the Authentication and Authorization microservices, use this backup to:

- Reuse or adapt paths, schemas, and behaviours (login methods, MFA, sessions, RBAC, API keys, SSO, SCIM, audit, metrics, webhooks, etc.).
- Ensure the new implementation provides at least the functionality in the Features table, and extend it (e.g. tenant/org model, Client Credentials, Token Exchange, no PII in URIs) per the Generic Identity and Access Management designs.

Do not edit this backup; treat it as read-only reference. The live spec for day-to-day work remains `specs/openapi.yaml` (or the per-microservice specs under `openapi/idam/` once they are added).
