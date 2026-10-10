# Story 17.14 — Test sign-in and connection enablement

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** D. Sign-in · **Depends on:** 17.13 · **Size:** S  
> **Status:** Ready

## Goal

Let an organisation admin prove a configured connection end to end before members can use it.

## Context

[DESIGN-saml-sp.md §7](../../../DESIGN-saml-sp.md).

## Scope

- `test=true` on `saml_login` for org admins: runs the full flow, links the admin's own identity, issues no tokens to an RP, and records the result; success moves the connection to tested.
- `enable_saml` requires tested; any metadata change returns the connection to configured.

## Design notes

- The test result page shows which rule failed, for the admin only.

## Acceptance criteria

- [ ] A failed test leaves the connection configured with a reason; a successful one makes it tested; enabling an untested connection is refused.

## Tests

- BDD over the lifecycle with the verifier and fixtures.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
