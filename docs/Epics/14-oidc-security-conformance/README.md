# Epic 14: OIDC Security Profile and Conformance

> **Status:** In progress  
> **Program:** Standards-first OIDC provider  
> **Normative profile:** [`docs/standards-first-oidc/security-profile-v1.md`](../../standards-first-oidc/security-profile-v1.md) (`provider_profile` **1.0.0**)  
> **Audit source:** [Non-BRRTRouter framework readiness audit](../../audit/non-brrtrouter-framework-readiness-2026-07-25.md)  
> **Dependencies:** Epics 1, 5, 8, 11, 12, and 13 (runtime largely met)

## Outcome

Sesame's OIDC compatibility is demonstrated by independent conformance tooling,
shared adversarial fixtures, and unmodified mainstream framework validators.
Protocol support is counted only when runtime behavior, metadata, public routing,
and negative security tests agree.

## Story status

| Story | Title | Status | Notes |
|---|---|---|---|
| 14.1 | OIDC security profile | **Done** | Frozen in `security-profile-v1.md`; manifest `provider_profile` 1.0.0 |
| 14.2 | Shared token validation boundary | **Done** | `sesame_common::verify_access_token`; session-code + token-exchange wired |
| 14.3 | Deterministic protocol fixture set | **Done** | `conformance/oidc-v1` fixture_version 1.1.0 |
| 14.4 | Adversarial negative suite | **In progress** | BDD: redirect/PKCE/code/refresh/access-token/userinfo; state/nonce deferred |
| 14.5 | Metadata/runtime/public-route contract | **Mostly done** | `oidc_live_api` + fixture metadata contract; keep as regression |
| 14.6 | Independent OIDC conformance run | **In progress** | Readiness + discovery probe; full OIDF report not yet green |
| 14.7 | Framework compatibility matrix | **In progress** | First slice documented (Auth.js, Authlib, Spring RS) |
| 14.8 | Security observability and redaction | **Done** | Manifest `redacted_fields` + `sesame_common` redaction helpers/tests |
| 14.9 | Release conformance gate | **Done** | CI job `oidc-conformance-gate` + fixture checksum tooling |

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

## Normative profile

See [`security-profile-v1.md`](../../standards-first-oidc/security-profile-v1.md). Summary:

- Authorization Code only; PKCE S256; exact redirect; no implicit/password.
- Access tokens: EdDSA, `typ=at+jwt`; refresh rotated and client-bound.
- Tenant fail-closed; metadata honesty.

## Test evidence (2026-08-03)

- Profile freeze: `docs/standards-first-oidc/security-profile-v1.md`
- Corpus: `conformance/oidc-v1/{manifest,protocol-cases}.json` (v1.1.0)
- BDD: `identity-login-service` `tests/bdd/oidc_conformance.rs`
  (authorize negatives, code replay/cross-client, refresh rotation/replay/cross-client,
  access-token forgery set, userinfo sub binding, redaction gate)
- Shared validator: `sesame_common::jwt::verified_access`
- Interactive PKCE: `oidc_interactive` + `oidc_live_api::live_interactive_pkce_round_trip`
- OIDF readiness: [`evidence/oidf-basic-pkce-readiness.md`](./evidence/oidf-basic-pkce-readiness.md)
- Framework slice: [`evidence/framework-matrix-v1.md`](./evidence/framework-matrix-v1.md)
- Fixture checksum: `python -m sesame_idam_tooling.oidc_conformance`

## Acceptance gate

- [x] One validator replaces payload-only token trust in credential-minting paths (session-code, token exchange).
- [x] Every mandatory fixture family has a stable machine-readable representation.
- [x] Server projects consume the corpus via BDD + shared validator.
- [x] Metadata claims are checked against runtime and public routing (`oidc_live_api`).
- [ ] The selected external OIDC conformance profile passes (OIDF Basic/PKCE — report pending).
- [ ] Every selected framework completes positive login and rejects the negative token set (first slice documented; full list open).
- [ ] JWKS rotation works without synchronized client cache flushes.
- [x] Redaction gate covers manifest `redacted_fields` (unit + conformance test).
- [x] Protocol regression tests are release-blocking (`oidc-conformance-gate` CI job).
- [x] Compatibility evidence records exact framework/library versions for the first slice.

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
