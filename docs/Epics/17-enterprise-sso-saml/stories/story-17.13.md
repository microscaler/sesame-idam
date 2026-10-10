# Story 17.13 — Code exchange, SP metadata and SSO discovery

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** D. Sign-in · **Depends on:** 17.12 · **Size:** M  
> **Status:** Ready

## Goal

Complete the relying-party side and the supporting public endpoints.

## Context

[DESIGN-saml-sp.md §3, §4](../../../DESIGN-saml-sp.md).

## Scope

- After linking: one-time `saml_access_code` (Redis, 60 s, bound to client and redirect URI) and redirect to the RP.
- `POST /auth/saml/callback`: exchange for a TokenResponse; client authentication as at the token endpoint; codes single use.
- `GET /auth/saml/sp/{connection_id}/metadata`: SP metadata (entity ID, ACS, SP certificate, NameID formats).
- `POST /auth/saml/discover {email, client_id}`: organisation with a VERIFIED or GRACE domain and an enabled connection, else empty; rate-limited; same response shape for unknown domains.
- Hosted authorize integration: the SSO flow may complete a pending authorize transaction (`redirect_uri` = authorize completion).
- Publish `saml_login`, `saml_acs`, `saml_callback`, `saml_sp_metadata`, `saml_domain_discovery` in the identity-login OpenAPI; un-ignore the identity-login half of `saml_track_a_contract`.

## Out of scope

- UI (17.15).

## Design notes

- Discovery timing does not depend on whether the domain exists (constant work).

## Acceptance criteria

- [ ] A code is usable once, by its client and redirect URI, within 60 s.
- [ ] SP metadata validates against the SAML metadata schema and imports into each IdP (manual check recorded).
- [ ] Discovery reveals nothing for unverified or unknown domains.
- [ ] The identity-login SAML contract test passes and gates.

## Tests

- Handler and contract tests; schema validation of metadata.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
