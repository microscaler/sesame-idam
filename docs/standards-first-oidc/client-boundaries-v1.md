# Client product boundaries v1

Status: normative  
Profile version: `1.0.0`  
Related: [client-ecosystem-selection-v1.md](./client-ecosystem-selection-v1.md)

Sesame publishes three thin client surfaces. Semantic authority remains the
provider profile, verified-principal schema, tenant-consumer OpenAPI, and
conformance fixtures — never a language SDK.

Auth.js is the Epic 16 **reference** integration only. It is not the Epic 15
contract proof consumer (Authlib is).

## Relying-party (RP) package

**May provide**

- framework provider/preset wiring;
- issuer and client defaults from discovery;
- claim → verified-principal mapping after the ecosystem library validates tokens;
- refresh / logout helpers;
- organization-switch helpers that call public tenant API routes.

**Must**

- delegate protocol validation to the ecosystem’s maintained OIDC library;
- refuse decoded-but-unverified JWT JSON;
- treat pre-org `organization_id: null` as valid.

**Must not**

- re-implement signature verification with ad-hoc crypto;
- hard-code internal service hostnames;
- mix browser cookie sessions with API bearer validation in one module.

## Resource-server (RS) package

**May provide**

- strict Sesame JWT validation defaults (EdDSA, `typ=at+jwt`, issuer/audience);
- verified-principal mapping;
- route/policy helpers;
- optional Postgres RLS / Lifeguard contextual-transaction helpers (ADR-005).

**Must not**

- combine browser session handling with API token validation;
- trust flat top-level `roles` / `permissions` (use `sx` namespace);
- require RLS for non-Postgres consumers.

## Tenant / admin API package

**May provide**

- typed operations from `openapi/idam/tenant-consumer/openapi.yaml` only;
- user-token and service-token HTTP clients;
- pagination, retry, idempotency, and error handling per
  [transport-policy-v1.md](./transport-policy-v1.md).

**Must not**

- expose internal microservice OpenAPI as the public SDK;
- elevate an end-user token into platform-admin authority;
- accept caller-selected tenancy headers when the bearer already binds tenant.

## Proof consumers (Epic 15)

| Consumer | Role |
|---|---|
| `sesame-idam-client` (Rust) | Primary typed client; contract sync tests |
| Authlib (Python tooling) | Non-Rust proof: fixtures + principal schema |
| Auth.js (`ui/reference-authjs`) | Epic 16 reference only |
