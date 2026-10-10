# Story 17.18 — Live acceptance and security evidence

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** F. Delivery · **Depends on:** all · **Size:** L  
> **Status:** Ready

## Goal

Show the whole epic working with real IdPs and record the evidence that closes the acceptance gate.

## Context

Epic acceptance gate; the deployment's acceptance plan.

## Scope

- Test organisations in Entra ID, Okta and Google Workspace, each with a custom domain the test can verify; scripted set-up where the IdP allows it.
- End-to-end: claim and verify a domain, configure the connection from IdP metadata, test sign-in, enable, member sign-in to a relying party, JIT, replay attempt, domain record removal → GRACE → REVOKED.
- Security evidence: the corpus report, fuzz run summary, connectivity matrix, audit trail extracts, threat-table walk-through.
- Evidence recorded in this epic's README.

## Acceptance criteria

- [ ] Every box of the epic acceptance gate checked with linked evidence.

## Tests

- The acceptance run.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
