# Story 17.11 — Assertion consumer service and replay protection

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** D. Sign-in · **Depends on:** 17.6, 17.10 · **Size:** M  
> **Status:** Ready

## Goal

Receive the IdP's POST, consume the state, have the verifier decide, and refuse replays.

## Context

[DESIGN-saml-sp.md §3, §5 (identity-login)](../../../DESIGN-saml-sp.md).

## Scope

- `POST /auth/saml/acs/{connection_id}` (form): consume `saml_state` (one use), check the path connection equals the state's and is enabled and allowed, call `/verify` with pinned certificates, entity IDs, ACS and request ID.
- Replay cache `saml_assertion:<connection>:<assertion id>` until NotOnOrAfter; a repeat is refused.
- Failure handling: valid state → redirect to the RP with `error=access_denied`; invalid state → static error page; details only in the audit log.

## Out of scope

- Linking (17.12).

## Design notes

- The handler holds no XML; it passes the base64 string through.

## Acceptance criteria

- [ ] A replayed response, a response for another connection, an unsolicited response and a response after state expiry are all refused.
- [ ] The verifier being unavailable fails closed with a retry-later page.
- [ ] No assertion content appears in logs.

## Tests

- Handler tests with a mock verifier; end-to-end test with the real verifier and corpus fixtures.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
