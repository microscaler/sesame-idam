# Sesame-IDAM LLM Wiki — Index

## Core

- [README](./README.md) — Wiki entry point
- [SCHEMA.md](./SCHEMA.md) — Conventions and page format
- [log.md](./log.md) — Session log

## Docs Catalog

- [docs-catalog.md](./docs-catalog.md) — Inventory of design docs and merge status

## Entities

Data structures and database entities across all 6 microservices.

| Page | Service | Status |
|------|---------|--------|
| [entity-user](./entities/entity-user.md) | identity-login-service | verified |
| [entity-organization](./entities/entity-organization.md) | org-mgmt | verified |
| [entity-session](./entities/entity-session.md) | identity-session-service | verified |
| [entity-api-key](./entities/entity-api-key.md) | api-keys | verified |
| [entity-role](./entities/entity-role.md) | org-mgmt | verified |
| [entity-permission](./entities/entity-permission.md) | org-mgmt | verified |
| [entity-application](./entities/entity-application.md) | org-mgmt | verified |
| [entity-mfa-device](./entities/entity-mfa-device.md) | identity-user-mgmt-service | verified |
| [entity-audit-log](./entities/entity-audit-log.md) | all services | verified |
| [entity-webhook](./entities/entity-webhook.md) | org-mgmt | verified |
| [entity-email-verification](./entities/entity-email-verification.md) | identity-user-mgmt-service | verified |
| [entity-social-account](./entities/entity-social-account.md) | identity-user-mgmt-service | verified |
| [entity-employee](./entities/entity-employee.md) | identity-user-mgmt-service | verified |
| [entity-scim-user](./entities/entity-scim-user.md) | org-mgmt | verified |
| [entity-org-domain](./entities/entity-org-domain.md) | org-mgmt | verified |
| [entity-org-invite](./entities/entity-org-invite.md) | org-mgmt | verified |
| [entity-org-membership](./entities/entity-org-membership.md) | org-mgmt | verified |

## Topics

Architectural concepts, workflows, and cross-cutting concerns.

||| Page | Description |
|||------|-------------|
||| [topic-architecture-overview](./topics/topic-architecture-overview.md) | Six-service split rationale, service map, 133 endpoints, 12 workspace crates. `cargo check --workspace` passes with 0 errors. |
||| [topic-package-naming-convention](./topics/topic-package-naming-convention.md) | Gen/impl package naming mismatch that breaks `brrtrouter client build` — current vs target |
||| [topic-build-infrastructure](./topics/topic-build-infrastructure.md) | Missing build.rs, config/service.yaml, services layer, tests, seeds |
||| [topic-tiltfile-architecture](./topics/topic-tiltfile-architecture.md) | Tiltfile is broken — rewrite plan based on hauliage pattern, infra wiring |
||| [topic-tooling-architecture](./topics/topic-tooling-architecture.md) | sesame-idam CLI shim, brrtrouter_tooling delegation map, justfile recipes |
||| [topic-remediation-plan](./topics/topic-remediation-plan.md) | 5-phase remediation plan (naming fix → build infra → Tiltfile → workspace cleanup → validation) |
||| [topic-hybrid-authz](./topics/topic-hybrid-authz.md) | Hybrid authorization model: JWT claims for common path, selective online fallback. Route classification (jwt-only, jwt-with-fallback, online-only), JWT middleware, route-specific decisions (Story 4.4), selective fallback caching (Story 4.3), RFC 7662 introspection (Story 4.5). |
||| [topic-tenancy-model](./topics/topic-tenancy-model.md) | Hard-segment multi-tenant model, X-Tenant-ID, isolation guarantees |
||| [topic-openapi-tenancy-strategy](./topics/topic-openapi-tenancy-strategy.md) | Global spec + middleware injection pattern, why not per-tenant specs |
||| [topic-jwt-schema](./topics/topic-jwt-schema.md) | JWT enrichment claims, coarse vs fine-grained auth |
||| [topic-login-flow](./topics/topic-login-flow.md) | User login flow: login → authz-core → JWT |
||| [topic-authorization-flow](./topics/topic-authorization-flow.md) | Per-request authorization: Redis cache, role evaluation |
||| [topic-api-key-validation](./topics/topic-api-key-validation.md) | M2M key validation flow |
||| [topic-rls-bridge](./topics/topic-rls-bridge.md) | RLS helpers, session injection, database security |
|||| [topic-brrtrouter-codegen](./topics/topic-brrtrouter-codegen.md) | OpenAPI → codegen workflow, gen/ vs impl/, package naming warning |
|||| [topic-jsf-linting](./topics/jsf-linting.md) | JSF-inspired clippy profile: pedantic mode, `-D warnings`, JSF-aligned thresholds in `clippy.toml`, Phase 1 warn → Phase 2 deny plan |
|||| [topic-data-model](./topics/topic-data-model.md) | Full ERD, key design decisions |
|||| [topic-entity-relationship-diagram](./topics/topic-entity-relationship-diagram.md) | Comprehensive ERD reconciled from OpenAPI specs + impl model verification |
||| [topic-scaling-profiles](./topics/topic-scaling-profiles.md) | Per-service scaling, cache strategies |
||| [topic-openapi-convention](./topics/topic-openapi-convention.md) | Spec layout, schema duplication convention |
||| [topic-inter-service-deps](./topics/topic-inter-service-deps.md) | Only dependency: login → authz-core at login |
||| [topic-two-user-types](./topics/topic-two-user-types.md) | customer vs platform user model |
||| [topic-org-personas](./topics/topic-org-personas.md) | Platform, provider, consumer org types |
|||| [topic-developer-contract](./topics/topic-developer-contract.md) | 3-layer SDK, Admin API, RLS helpers |
|||| [topic-token-versioning](./topics/topic-token-versioning.md) | ver/sid claims, Redis version storage, validation flow, TTL strategy, version bump on authz change, version mismatch handling (Story 5.1-5.5) |
|||| [topic-delegation](./topics/topic-delegation.md) | RFC 8693 token exchange, act claim structure, delegation chain, actor can_delegate logic, support impersonation flow, step-up MFA, mfa_type strength (Story 6.1-6.3) |
|||| [topic-mfa](./topics/topic-mfa.md) | sx.mfa_verified claim, step-up MFA flow, mfa_type strength table, F-006 refresh token invalidation, F-016 SMS restriction (Story 6.3) |

## Audit

OpenAPI spec quality audits and remediation logs.

|| Page | Description |
||------|-------------|
|| [security_evaluation_001](../audit/security_evaluation_001.md) | Comprehensive API design failure audit across all 6 specs (2026-05-09). 90 error responses, pagination, SCIM, MCP, TokenResponse standardization. All specs now pass brrtrouter-gen lint. |

## Reference

External integrations, API surfaces, patterns.

|| Page | Description |
||------|-------------|
|| [ref-api-surface](./reference/ref-api-surface.md) | Complete API surface across 6 services (119 endpoints, 26 tags) |
|| [ref-propelauth-comparison](./reference/ref-propelauth-comparison.md) | PropelAuth vs Supabase vs Sesame benchmark |
|| [ref-frontend-sdk](./reference/ref-frontend-sdk.md) | Frontend SDK integration pattern |
|| [ref-backend-admin-api](./reference/ref-backend-admin-api.md) | Backend Admin API contract |
