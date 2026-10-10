# Story 17.12 — Identity linking and just-in-time provisioning

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** D. Sign-in · **Depends on:** 17.4, 17.11 · **Size:** M  
> **Status:** Ready

## Goal

Decide who the verified SAML subject is in the organisation's tenant, linking or provisioning only through a VERIFIED domain.

## Context

[DESIGN-saml-sp.md §6](../../../DESIGN-saml-sp.md); [DESIGN-org-domain-verification.md §5](../../../DESIGN-org-domain-verification.md).

## Scope

- `saml_identities` table (RLS) and model.
- Existing link (connection, NameID) → that user.
- First sign-in: email (connection attribute, else NameID if an email) must be in a VERIFIED domain of the organisation (17.3 query, scope honoured); link an existing tenant user who is a member, or provision (JIT) with the default role, or refuse `not_provisioned`.
- Token issuance through `issue_login_response` with organisation, `amr: ["saml"]`, session capped by SessionNotOnOrAfter.

## Out of scope

- Code exchange (17.13).

## Design notes

- An email change at the IdP for a linked user changes nothing silently (audited).

## Acceptance criteria

- [ ] GRACE, REVOKED or unverified domains never link or provision.
- [ ] A user of another tenant with the same email is never linked.
- [ ] JIT never grants an admin role.
- [ ] Tokens carry the organisation and `amr` saml.

## Tests

- Unit tests per rule; BDD across two tenants and domain states.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
