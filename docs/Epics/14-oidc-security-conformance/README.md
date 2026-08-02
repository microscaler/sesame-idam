# Epic 14: OIDC Security Profile and Conformance

> **Status:** In progress (fixture runner + interactive PKCE evidence)  
> **Program:** Standards-first OIDC provider  
> **Audit source:** [Non-BRRTRouter framework readiness audit](../../audit/non-brrtrouter-framework-readiness-2026-07-25.md)  
> **Dependencies:** Epics 1, 5, 8, 11, 12, and 13

## Outcome

Sesame's OIDC compatibility is demonstrated by independent conformance tooling,
shared adversarial fixtures, and unmodified mainstream framework validators.
Protocol support is counted only when runtime behavior, metadata, public routing,
and negative security tests agree.

## Why this epic exists

OpenAPI presence and generated handlers do not prove OIDC interoperability.
The current repository contains security-critical examples:

- session-code minting checks an unverified JWT payload;
- token exchange parses unverified subject and actor tokens;
- discovery advertises unsupported flows;
- examples and docs still mention RS256 while runtime uses EdDSA;
- tenant headers can disagree with validated claims;
- multiple services and consumers duplicate claim interpretation.

A standards-first provider needs an explicit security profile and one
language-neutral test corpus shared by server code and every future client.

## Scope

- OAuth 2.0/OIDC security profile;
- token validation centralization;
- positive and negative conformance fixtures;
- independent OIDC provider conformance suite;
- mainstream framework compatibility matrix;
- metadata-to-runtime contract tests;
- redirect, PKCE, state, nonce, refresh, logout, and JWKS adversarial tests;
- security logging/redaction;
- release evidence and regression policy.

## Non-goals

- implementing protocol endpoints (Epic 12);
- implementing public routing (Epic 13);
- framework-specific product ergonomics (Epic 16);
- claiming certification before the selected conformance profile passes.

## Stories

| Story | Title | Result |
|---|---|---|
| 14.1 | OIDC security profile | Normative choices for flows, clients, PKCE, algorithms, tokens, sessions, and errors |
| 14.2 | Shared token validation boundary | One validator for signature, algorithm, type, issuer, audience, time, tenant, version, and denylist |
| 14.3 | Deterministic protocol fixture set | Language-neutral metadata, JWKS, token, code, refresh, UserInfo, and logout vectors |
| 14.4 | Adversarial negative suite | Forgery, substitution, replay, confusion, redirect, state, nonce, and tenant attacks |
| 14.5 | Metadata/runtime/public-route contract | Every advertised feature maps to a reachable implemented operation |
| 14.6 | Independent OIDC conformance run | Selected provider profile passes an external conformance suite |
| 14.7 | Framework compatibility matrix | Unmodified framework libraries validate the same provider and fixtures |
| 14.8 | Security observability and redaction | Stable events and metrics without credentials, codes, tokens, or secrets |
| 14.9 | Release conformance gate | CI and release policy prevent protocol drift |

## Normative profile decisions

Unless superseded by an accepted ADR:

- Authorization Code is the only browser login response type.
- PKCE S256 is required for public clients and preferred for all clients.
- Implicit flow is not supported.
- Redirect URIs use exact matching.
- Access tokens use `typ=at+jwt`.
- Access-token validation requires signature, EdDSA allowlist, exact issuer,
  intended audience, expiry, not-before, tenant, session/version policy, and
  denylist policy.
- ID-token validation requires signature, issuer, client audience, expiry,
  issued-at, nonce, and `azp` where required.
- Refresh tokens are rotated, client-bound, and server-side state is authoritative.
- `X-Tenant-ID` never overrides a validated client or token tenant.
- Unknown algorithms, key types, critical headers, and token types fail closed.
- Provider metadata advertises only completed behavior.

## Mandatory fixture families

### Metadata and keys

- valid discovery;
- missing/mismatched issuer;
- unreachable or wrong-host JWKS URI;
- malformed JWKS;
- current, next, grace, retired, revoked, and unknown `kid`;
- duplicate `kid`;
- wrong key type/use/algorithm;
- rotation while relying-party caches are warm.

### Access tokens

- valid org and no-org claims;
- wrong issuer/audience/type/algorithm;
- expired and not-yet-valid;
- missing required standard or Sesame claims;
- `sub != user_id`;
- top-level tenant different from namespaced tenant;
- malformed roles/permissions;
- modified header/payload/signature;
- `alg=none` and algorithm-confusion attempts;
- stale version and denied `jti`.

### Authorization and token

- valid public/confidential clients;
- wrong redirect/client/tenant;
- state and nonce mismatch/replay;
- missing/plain/wrong PKCE;
- authorization-code reuse and concurrent redemption;
- expired code;
- invalid client authentication;
- refresh rotation, replay, cross-client use, and Redis outage;
- invalid scope/audience escalation.

### UserInfo and logout

- valid UserInfo;
- UserInfo `sub` mismatch;
- access token from wrong client/tenant;
- valid/invalid `id_token_hint`;
- valid/invalid post-logout redirect;
- logout state preservation;
- revoked session rejected at resource server.

## Selected independent consumers

The compatibility matrix must include at least:

- Auth.js generic OIDC provider;
- Spring Security OAuth2 Login and resource-server JWT;
- ASP.NET Core OpenID Connect and JWT Bearer;
- Authlib;
- OmniAuth OpenID Connect;
- a PHP OIDC implementation selected for the Laravel adapter;
- `coreos/go-oidc`.

Passing one library is not sufficient evidence because parser and metadata
tolerance differ across ecosystems.

## Test evidence (2026-08-02)

- Machine-readable corpus: `conformance/oidc-v1/{manifest,protocol-cases}.json`
- BDD runner: `identity-login-service` `tests/bdd/oidc_conformance.rs`
  (redirect-prefix, PKCE plain, valid public PKCE, code replay/cross-client)
- Interactive PKCE: `tests/bdd/oidc_interactive.rs` + live
  `oidc_live_api::live_interactive_pkce_round_trip`
  (authorize → login → complete → token → userinfo) using `fixture-public-client`
- Seed: `impl/seeds/20260802220000_oidc_fixture_public_clients.sql`
- Hosted auth SPA completes OIDC via `/oauth/authorize/complete` when
  `request_id` is present (`frontend/auth`)

## Acceptance gate

- [ ] One validator replaces payload-only token trust in all credential-minting paths.
- [x] Every mandatory fixture has a stable machine-readable representation.
- [ ] Server and client projects consume the same expected outcomes.
- [x] All metadata claims are checked against runtime and public routing.
- [ ] The selected external OIDC conformance profile passes.
- [ ] Every selected framework completes positive login and rejects the negative token set.
- [ ] JWKS rotation works without synchronized client cache flushes.
- [ ] Logs, traces, metrics, and errors contain no token, code, verifier, or secret.
- [x] Protocol regression tests are release-blocking.
- [ ] Compatibility evidence records exact framework/library versions.

## Release evidence

- conformance suite report and profile/version;
- fixture bundle checksum/version;
- compatibility matrix;
- public endpoint smoke report;
- negative security test report;
- JWKS rotation report;
- redaction/log review;
- known deviations with accepted ADR and expiry/review date.

## Exit condition

Epic 14 is complete when Sesame's standards claims are independently reproducible
and every selected framework consumes the same provider without compatibility
patches. Future provider and client releases must continue to pass this gate.
