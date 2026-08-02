# Epic 11: OIDC Relying-Party Registry and Tenant Binding

> **Status:** Proposed  
> **Program:** Standards-first OIDC provider  
> **Audit source:** [Non-BRRTRouter framework readiness audit](../../audit/non-brrtrouter-framework-readiness-2026-07-25.md)  
> **Dependencies:** Epic 10 tenant registry; ADR-011 authority separation; ADR-013 public issuer

## Outcome

Sesame has an authoritative registry for OAuth/OIDC relying parties. Every
authorization, token, refresh, and logout request can resolve its tenant,
application, redirect policy, client type, and allowed capabilities from a
registered client rather than a caller-selected `X-Tenant-ID`.

This epic is the tenancy and trust foundation for all mainstream framework
integrations.

## Why this epic exists

The existing `applications` table is not yet a complete OAuth client registry:

- redirect URIs are unstructured text;
- client secrets can be stored as plaintext-capable text;
- public and confidential clients are not distinguished;
- grants, scopes, audiences, and logout redirects are not registered policy;
- the current authorization handler does not validate clients;
- pre-authentication APIs rely on `X-Tenant-ID`;
- tenant upstream social-provider credentials are modelled separately but can
  be confused conceptually with customer relying-party clients.

OIDC libraries assume `client_id` identifies a registered relying party.
Sesame must make that identifier the structural tenant/application boundary.

## Scope

- registered public and confidential OIDC clients;
- exact redirect and post-logout redirect URI policy;
- client-to-tenant and client-to-application binding;
- allowed grants, response types, scopes, and audiences;
- confidential client authentication and secret lifecycle;
- tenant-console administration;
- audit and lifecycle status;
- migration of existing application records.

## Non-goals

- implementing `/oauth/authorize` or the token endpoint (Epic 12);
- tenant upstream Google/Microsoft configuration;
- dynamic client registration for arbitrary internet callers;
- platform-admin APIs on the relying-party public edge;
- framework-specific client libraries.

## Stories

| Story | Title | Result |
|---|---|---|
| 11.1 | Canonical OIDC client entity | Normalized client type, tenant, application, grants, scopes, audiences, status, and timestamps |
| 11.2 | Redirect URI and logout URI registry | Exact, normalized, multi-value redirect policy with no wildcard matching |
| 11.3 | Confidential client authentication | Hashed secrets, rotation overlap, revocation, and supported token-endpoint auth methods |
| 11.4 | Public client and PKCE policy | Public clients have no secret and require PKCE S256 |
| 11.5 | Tenant-console client lifecycle | Tenant admins create, inspect, rotate, disable, and delete only their own clients |
| 11.6 | Client-derived tenant and portal context | Auth flows derive tenant/application from validated `client_id` |
| 11.7 | Existing application migration | Existing application records migrate without ambiguous or unsafe defaults |
| 11.8 | Registry security and isolation BDD | Cross-tenant, redirect, secret, lifecycle, and authority-separation evidence |

## Required design decisions

### Client types

- **Public:** browser, native, desktop, or other clients unable to keep a secret.
- **Confidential:** server-side web applications and BFFs that can protect a
  credential.

The type is immutable after creation. Changing type requires a new client.

### Tenant binding

Every ordinary relying-party client belongs to exactly one Sesame tenant.
Tenant is derived from the client registry before authentication and from
validated token claims after authentication.

Platform-owned clients require a separate explicit classification and must not
be representable as tenant-owned clients with elevated flags.

### Redirect policy

- exact URI matching after defined normalization;
- HTTPS outside explicitly permitted loopback development/native cases;
- no wildcard hosts, paths, ports, query fragments, or suffix matching;
- separate allowlists for login callbacks and post-logout redirects;
- redirect URIs never supplied by tenant metadata outside the client registry.

### Secret policy

- secrets are shown once;
- only a password-hash-equivalent verifier is stored;
- rotation supports a bounded overlap period;
- revocation is immediate and audited;
- secret values are never logged or returned by read APIs.

## Acceptance gate

- [ ] A valid `client_id` resolves one active tenant and application.
- [ ] Unknown, disabled, or cross-tenant clients fail before user authentication.
- [ ] Redirect and post-logout URI matching is exact.
- [ ] Public clients cannot authenticate with or retrieve a secret.
- [ ] Public clients are marked PKCE-S256-required.
- [ ] Confidential secrets are hashed, rotatable, revocable, and redacted.
- [ ] Tenant admins cannot inspect or mutate another tenant's clients.
- [ ] Platform authority and tenant authority use separate credentials and routes.
- [ ] All client lifecycle operations emit security audit events.
- [ ] Existing application records have an explicit migration disposition.

## Test evidence

- entity/schema migration tests;
- tenant-isolation and RLS tests;
- redirect URI normalization and attack corpus;
- secret hash, rotation, overlap, and revocation tests;
- public/confidential policy tests;
- tenant-console authorization BDD;
- audit redaction tests;
- concurrent rotation and disablement tests.

## Security cases

- Unicode/punycode and case-confusable hostnames;
- loopback redirect port handling;
- encoded path traversal and duplicate query parameters;
- client ID enumeration;
- secret timing differences;
- stale-secret overlap after rotation;
- deleted/suspended tenant with otherwise active client;
- application moved between organizations;
- tenant admin attempting to mint a platform client.

## Exit condition

Epic 11 is complete when every protocol operation in Epic 12 can accept a
`client_id` and obtain a complete, tenant-safe policy decision without trusting
`X-Tenant-ID`, request-supplied redirect policy, or product-specific code.
