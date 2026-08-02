# Epic 13: Public OIDC Provider Surface

> **Status:** Implemented (runtime smoke 2026-08-02)  
> **Program:** Standards-first OIDC provider  
> **Audit source:** [Non-BRRTRouter framework readiness audit](../../audit/non-brrtrouter-framework-readiness-2026-07-25.md)  
> **Dependencies:** ADR-013; Epic 11 client registry; Epic 12 authorization server

## Outcome

Sesame exposes one coherent, externally reachable OIDC provider:

- `id.` is the stable, cookie-free issuer and discovery/JWKS host;
- `auth.` owns browser authentication, consent, and the IdP session;
- `api.` owns token, refresh, UserInfo, logout/revocation, and approved tenant
  API operations.

An external relying party can configure only the issuer and complete every
advertised operation without internal Kubernetes service URLs.

## Why this epic exists

The host separation in ADR-013 is sound, but the current routes do not yet form
an executable OIDC flow:

- discovery can identify the wrong fallback issuer/public URL;
- `/oauth/authorize` is advertised but excluded from the API edge;
- `/session/refresh` is implemented but not publicly routed;
- API-key operations are internal-only without a published M2M boundary;
- pre-auth APIs require a tenant header the edge strips;
- no Sesame API-edge CORS policy was found;
- public endpoint ownership is split among service paths rather than a stable
  product contract.

## Scope

- exact issuer and discovery URL behavior;
- public host and path ownership;
- root well-known discovery/JWKS;
- truthful discovery metadata;
- browser CORS and security headers;
- cookie boundaries;
- TLS, caching, rate limiting, and availability;
- public route allowlisting and accidental-exposure tests;
- relying-party logout/revocation routing;
- environment parity.

## Non-goals

- protocol handler implementation already owned by Epic 12;
- platform/tenant console UI implementation;
- exposing authz-core, platform-admin, tenant-console-admin, SCIM, or unsafe
  user-management endpoints;
- framework client libraries.

## Stories

| Story | Title | Result |
|---|---|---|
| 13.1 | Stable issuer configuration | Exact environment-specific issuer with no fallback drift or misspelling |
| 13.2 | Truthful discovery document | Only delivered endpoints, grants, response types, scopes, algorithms, claims, and auth methods |
| 13.3 | Hosted-auth route ownership | Authorization, login, MFA, and consent operate on `auth.` with host-only session cookies |
| 13.4 | Public API route ownership | Token, refresh, UserInfo, logout/revocation, and approved tenant APIs operate on `api.` |
| 13.5 | Issuer route invariants | `id.` serves only root discovery and JWKS, with no cookies or ambient authority |
| 13.6 | Credential-derived tenant edge | Public routes no longer depend on a trusted caller-provided tenant header |
| 13.7 | CORS and browser policy | Per-route origin policy, preflight behavior, exposed headers, and no wildcard credentialing |
| 13.8 | Rate limit, cache, and key-rotation behavior | Provider-safe budgets and JWKS caching across current/next/grace keys |
| 13.9 | Public route inventory guard | New internal/admin paths cannot become public accidentally |
| 13.10 | Environment parity and external smoke | Dev, staging, and production-shaped issuer/auth/API flows |

## Host contract

### `id.sesameidentity.com`

Permitted:

- `/.well-known/openid-configuration`;
- `/.well-known/jwks.json`.

Required properties:

- no inbound or outbound cookies;
- public cacheability according to the document type;
- stable issuer identity;
- no console, login, token, admin, or tenant API routes;
- high availability independent of brochure and console deploys.

### `auth.sesameidentity.com`

Permitted:

- authorization endpoint;
- login, registration, OTP/MFA, recovery, and consent user journeys;
- IdP session creation and termination;
- browser assets required for those journeys.

Required properties:

- cookies scoped to the exact host;
- strong CSRF/session fixation protections;
- CSP and framing policy;
- safe redirects based only on Epic 11 registration and Epic 12 state.

### `api.sesameidentity.com`

Permitted:

- token and refresh grants;
- UserInfo;
- logout/revocation/introspection if delivered and advertised;
- stable tenant-consumer API;
- approved M2M/token operations.

Required properties:

- inbound `Cookie` and outbound `Set-Cookie` stripped;
- bearer/client authentication only;
- tenant derived from validated client/token;
- explicit route allowlist;
- independent rate limits and availability from console workloads.

## Discovery invariants

- `issuer` exactly equals the configured issuer and token `iss`;
- `jwks_uri` is reachable and same-site with the issuer;
- authorization, token, UserInfo, and end-session URLs identify the public hosts;
- only `response_types_supported=["code"]` until another flow is intentionally
  implemented and justified;
- only delivered grants are advertised;
- `S256` is the only advertised PKCE method;
- EdDSA is advertised only if all selected framework validators support the
  delivered OKP/Ed25519 keys;
- token-endpoint authentication methods match Epic 11/12 behavior;
- subject types and claims are truthful;
- metadata is covered by automated contract tests.

## CORS policy

CORS is not a substitute for OAuth redirect security. It is required only for
documented browser-direct API operations.

- issuer documents allow safe public GET access;
- token endpoint browser access is allowed only for registered public-client
  origins and only if the architecture intentionally permits it;
- tenant APIs use an allowlist associated with the registered client/application;
- credentials mode is disabled on cookie-free `api.`;
- preflight caching and allowed headers are explicit;
- `Authorization` is permitted only where required;
- error responses carry the same CORS policy as success responses.

## Implementation evidence (2026-08-02)

- Live discovery at `https://id.sesameidentity.dev.local/.well-known/openid-configuration`
  advertises:
  - authorize → `https://auth.sesameidentity.dev.local/oauth/authorize`
  - token/userinfo → `https://api.sesameidentity.dev.local/oauth/{token,userinfo}`
  - JWKS → `https://id.sesameidentity.dev.local/.well-known/jwks.json`
  - grants `authorization_code` + `refresh_token` only; response_type `code` only; PKCE S256.
- Edge routes: `sesame-api-oidc`, auth `/oauth/authorize` rewrite, id well-known.
- Session service env: `SESAME_JWT_ISSUER`, `SESAME_AUTH_PUBLIC_URL`,
  `SESAME_API_PUBLIC_URL`.
- Hauliage BFF can fetch cluster JWKS from
  `identity-session-service.sesame-idam` (2 keys) after restart.

## Test evidence

- Session BDD: `oidc_discovery` asserts auth/api endpoint split, code-only
  response types, no implicit grant, S256-only PKCE.
- Live API BDD: truthful discovery document, JWKS without private material,
  advertised endpoints reachable on public hosts.
- CORS unit: `CORS_ALLOWED_ORIGINS` overrides config origins (Epic 13 edge).

## Acceptance gate

- [x] Issuer discovery works at the root well-known path (dev).
- [x] Discovery contains no stale fallback hostname or unsupported grants.
- [x] Advertised authorize/token/userinfo/JWKS are publicly reachable.
- [ ] No unadvertised internal/admin operation is publicly reachable (inventory).
- [ ] `id.` and `api.` cannot receive or issue cookies (edge tests).
- [ ] `auth.` cookies cannot be sent to platform or tenant consoles.
- [x] Authorize derives tenant from `client_id` (no public tenant header).
- [ ] Refresh, UserInfo (authenticated), and logout through public URLs.
- [ ] CORS allow/deny and OPTIONS behavior have executable tests.
- [x] JWKS reachable for independent validators (public + in-cluster BFF).
- [ ] Rate limits distinguish human auth, token, metadata, and tenant API traffic.
- [x] Dev topology mirrors production host split (`id.` / `auth.` / `api.`).

## Operational evidence

- synthetic issuer/discovery/JWKS checks;
- full external browser and server-side smoke tests;
- route inventory snapshot;
- TLS and HSTS evidence;
- cookie strip/set-cookie strip tests;
- CORS corpus;
- JWKS cache/rotation tests across at least two independent validators;
- rate-limit isolation and retry-header tests;
- issuer/auth/API availability SLOs and alerts.

## Exit condition

Epic 13 is complete when an application outside the Kubernetes cluster can use
only the public issuer/auth/API hosts to complete every capability Sesame
advertises, while route inventory tests prove that no platform or internal
authority crossed the public boundary.
