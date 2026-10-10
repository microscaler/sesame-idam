# Story 17.15 — Hosted "Sign in with SSO" and console administration

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** E. Surfaces · **Depends on:** 17.4, 17.14 · **Size:** L  
> **Status:** Ready

## Goal

Make SSO usable without API calls: the hosted login offers it, and the organisation and tenant consoles manage domains and connections.

## Context

ADR-010 (hosted auth, client SDK); [DESIGN-saml-sp.md §3, §11](../../../DESIGN-saml-sp.md); [DESIGN-org-domain-verification.md §6](../../../DESIGN-org-domain-verification.md).

## Scope

- Hosted auth app: "Sign in with SSO" (email → discovery → saml_login), error states.
- Organisation console: domains (claim, the record to publish with copy buttons, verify, states, auto-join), SAML (setup link, metadata upload or URL, the values for each IdP per §11, test sign-in, enable/disable).
- Tenant console: allow/disallow SAML per organisation, view and revoke domain claims.

## Out of scope

- Branding of the IdP buttons.

## Design notes

- Per-IdP set-up guidance rendered from one data table (DESIGN-saml-sp.md §11).

## Acceptance criteria

- [ ] Each flow works with keyboard only and on narrow screens; no secret or token appears in the UI; every action maps to an audited API call.

## Tests

- Component tests; Playwright end-to-end against the dev stack with the mock IdP.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
