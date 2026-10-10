# Story 17.7 — Verification corpus and fuzzing

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** B. Verifier · **Depends on:** 17.6 · **Size:** L  
> **Status:** Ready

## Goal

Prove the verifier against real-shaped responses from the three IdPs and against every known attack class, and keep proving it on every dependency bump.

## Context

[DESIGN-saml-sp.md §9](../../../DESIGN-saml-sp.md); ADR-014 §4 (supply chain).

## Scope

- Positive corpus: responses shaped as Entra ID, Okta and Google Workspace emit them (response-signed, assertion-signed, both; RSA and ECDSA), re-signed with test keys; generator script so fixtures are reproducible.
- Negative corpus, one case per rule: unsigned; wrong key; certificate only in KeyInfo; SHA-1; XSW1–8; second Assertion; comment in NameID; DOCTYPE, external entity, entity expansion; processing instruction; CDATA; EncryptedAssertion; wrong issuer, audience, recipient, destination, InResponseTo; expired; not yet valid; no AuthnStatement; non-Success; oversize; bad base64.
- Parser-differential cases (namespace redeclaration, duplicate attributes, entity references) that must be refused or read identically by libxml2 and quick-xml.
- `cargo fuzz` target over mutated corpus documents asserting no forbidden accept; CI smoke run; a scheduled longer run.
- CI job: the corpus runs on every change to the verifier and on any samael, libxmlsec1 or libxml2 bump.

## Out of scope

- Live IdP traffic (17.18).

## Design notes

- Fixtures are data with a manifest (expected verdict and reason code per file); no assertion in a fixture is valid for any real IdP.

## Acceptance criteria

- [ ] 100% of the negative corpus refused with the expected reason; 100% of the positive corpus accepted.
- [ ] The fuzz target finds no forbidden accept in its CI budget.
- [ ] A dependency bump without a green corpus cannot merge.

## Tests

- The corpus is the test.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
