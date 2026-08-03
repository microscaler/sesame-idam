# Epic 15: Language-Neutral Sesame Consumer Contract

> **Status:** In progress  
> **Program:** Standards-first OIDC provider  
> **Audit source:** [Non-BRRTRouter framework readiness audit](../../audit/non-brrtrouter-framework-readiness-2026-07-25.md)  
> **Dependencies:** Epics 11–14; tenant-consumer API; ADR-005 RLS contract  
> **Canonical entry:** [provider-profile-v1.md](../../standards-first-oidc/provider-profile-v1.md)

## Outcome

Sesame publishes one versioned, executable, language-neutral contract from which
all supported client libraries, framework presets, examples, and the existing
Rust client derive.

The contract separates:

1. OIDC relying-party login;
2. resource-server token validation and policy context;
3. tenant/admin API operations.

No language client becomes the semantic source of truth.

## Why this epic exists

Current integration knowledge is distributed across:

- six service OpenAPI documents;
- the tenant-consumer draft OpenAPI;
- runtime JWT types;
- stale root README examples;
- partially verified wiki pages;
- BRRTRouter security configuration;
- `sesame-idam-client`;
- product-specific consumer examples outside the contract.

This has produced drift in algorithm, claims, optional `org_id`, token response
fields, endpoint URLs, refresh behavior, and the RLS integration story.

Client libraries cannot be maintained safely until those semantics are
centralized and versioned.

## Scope

- normative provider profile documentation;
- canonical access-token and ID-token schemas;
- normalized verified-principal model;
- stable public tenant API OpenAPI;
- structured error model;
- pagination, idempotency, retry, and rate-limit semantics;
- browser/BFF/API/worker threat-model guidance;
- conformance fixture packaging;
- compatibility and versioning policy;
- reconciliation of the Rust client and existing docs.

## Non-goals

- implementing framework libraries (Epic 16);
- exposing internal service OpenAPI as the public SDK;
- requiring every language to expose identical idioms;
- making RLS mandatory for non-PostgreSQL consumers.

## Stories

| Story | Title | Result |
|---|---|---|
| 15.1 | Normative OIDC and token profile | One provider document with exact supported behavior and validation rules |
| 15.2 | Canonical verified principal | Stable tenant/user/session/org/role/permission model after token validation |
| 15.3 | Public tenant API OpenAPI | One externally supported API document independent of service topology |
| 15.4 | Stable errors and transport policy | Machine-readable errors, redaction, retries, idempotency, pagination, and rate limits |
| 15.5 | Client product boundaries | Separate relying-party, resource-server, and tenant/admin client responsibilities |
| 15.6 | Conformance fixture distribution | Versioned fixtures consumable from every client repository |
| 15.7 | Documentation and quickstart contract | Executable browser, BFF, API, worker, org, and RLS journeys |
| 15.8 | Rust client reconciliation | Existing BRRTRouter/may client aligns with the canonical contract |
| 15.9 | Contract versioning and deprecation | Compatibility rules for claims, APIs, metadata, and SDK releases |

## Canonical artifacts

### Provider profile

Defines:

- issuer/discovery contract;
- supported grants, response types, scopes, claims, and algorithms;
- registered-client types and authentication;
- authorization, token, refresh, UserInfo, and logout behavior;
- access-token and ID-token validation;
- key rotation and caching;
- tenant/application/organization semantics;
- errors and security requirements.

### Verified principal

Framework adapters should normalize validated claims into a semantic model
equivalent to:

- tenant ID;
- subject/user ID;
- client/application/portal ID;
- session ID and token version;
- optional active organization ID;
- user type;
- roles and permissions;
- entitlement reference/hash when present;
- actor/delegation context when supported.

The model is created only after framework-native cryptographic validation.
Decoded-but-unverified JWT JSON is never accepted.

### Public tenant API

The public OpenAPI must:

- describe public routes, not internal microservice placement;
- use public host/path semantics;
- identify public versus bearer/client-authenticated operations;
- remove caller-selected tenancy where credentials already identify tenant;
- contain complete request, response, and error schemas;
- define pagination and idempotency;
- exclude platform/internal/admin-only operations;
- be validated against live public routing.

## Required documentation correction

The following claims must be reconciled with delivered behavior:

- RS256 versus EdDSA;
- flat roles/permissions versus `https://sesame-idam.dev/claims`;
- required versus optional `org_id`;
- `SesameExecutor` versus Lifeguard contextual transactions;
- implemented versus target frontend SDK;
- implemented versus target hosted auth/OIDC;
- one public provider contract versus internal service URLs;
- exact endpoint and implementation status.

Design targets remain valuable, but they must be labelled as targets rather
than current integration instructions.

## Client product boundaries

### Relying-party package

May provide:

- a named framework provider/preset;
- issuer/client defaults;
- claim and profile mapping;
- refresh/logout integration;
- organization-switching helpers.

Must delegate protocol validation to the ecosystem's maintained OIDC library.

### Resource-server package

May provide:

- strict Sesame JWT validation defaults;
- verified-principal mapping;
- route/policy helpers;
- optional framework-native RLS/context integration.

Must not combine browser session handling with API token validation.

### Tenant/admin API package

May provide:

- typed public API operations;
- user-token and service-token clients;
- pagination, retry, idempotency, and error handling.

Must not implicitly elevate an end-user token into administrative authority.

## Acceptance gate

- [x] One normative provider profile replaces contradictory integration docs.
- [x] Access-token, ID-token, and verified-principal schemas are versioned.
- [x] Optional pre-organization state is representable.
- [x] One public tenant API OpenAPI validates against live routes.
- [x] Internal service hostnames are absent from consumer quickstarts.
- [x] Errors, pagination, retries, idempotency, and rate limits are defined.
- [x] Shared conformance fixtures are published as a versioned artifact.
- [x] Rust and at least one non-Rust proof consumer use the same contract.
- [x] Documentation snippets execute in CI.
- [x] Deprecation and compatibility policies are published.

## Evidence

| Item | Location |
|---|---|
| Provider profile entry | `docs/standards-first-oidc/provider-profile-v1.md` |
| Principal mapping + tests | `verified-principal-mapping-v1.md`; `sesame-common` `verified_principal`; tooling `verified_principal.py` |
| Transport policy | `docs/standards-first-oidc/transport-policy-v1.md` |
| Public OpenAPI | `openapi/idam/tenant-consumer/openapi.yaml` |
| Live contract BDD | `tenant_consumer_live_contract.rs` |
| Fixture package | `conformance/oidc-v1/{VERSION,CHECKSUM,README.md}` |
| Contract sync | `python -m sesame_idam_tooling.contract_sync` |
| Authlib proof | `python -m sesame_idam_tooling.authlib_contract_proof` |
| Rust client sync | `sesame-idam-client` `tests/contract_sync.rs` |
| Compatibility | `docs/standards-first-oidc/compatibility-v1.md` |
| Quickstarts | `docs/standards-first-oidc/quickstarts/` |

## Versioning rules

- provider metadata and token-profile changes require compatibility review;
- removing or changing a required claim is breaking;
- additive optional claims are non-breaking only when parsers are required to
  ignore unknown fields;
- public API schema changes follow semantic versioning and OpenAPI diff gates;
- conformance fixtures version with the provider profile;
- every client release states supported profile/API versions;
- deprecated behavior has a migration document and removal window.

## Exit condition

Epic 15 is complete when a client team can implement a new language integration
from the provider profile, public OpenAPI, and fixture bundle without reading
Sesame Rust code, BRRTRouter code, Kubernetes configuration, or product-specific
application code.
