# Story 17.6 — `saml-verifier` service

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** B. Verifier · **Depends on:** — · **Size:** L  
> **Status:** In progress

## Goal

A stateless internal service that verifies SAML responses and parses IdP metadata with samael and libxmlsec1, applying every rule of DESIGN-saml-sp.md §5 that is the verifier's.

## Context

[ADR-014](../../../ADR-014-saml-signature-verification.md) §1–4; [DESIGN-saml-sp.md §4 (verifier), §5, §10](../../../DESIGN-saml-sp.md).

## Scope

- New crate and BRRTRouter service `saml-verifier` with OpenAPI: `POST /verify`, `POST /metadata/parse`, health.
- Pre-checks (quick-xml on the raw document): size, base64, UTF-8, no DOCTYPE, comments, processing instructions, CDATA or EncryptedAssertion, exactly one Assertion, at least one pinned certificate.
- samael (`xmlsec` feature, exact version) configured per ADR-014 §1: algorithm allow-list, IdP-initiated off, ACS, entity IDs, clock skew; default reduction mode.
- Post-checks: Success status, assertion ID equals the verified reference, AuthnStatement present, NameID non-empty; normalised result (name_id, format, attributes, assertion_id, not_on_or_after, session_not_on_or_after, authn_instant).
- `/metadata/parse`: entity ID, HTTP-Redirect SSO URL, signing certificates with expiry, NameID formats; size limit; same pre-checks for DOCTYPE and entities.
- Dockerfile (Debian bookworm, digests pinned, runtime with only libxmlsec1-openssl, libxml2, libxslt, OpenSSL), skaffold docker artifact; Tilt resource for dev.

## Out of scope

- Corpus beyond smoke cases (17.7).

## Design notes

- Refusals return a fixed reason code and never echo input; no logging of assertion content (IDs and reason codes only); request body limit enforced before parsing.

## Acceptance criteria

- [ ] A response signed by the pinned key and satisfying every rule verifies; the same response with the certificate removed from the request is refused (never 'verified without certificates').
- [ ] Each pre- and post-check has at least one refusing test.
- [ ] The image runs as non-root on a read-only root file system and needs no network.
- [ ] The pipeline builds and publishes the image; the suite's run image is unchanged.

## Tests

- Unit tests with samael's test vectors and our first fixtures; container test in CI.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.

## Implementation notes

- `microservices/saml-verifier/`: a Cargo workspace of its own (excluded from the suite workspace), so libxmlsec1,
  libxml2 and OpenSSL never reach the suite or the Rust buildpack; samael pinned `=0.0.22`.
- Done (2026-10-11): the verification core. `precheck` (one namespace-aware quick-xml pass: no DOCTYPE, comments,
  processing instructions, CDATA, encrypted content; root a single `samlp:Response`; exactly one element named
  `Assertion`, in any namespace; at most two signatures, no `ds:Object`/`Manifest`/`RetrievalMethod`, transforms and
  canonicalisation from an allow-list; depth limit) and `verify` (strict base64, 256 KiB limit, pinned certificates
  required, samael with an algorithm allow-list and IdP-initiated off, then post-checks: assertion ID equals the
  document's, status Success, destination, audience required, `AuthnStatement`, non-empty `NameID`).
- Tests sign responses at run time with a throwaway IdP key and certificate (OpenSSL), so no private key is committed:
  a valid response, pinned and rotated certificates, tampering, comment injection, a second assertion, each binding
  (audience, recipient, `InResponseTo`, status, `AuthnStatement`, expiry, `NameID`), SHA-1 refused, DOCTYPE/XXE, bad
  base64, size.
- HTTP layer: a small synchronous server (tiny_http) rather than a generated BRRTRouter service: two internal
  endpoints, no auth, no database, and a crate outside the suite workspace; the OpenAPI document describes the contract.
- Remaining: HTTP server and OpenAPI, `/metadata/parse`, Dockerfile and skaffold artifact (tests run in the image
  build), Tilt resource.

