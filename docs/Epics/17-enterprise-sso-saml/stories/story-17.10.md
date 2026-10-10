# Story 17.10 — SP key and signed AuthnRequest (`saml_login`)

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** D. Sign-in · **Depends on:** 17.8 · **Size:** M  
> **Status:** Ready

## Goal

Start SP-initiated sign-in: validate the request, record the pending state, and send the browser to the IdP with a signed AuthnRequest.

## Context

[DESIGN-saml-sp.md §3, §4](../../../DESIGN-saml-sp.md); ADR-014 §5.

## Scope

- SP key (RSA 3072) and certificate per deployment from the secret backend; generate-if-absent is the platform's job (17.17).
- `GET /auth/saml/login?org_id&client_id&redirect_uri`: client registered, redirect URI registered for it (Epic 11), connection enabled; state in Redis `saml_state:<relay>` (request ID, tenant, org, connection, client, redirect_uri), 10 min, one use.
- AuthnRequest XML (ID, IssueInstant, Destination, AssertionConsumerServiceURL, Issuer = SP entity ID, NameIDPolicy), DEFLATE, base64, HTTP-Redirect query signed RSA-SHA256 with ring (SigAlg, Signature over the exact query bytes).

## Out of scope

- ACS (17.11).

## Design notes

- AuthnRequest built with quick-xml writer; no XML signing; RelayState is 32 random bytes base64url.

## Acceptance criteria

- [ ] An unregistered redirect URI, unknown client, or a connection that is not enabled is refused before any redirect.
- [ ] The redirect signature verifies with the SP certificate (test with an independent implementation, e.g. xmlsec1's or samael's verification of redirect bindings).
- [ ] State is stored once and expires after 10 minutes.

## Tests

- Unit tests for encoding and signing; handler tests; Redis integration test.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
