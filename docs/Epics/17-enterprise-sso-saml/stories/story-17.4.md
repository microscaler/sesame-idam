# Story 17.4 — Organisation domain API

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** A. Domains · **Depends on:** 17.2, 17.3 · **Size:** M  
> **Status:** Ready

## Goal

Let an organisation admin claim a domain, see the TXT record to publish, verify it, and manage auto-join; let a tenant admin revoke.

## Context

[DESIGN-org-domain-verification.md §6](../../../DESIGN-org-domain-verification.md); FINDING-2026-07-25 (no tenant from headers).

## Scope

- OpenAPI (org-mgmt): `create_org_domain`, `list_org_domains`, `verify_org_domain`, `update_org_domain`, `delete_org_domain`; tenant console `revoke_org_domain`. Remove `update_org_domains` and its request schema.
- Handlers: eligibility (17.1) at create; inline verification on `verify_org_domain` with a rate limit (1/min/claim); PENDING claims polled by the job (17.5).
- Authorisation: organisation from the path and the validated token; org admins (owner/admin roles) for mutations, members for list; tenant admins for revoke.
- Errors: `invalid_domain`, `domain_unavailable` (covers ineligible and held elsewhere), `not_verified` (auto_join on a non-VERIFIED claim).

## Out of scope

- Console UI (17.15).

## Design notes

- `K_enroll` mounted from the secret backend; the response shows the record name and value; nothing in any response reveals another organisation's claim.

## Acceptance criteria

- [ ] No request body or path can set a claim's state.
- [ ] A member who is not an org admin cannot create, verify, update or delete; a user of another tenant gets 404.
- [ ] Creating `gmail.com` or a public suffix returns `domain_unavailable`; the same response as a domain held by another organisation.
- [ ] Publishing the shown record and calling verify moves the claim to VERIFIED (mock `dns-probe` in tests); a record for another organisation does not.
- [ ] `update_org_domains` is gone from the spec and the code; the contract test for org-mgmt domain operations passes.

## Tests

- Handler unit tests with a mock `dns-probe`; BDD for authorisation and isolation; OpenAPI lint.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
