# Story 17.16 — `sesame-idam-client` contract

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** E. Surfaces · **Depends on:** 17.13 · **Size:** S  
> **Status:** Ready

## Goal

Make the client's SAML calls contract, not pre-contract.

## Context

sesame-idam-client `src/saml.rs` (aligned 2026-10-10, 9fe5e0b).

## Scope

- Un-ignore `saml_operations_exist_in_provider_contract`; adjust to the published schemas.
- README SAML status: supported; examples for a BFF.
- Release a client version.

## Acceptance criteria

- [ ] The client's contract test passes against the published identity-login spec; CI green.

## Tests

- Contract test; unit tests.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
