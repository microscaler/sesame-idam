# Non-BRRTRouter Framework Readiness Audit — 2026-07-25

> **Status:** code-verified audit; planning input, not an implementation commitment  
> **Scope:** what remains before external applications can use Sesame through
> mainstream framework authentication libraries without BRRTRouter  
> **Repositories reviewed:** `sesame-idam`, `sesame-idam-client`, and the delivered
> Hauliage integration  
> **Source-of-truth order:** runtime code, generated code, OpenAPI, deployment
> configuration, then design documentation

## Executive finding

Sesame is a working tenant-scoped Bearer-JWT identity platform, but it is not yet
a drop-in OpenID Connect provider for arbitrary relying parties.

The delivered path is:

1. a product-specific BFF calls Sesame's login, organization, and session APIs;
2. Sesame returns EdDSA access and refresh tokens;
3. BRRTRouter fetches Sesame JWKS, validates the access token, and passes validated
   claims to the consumer;
4. Lifeguard can project those claims into PostgreSQL RLS context.

That path is proven by Hauliage. It is not portable to Auth.js, Spring Security,
ASP.NET Core, Django, FastAPI, Rails, Laravel, or Go OIDC middleware because the
standard authorization-code surface is incomplete and the public edge does not
currently expose a coherent end-to-end protocol.

**Planning conclusion:** do not begin by porting `sesame-idam-client` to other
languages. First make Sesame a standards-conforming OIDC provider and publish a
language-neutral consumer contract. Framework packages should then be thin
presets and claim mappers over each ecosystem's established OIDC libraries.

## Audit boundary

This audit covers:

- browser and server-side OIDC login;
- API/resource-server JWT validation;
- token refresh and logout;
- tenant, application, and organization context;
- the public `id.`, `auth.`, and `api.` surfaces;
- portable claims and tenant/admin API contracts;
- framework adapter and compatibility evidence.

It does not propose exposing platform-admin, user-management-admin, SCIM, SAML,
MCP, or internal authz-core APIs to ordinary relying parties.

## Delivered capabilities

The following are real enough to preserve and build on:

- password login and registration;
- Google/Microsoft social login;
- Ed25519/EdDSA access-token signing;
- public JWKS generation with current/next/grace keys;
- Redis-backed refresh-token rotation and replay detection;
- logout revocation write path;
- DB-backed UserInfo/current-user responses;
- organization creation, invitations, memberships, and active-organization
  token rotation;
- `client_credentials` issuance against tenant-scoped API keys;
- namespaced authorization claims suitable for policy and RLS;
- public issuer and API-edge deployment architecture;
- a working Rust/BRRTRouter consumer and a real Hauliage integration.

The canonical implemented access-token shape is in
`microservices/idam/common/src/jwt/types.rs`:

- standard JWT claims including `iss`, `sub`, `aud`, `exp`, `nbf`, `iat`, `jti`;
- `client_id`, `scope`, `ver`, and `sid`;
- `tenant_id`, `user_id`, `user_type`, and optional `org_id`;
- `https://sesame-idam.dev/claims` containing `tenant`, `portal`, `roles`,
  `permissions`, and optional entitlement metadata.

## Current viable non-BRRTRouter path

A non-BRRTRouter team can integrate today only by treating Sesame as a custom
Bearer-token API:

1. implement a BFF;
2. call the tenant-consumer/login APIs with `X-Tenant-ID`;
3. store refresh tokens server-side;
4. validate EdDSA access tokens against Sesame JWKS;
5. enforce exact `iss`, `aud`, `typ`, time, and claim requirements;
6. map the namespaced claims into the framework's authorization model;
7. implement refresh, logout, organization switching, and failure policy itself.

This is technically possible, but it is not a supported PropelAuth-like developer
experience and should not be presented as generic OIDC compatibility.

## P0 — security and protocol blockers

These items block external relying-party testing. They are ordered by dependency,
not estimated effort.

### NBF-01 — Verify tokens before minting or exchanging credentials

**Finding**

`identity-login-service/impl/src/controllers/auth_session_code.rs` claims the
presented access token is verified, but `token_is_ours` only splits the compact
JWT, base64url-decodes the payload, and compares `tenant_id`. An attacker can
forge that payload and obtain a redeemable session code.

`identity-login-service/impl/src/controllers/auth_token.rs` also parses subject
and actor token payloads without signature verification in the token-exchange
path and contains fallback identities for malformed/non-JWT inputs.

**Required outcome**

- Use one shared access-token validator that enforces signature, permitted
  algorithm, `typ`, issuer, audience, time claims, tenant, denylist, and token
  version.
- Remove all production fallbacks that manufacture claims for invalid tokens.
- Derive actor identity and delegation authority only from validated tokens and
  authoritative policy data.

**Acceptance evidence**

- forged-payload session-code mint is rejected;
- unknown `kid`, wrong issuer/audience/tenant, expired token, `alg=none`, and
  modified signature tests fail closed;
- token-exchange tests prove no unverified payload reaches issuance;
- live negative tests run through the public edge.

### NBF-02 — Establish a registered relying-party client model

**Finding**

The `applications` table contains `client_id`, a plaintext-capable
`client_secret`, and unstructured redirect URI text, but the authorization flow
does not use it as a complete OAuth client registry. Tenant upstream social
provider configuration is a different concern and must not be confused with
customer relying-party registration.

**Required outcome**

- Define public and confidential clients, exact redirect URIs, allowed grants,
  allowed scopes/audiences, post-logout redirect URIs, client status, tenant
  ownership, secret hashing/rotation, and application/portal identity.
- Bind every client to exactly one tenant or to an explicit platform-owned
  policy that cannot be changed by request headers.
- Expose tenant-admin lifecycle operations through the tenant console, not the
  public end-user API.

**Acceptance evidence**

- unregistered client and redirect URI are rejected;
- cross-tenant client use is impossible;
- public clients cannot use a secret grant;
- confidential secret rotation has overlap/revocation tests;
- redirect and post-logout URI matching is exact.

### NBF-03 — Implement Authorization Code + PKCE

**Finding**

`identity-login-service/impl/src/controllers/oauth_authorize.rs` is not an OAuth
authorization endpoint. It consumes generated fields such as `success`,
`user_id`, and `redirect_url`, emits an audit event, and echoes a response. It
does not validate a client, redirect to hosted authentication, preserve state,
bind a user session, obtain consent, or mint an authorization code.

The OpenAPI advertises PKCE parameters, but `TokenRequest` and the code redemption
path do not enforce a `code_verifier`.

**Required outcome**

- Implement Authorization Code flow for registered clients.
- Require PKCE S256 for public clients and support it for confidential clients.
- Preserve and return `state`; generate and validate OIDC `nonce`.
- Bind short-lived, one-time codes to client, redirect URI, tenant, user,
  authorization session, requested scopes, nonce, and PKCE challenge.
- Return errors through OAuth-compliant redirects only after the redirect URI
  has been validated.
- Reuse hosted `auth.` login/consent UI without transferring tokens through URLs.

**Acceptance evidence**

- Auth.js, Spring Security, and ASP.NET Core complete login using issuer-only
  discovery plus registered client credentials;
- code reuse, wrong verifier, wrong redirect URI, wrong client, expired code,
  and cross-tenant redemption all fail;
- state and nonce negative tests pass;
- no token appears in URLs, history, referrers, or logs.

### NBF-04 — Make the token endpoint standards-compatible

**Finding**

Discovery points at `/auth/token`, but the endpoint currently mixes custom JSON
requests, authorization-code handoff, client credentials, and RFC 8693 exchange.
Refresh is implemented separately at `/session/refresh`. Error paths can return
an empty success-shaped response instead of OAuth status/error semantics.
ID tokens are generally `null` while discovery advertises OIDC response types.

**Required outcome**

- Accept standard `application/x-www-form-urlencoded` token requests.
- Define supported client authentication methods in discovery.
- Implement authorization-code redemption with PKCE.
- Provide one discoverable refresh-token grant contract.
- Issue and validate OIDC ID tokens when `openid` is granted, including `aud`,
  `azp` where required, `nonce`, `auth_time`, and consistent subject semantics.
- Return RFC-compatible status codes and error bodies without empty token
  placeholders.
- Either harden token exchange fully or remove it from public discovery until
  it is production-ready.

**Acceptance evidence**

- standard clients exchange and refresh without custom transport hooks;
- public and confidential client authentication tests pass;
- ID-token validation passes framework-native validators;
- invalid grant/client/code/verifier errors are interoperable;
- refresh rotation and replay detection pass through the discoverable endpoint.

### NBF-05 — Publish truthful discovery metadata

**Finding**

`identity-session-service/impl/src/services/discovery.rs` advertises implicit and
hybrid response types, pairwise subjects, PKCE, authorization code, ID-token
signing, and multiple grant types that are not all delivered.

Its fallback public URL is the misspelled
`https://identity.seasame-idam.microscaler.local`, while ADR-013 fixes the issuer
at `https://id.sesameidentity.com`. The issuer route documents this known
self-consistency gap.

**Required outcome**

- Configure one exact issuer per environment.
- Serve discovery at `{issuer}/.well-known/openid-configuration`.
- Advertise only implemented endpoints, grants, response types, scopes,
  algorithms, subject types, token authentication methods, claims, and PKCE
  methods.
- Keep the issuer stable while allowing discovered endpoints to live on
  `auth.` and `api.`.
- Add `end_session_endpoint`, revocation, or introspection metadata only when
  the corresponding operation is delivered.

**Acceptance evidence**

- discovery is self-consistent in dev, staging, and production;
- all advertised URLs are externally reachable;
- an automated check exercises every advertised capability;
- common framework clients bootstrap from `issuer` without endpoint overrides.

### NBF-06 — Replace caller-selected tenancy with credential-derived tenancy

**Finding**

Most pre-authentication operations require `X-Tenant-ID`. The new API edge
strips that header to mitigate an org-mgmt header-over-verified-claim vulnerability.
Consequently public login and registration lose the tenant value they require.

For authenticated requests, accepting a caller-controlled tenant header beside
a signed tenant claim creates an avoidable precedence hazard.

**Required outcome**

- Derive tenant for authorization flows from the registered `client_id`.
- Derive tenant for authenticated API calls from validated access-token claims.
- Define how hosted auth resolves tenant before authentication, without trusting
  an arbitrary public header.
- Retain explicit tenant parameters only for internal/platform operations where
  caller authority independently permits selecting a tenant.
- Fix east-west services as well as edge behavior; header stripping is not the
  underlying fix.

**Acceptance evidence**

- login/register work through the public surface without a trusted
  caller-supplied tenant header;
- a tenant-A token/client can never select tenant B;
- edge and east-west tests enforce identical tenant isolation;
- all tenant resolution paths have precedence and negative tests.

### NBF-07 — Complete public routing, CORS, and endpoint ownership

**Finding**

The issuer host correctly routes only discovery and JWKS. The API edge excludes
`/oauth/*`, while discovery advertises `/oauth/authorize`. It also omits the
implemented `/session/refresh` and API-key service. No Sesame microservice or
API-edge CORS policy was found.

The intended hosted-auth origin is separate from both issuer and API origins,
but endpoint ownership among `auth.`, `api.`, and the internal services is not
yet executable as one flow.

**Required outcome**

- Route browser authorization and hosted login/consent on `auth.`.
- Route token, refresh, UserInfo, logout/revocation, tenant-consumer, and approved
  M2M operations through explicit `api.` paths.
- Keep discovery/JWKS cookie-free on `id.`.
- Define an allowlisted CORS policy per endpoint family; do not use wildcard
  credentialed origins.
- Continue stripping ambient cookies from `api.` and `id.` while allowing
  hosted-auth session cookies only on `auth.`.
- Keep platform-admin, tenant-console-admin, authz-core, and unsafe user-management
  paths off the relying-party surface.

**Acceptance evidence**

- a browser and a server-side client complete the full flow through public hosts;
- OPTIONS/preflight, origin allow/deny, cookie stripping, and security headers
  have executable tests;
- route inventory tests fail when a new service path is accidentally exposed;
- refresh and logout work without internal service URLs.

### NBF-08 — Deliver interoperable logout and revocation

**Finding**

`/auth/logout` implements token revocation behavior, but the OIDC
`/oauth/logout` handler is a stub and discovery has no end-session metadata.
Hauliage's frontend logout currently clears local state rather than proving
provider logout propagation.

**Required outcome**

- Define local application logout separately from Sesame IdP-session logout.
- Implement RP-initiated logout with validated `id_token_hint`,
  `post_logout_redirect_uri`, and logout `state`.
- Revoke refresh-token families and denylist access tokens according to the
  published session policy.
- Document front-channel/back-channel logout scope; advertise only what exists.

**Acceptance evidence**

- logout through at least Auth.js, Spring, and ASP.NET works;
- invalid post-logout redirect URIs are rejected;
- a revoked session fails at a resource server within the documented bound;
- Redis outage behavior remains fail-closed and bounded.

## P1 — portable consumer contract

These tasks can begin while P0 implementation is underway, but they cannot be
declared stable until the protocol gates pass.

### NBF-09 — Publish one language-neutral authentication profile

The portable source of truth must live in `sesame-idam`, not in the Rust client.
It should define:

- issuer and discovery rules;
- supported OAuth/OIDC flows and client types;
- exact access-token and ID-token profiles;
- JWKS caching and key-rotation expectations;
- issuer, audience, algorithm, type, and time validation;
- tenant/application/org claim semantics;
- required versus optional claims before and after onboarding;
- refresh, logout, revocation, and failure policy;
- stable machine-readable error categories;
- browser, BFF, API, worker, and CLI threat models.

The current root README and wiki must be corrected: they still describe RS256,
flat roles/permissions, `SesameExecutor`, and an implemented frontend SDK, while
runtime code uses EdDSA, namespaced claims, Lifeguard session transactions, and
no published TypeScript package.

### NBF-10 — Define three separate client products

Do not make one SDK own unrelated trust boundaries.

1. **OIDC relying-party preset**
   - issuer/client configuration;
   - framework-native login/session hooks;
   - refresh/logout recipes;
   - Sesame claim mapping.
2. **Resource-server adapter**
   - framework-native JWT/JWKS validation;
   - strict issuer/audience/type/algorithm defaults;
   - normalized Sesame principal and policy helpers;
   - optional RLS context projection.
3. **Tenant/admin API client**
   - supported user/org/membership/invitation operations;
   - pagination, idempotency, retries, and structured errors;
   - service credentials and end-user delegation kept explicit.

### NBF-11 — Stabilize the public OpenAPI contract

`openapi/idam/tenant-consumer/openapi.yaml` is the best starting point, but it
still describes header-selected tenancy and is not the deployed aggregation
contract. Six service-specific specs are useful for service generation but are
not an acceptable public SDK boundary.

Required work:

- publish one public relying-party/tenant API document;
- exclude internal/admin operations by construction;
- align operation paths and schemas with public routing;
- settle `TokenResponse`, errors, pagination, and idempotency;
- generate clients only from this stable public document;
- test the document against live public endpoints.

### NBF-12 — Publish shared conformance fixtures

All language adapters must consume the same fixtures:

- deterministic discovery and JWKS documents;
- current, next, retired, and unknown `kid` cases;
- valid access and ID tokens;
- optional/no-organization and active-organization claims;
- wrong issuer/audience/nonce/state/tenant;
- expired/not-yet-valid tokens;
- `alg=none`, algorithm confusion, and modified signatures;
- PKCE mismatch, code reuse, redirect mismatch, refresh replay;
- UserInfo `sub` mismatch;
- logout and revocation;
- namespaced roles, permissions, and entitlement references.

Use black-box protocol tests in addition to generated-handler unit tests.

### NBF-13 — Reconcile `sesame-idam-client`

Keep the crate as the supported BRRTRouter/may adapter, but align it with the
portable contract:

- make optional `org_id` representable before onboarding;
- reconcile entitlement fields and remove/confirm legacy `org_type`;
- align the complete `TokenResponse`;
- add refresh/logout only after the public operations stabilize;
- complete contract tests for session, identity, delete, and response schemas;
- unify duplicate HTTP error shapes;
- remove or feature-gate SAML methods until provider OpenAPI owns them;
- document that cryptographic verification happens before claims parsing.

It should not become the semantic source from which other languages are ported.

## P2 — framework delivery backlog

Framework work begins after the P0 conformance gates are demonstrably green.

### NBF-14 — Auth.js preset and samples

**Coverage:** Next.js, SvelteKit, and Express.

Deliver a thin named OIDC provider that uses issuer discovery and framework-native
PKCE/state/nonce handling. Add claim/session callbacks, refresh rotation, provider
logout, protected API, and organization-switching examples. Do not implement a
parallel OAuth stack.

### NBF-15 — Laravel Socialite driver

Laravel has the strongest ergonomic expectation for a named provider and uneven
generic OIDC support. Deliver a driver that delegates protocol validation to a
maintained OIDC implementation and proves issuer, audience, nonce, PKCE, and
JWKS validation. Do not recommend `stateless()` for browser login.

### NBF-16 — Django and Rails presets

- django-allauth generic OIDC provider configuration and claim adapter;
- Rails OmniAuth OpenID Connect strategy/preset and Devise example;
- server-side refresh-token storage and framework authorization mapping.

### NBF-17 — Standards-first reference applications

Publish conformance-tested examples rather than unnecessary SDKs for:

- ASP.NET Core `AddOpenIdConnect` and `AddJwtBearer`;
- Spring Security `oauth2Login` and resource-server JWT;
- FastAPI/Starlette with Authlib;
- Go with `coreos/go-oidc` and `x/oauth2`.

Each example must show login, protected route, API bearer validation, refresh,
logout, organization context, role/permission mapping, and negative tests.

### NBF-18 — Backend tenant/admin clients

Generate and hand-curate clients only after NBF-11 stabilizes. Initial language
priority should follow real customer demand rather than mirroring the login
adapter list. SDKs need:

- explicit user-token versus service-token methods;
- safe retries only for idempotent operations;
- idempotency keys for supported writes;
- pagination and rate-limit metadata;
- stable structured errors;
- token/secret redaction;
- semantic versioning, provenance, and compatibility CI.

### NBF-19 — Dogfood the external contract with Hauliage

Hauliage currently proves the internal BRRTRouter path. It must also prove the
customer path:

- use the public issuer and public API/auth hosts;
- use registered-client authorization code + PKCE;
- remove direct knowledge of Sesame service topology from application code;
- use the same resource-server claim profile and conformance fixtures;
- prove refresh, logout/revocation, org switch, and RLS;
- retain BRRTRouter only as one resource-server implementation, not as a
  protocol prerequisite.

## Dependency order

```text
NBF-01 token trust ───────────────────────────────────────────────┐
NBF-02 client registry ──> NBF-03 authorize+PKCE ──> NBF-04 token│
NBF-06 tenant derivation ─────────────────────────────────────────┤
NBF-05 truthful discovery <────────────────────────────── NBF-04 │
NBF-07 public routing/CORS <──────────────────── NBF-03/04/05/06 │
NBF-08 logout/revocation <────────────────────────── NBF-02/04/07│
                                                                 ▼
NBF-09 profile ──> NBF-10 client boundaries ──> NBF-11 OpenAPI
        └──────────────────────> NBF-12 conformance fixtures
                                      │
                                      ├──> NBF-13 Rust reconciliation
                                      ├──> NBF-14 Auth.js
                                      ├──> NBF-15 Laravel
                                      ├──> NBF-16 Django/Rails
                                      ├──> NBF-17 .NET/Spring/Python/Go
                                      ├──> NBF-18 admin clients
                                      └──> NBF-19 Hauliage dogfood
```

## Epic decomposition

The planning backlog is organized into six provider-first epics:

1. [Epic 11: OIDC Relying-Party Registry and Tenant Binding](../Epics/11-oidc-client-registry/README.md)
2. [Epic 12: Standards-Compliant OIDC Authorization Server](../Epics/12-oidc-authorization-server/README.md)
3. [Epic 13: Public OIDC Provider Surface](../Epics/13-oidc-public-provider-surface/README.md)
4. [Epic 14: OIDC Security Profile and Conformance](../Epics/14-oidc-security-conformance/README.md)
5. [Epic 15: Language-Neutral Sesame Consumer Contract](../Epics/15-portable-consumer-contract/README.md)
6. [Epic 16: Mainstream Framework Client Ecosystem](../Epics/16-mainstream-client-ecosystem/README.md)

Epics 11–15 make Sesame independently consumable through generic OIDC
libraries. Epic 16 selects and implements supported language/framework packages
only after those provider gates pass.

## Proposed delivery gates

### Gate A — safe protocol core

Closes NBF-01, NBF-02, NBF-03, NBF-04, and NBF-06.

Exit condition: a registered public client completes authorization code + PKCE,
and all token forgery, redirect, code-reuse, PKCE, and cross-tenant negative
tests pass.

### Gate B — truthful public provider

Closes NBF-05, NBF-07, and NBF-08.

Exit condition: issuer-only configuration works from outside the cluster for
login, token, refresh, UserInfo, JWKS, and logout; discovery advertises nothing
else.

### Gate C — portable contract

Closes NBF-09 through NBF-12.

Exit condition: one public protocol/OpenAPI profile and one conformance fixture
suite are versioned, executable, and independent of BRRTRouter.

### Gate D — first mainstream integration

Closes NBF-14 and the Auth.js slice of NBF-19.

Exit condition: clean Next.js and SvelteKit examples integrate from published
documentation without custom endpoint overrides or Sesame-internal knowledge.

### Gate E — ecosystem breadth

Closes NBF-13 and NBF-15 through NBF-19 according to customer priority.

Exit condition: every supported adapter passes the same conformance suite and
has a published compatibility and maintenance policy.

## Definition of “usable without BRRTRouter”

Sesame is usable by a non-BRRTRouter framework only when all of the following
are true:

- issuer-only discovery is sufficient;
- a registered framework client completes Authorization Code + PKCE;
- the framework's native OIDC validator accepts Sesame ID tokens;
- the framework's native JWT validator accepts Sesame API access tokens;
- no consumer must know internal service hostnames;
- tenancy is derived from validated credentials, not a caller-selected header;
- refresh and logout work through public documented endpoints;
- roles, permissions, organization, and tenant map through native framework
  policy hooks;
- all shared negative conformance vectors pass;
- documentation describes delivered code rather than target architecture.

Until then, the supported characterization should remain:

> Sesame supports custom BFF integration with Bearer JWT validation. Generic OIDC
> relying-party compatibility is under development.

## Code and configuration anchors

- `microservices/idam/identity-login-service/impl/src/controllers/oauth_authorize.rs`
- `microservices/idam/identity-login-service/impl/src/controllers/auth_session_code.rs`
- `microservices/idam/identity-login-service/impl/src/controllers/auth_token.rs`
- `microservices/idam/identity-session-service/impl/src/controllers/auth_refresh.rs`
- `microservices/idam/identity-session-service/impl/src/controllers/oauth_userinfo.rs`
- `microservices/idam/identity-session-service/impl/src/services/discovery.rs`
- `microservices/idam/common/src/jwt/types.rs`
- `openapi/idam/identity-login-service/openapi.yaml`
- `openapi/idam/identity-session-service/openapi.yaml`
- `openapi/idam/tenant-consumer/openapi.yaml`
- `helm/sesame-idam-api-edge/templates/httproute-api.yaml`
- `helm/sesame-idam-api-edge/templates/httproute-issuer.yaml`
- `helm/sesame-idam-api-edge/templates/_helpers.tpl`
- `helm/sesame-idam-api-edge/values.yaml`
- `docs/ADR-013-public-domain-and-issuer.md`
- `docs/roadmap/launch-1.0/p4-developer-contract/README.md`
- `../sesame-idam-client/src/config.rs`
- `../sesame-idam-client/src/claims.rs`
- `../sesame-idam-client/tests/contract_sync.rs`

## Verification note

This was a read-only code and contract audit. No runtime behavior was changed,
and no build or test command was run. Every item above therefore requires its
own implementation and executable evidence before its status can move from
outstanding to delivered.
