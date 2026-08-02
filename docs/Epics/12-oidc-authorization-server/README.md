# Epic 12: Standards-Compliant OIDC Authorization Server

> **Status:** Implemented (runtime smoke 2026-08-02)  
> **Program:** Standards-first OIDC provider  
> **Audit source:** [Non-BRRTRouter framework readiness audit](../../audit/non-brrtrouter-framework-readiness-2026-07-25.md)  
> **Dependencies:** Epic 1 JWKS; Epic 3 token lifecycle; Epic 5 revocation; Epic 11 client registry

## Outcome

Mainstream OIDC libraries can authenticate against Sesame using Authorization
Code flow with PKCE, exchange and refresh tokens through standard requests,
validate an ID token, call UserInfo, and end a session without custom protocol
code or endpoint overrides.

## Why this epic exists

The current provider surface is internally useful but not OIDC-complete:

- `/oauth/authorize` is an echo-style stub;
- the custom session-code handoff does not enforce PKCE;
- token requests are JSON-shaped rather than standard form requests;
- refresh lives on a separate non-discovered endpoint;
- ID tokens are generally absent;
- OAuth errors can be represented as empty token responses;
- discovery advertises capabilities not implemented by the handlers.

Framework libraries should drive the design. Auth.js, Spring Security,
ASP.NET Core, Authlib, OmniAuth, Socialite-compatible OIDC drivers, and
`coreos/go-oidc` must use their standard paths.

## Scope

- Authorization Code flow;
- PKCE S256, state, and OIDC nonce;
- hosted-auth session and consent handoff;
- standard token endpoint;
- access tokens, ID tokens, and refresh rotation;
- UserInfo subject consistency;
- OAuth/OIDC error semantics;
- optional confidential-client support;
- deprecation/migration of the custom session-code flow.

## Non-goals

- implicit or hybrid flow;
- resource-owner password credentials grant;
- framework-specific SDKs;
- dynamic client registration;
- device authorization grant;
- exposing RFC 8693 token exchange before its security gate is complete.

## Stories

| Story | Title | Result |
|---|---|---|
| 12.1 | Authorization request validation | Registered client, exact redirect, response type, scope, state, nonce, and PKCE are validated |
| 12.2 | Hosted-auth authorization session | Browser login/consent state is server-side, short-lived, tenant-bound, and tamper-resistant |
| 12.3 | One-time authorization codes | Codes are opaque, short-lived, single-use, and bound to client, redirect, user, scopes, nonce, and PKCE |
| 12.4 | Standard token endpoint transport | Form-encoded requests and declared client authentication methods |
| 12.5 | Authorization-code redemption | PKCE and confidential-client validation with OAuth-compliant errors |
| 12.6 | OIDC ID-token issuance | Signed ID tokens with correct issuer, audience, nonce, subject, and authentication claims |
| 12.7 | Refresh-token grant alignment | Discoverable refresh with rotation, replay detection, client binding, and stable errors |
| 12.8 | UserInfo conformance | Bearer-protected response whose `sub` matches the ID/access-token subject |
| 12.9 | Session-code migration | Existing custom handoff is removed, internal-only, or explicitly versioned during migration |
| 12.10 | Authorization-server interoperability BDD | Framework-native login, refresh, UserInfo, and negative-flow evidence |

## Required protocol profile

### Authorization request

Required:

- `client_id`;
- `response_type=code`;
- exact `redirect_uri`;
- `scope` containing `openid`;
- unpredictable `state`;
- `nonce`;
- `code_challenge`;
- `code_challenge_method=S256`.

Confidential clients may still be required to use PKCE. Public clients always
must.

### Authorization code

The code must be:

- opaque to the browser;
- high entropy;
- short TTL;
- single use with atomic redemption;
- bound to client, tenant, redirect URI, user/session, requested/granted scopes,
  nonce, and PKCE challenge;
- safe to include in a callback URL because it carries no token or identity data.

### Token endpoint

Initial supported grants:

- `authorization_code`;
- `refresh_token`;
- `client_credentials` for approved tenant service clients.

Supported client authentication methods must be explicit in discovery and
consistent with Epic 11. JSON bodies may remain on internal legacy endpoints,
but the public token endpoint must accept standards-compatible form encoding.

### ID token

At minimum:

- `iss`, `sub`, `aud`, `exp`, `iat`;
- `nonce` for authorization-code login;
- `auth_time`;
- `azp` when required by audience shape;
- optional profile claims only according to granted scopes.

Authorization roles and permissions remain in the access-token profile unless a
separate documented ID-token policy explicitly includes them.

## Error contract

- authorization errors redirect only after `redirect_uri` is validated;
- token errors use correct HTTP status and `error` values;
- no empty success-shaped token response represents failure;
- unknown client, redirect mismatch, invalid code, expired code, code reuse,
  PKCE mismatch, invalid grant, and invalid client remain distinguishable where
  the standards permit without creating an enumeration oracle;
- tokens, codes, verifiers, secrets, and session identifiers are redacted.

## Implementation evidence (2026-08-02)

- `oauth_authorize` / `oauth_authorize_complete` / `oauth_token` / `oauth_userinfo`
  handlers issue Authorization Code + PKCE S256, tokens, and UserInfo.
- Pre-auth RLS SELECT policies
  (`20260802190000_oidc_preauth_client_lookup.sql`) allow `client_id` lookup
  before `app.tenant_id` is known.
- `ErrorResponse` OpenAPI enum includes OAuth codes (`invalid_client`,
  `unsupported_grant_type`, `temporarily_unavailable`, …) so BRRTRouter no
  longer rewrites protocol errors into 500 response-validation failures.
- Live smoke: `GET /oauth/authorize` for `hauliage-web` returns `302` to
  `https://auth.sesameidentity.dev.local/authorize?request_id=…`; hosted auth
  HTML serves `200`; token without client auth returns `401 invalid_client`.

## Test evidence

- Unit: `oidc_authorization` (valid PKCE, redirect/scope/plain PKCE/state/nonce/
  response_type/openid-required negatives, OAuth error-code mapping).
- In-process BDD (`tests/bdd/oidc_protocol.rs`): preauth `ClientRegistry`
  resolve, authorize handler 302/invalid_client/bad redirect, Redis code
  mint/redeem with cross-client burn + replay rejection.
- Live API BDD (`tests/bdd/oidc_live_api.rs`): discovery/JWKS/authorize
  positive+negative, token fail-closed, userinfo 401, advertised endpoint
  reachability against `id.`/`auth.`/`api.`.
- OpenAPI contract: authorize/token public; userinfo BearerAuth; ErrorResponse
  OAuth enum membership.

## Acceptance gate

- [x] Authorization request + PKCE S256 reaches the hosted-auth surface.
- [ ] Full browser login → code → token → UserInfo path (interactive).
- [ ] State and nonce are preserved and validated end-to-end.
- [ ] Code redemption is atomic and single-use.
- [x] Token requests work with standard form encoding on `api./oauth/token`.
- [x] Confidential client auth is enforced at the token endpoint.
- [ ] ID tokens pass framework-native OIDC validators.
- [ ] Refresh rotation and replay detection work through the discovered endpoint.
- [ ] UserInfo `sub` matches the authenticated subject.
- [x] OAuth error codes survive OpenAPI response validation.
- [ ] No access, refresh, or ID token appears in a URL or log.
- [x] Implicit/hybrid capabilities are not advertised (discovery).

## Interoperability evidence

Before completion, unmodified standard clients must complete the flow:

- Auth.js generic `type: "oidc"` provider;
- Spring Security `oauth2Login`;
- ASP.NET Core `AddOpenIdConnect`;
- Authlib Starlette/FastAPI client;
- one additional independent OIDC conformance client.

Each client must be configured from issuer, client ID, client credential where
appropriate, and redirect URI only.

## Security regression set

- wrong/unknown/disabled client;
- redirect confusion and open redirect attempts;
- missing/replayed/changed state;
- missing/replayed/changed nonce;
- missing/plain/wrong PKCE verifier;
- concurrent code redemption;
- code/client/tenant/redirect substitution;
- authorization-session fixation;
- refresh replay and cross-client refresh;
- ID-token issuer/audience/nonce/time failures;
- UserInfo token substitution.

## Exit condition

Epic 12 is complete when the generic framework clients listed above complete
login, refresh, and UserInfo without Sesame-specific protocol handlers, and the
shared negative suite proves that malformed, forged, replayed, or cross-tenant
requests fail closed.
