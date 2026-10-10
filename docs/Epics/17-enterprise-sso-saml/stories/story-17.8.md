# Story 17.8 — SAML connection schema and organisation-scoped API

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** C. Connections · **Depends on:** 17.3 · **Size:** M  
> **Status:** Ready

## Goal

Make SAML connections real data with a lifecycle, managed through organisation-scoped endpoints, replacing the stubs.

## Context

[DESIGN-saml-sp.md §4 (org-mgmt), §7, §8](../../../DESIGN-saml-sp.md).

## Scope

- Migration: `saml_connections` columns (tenant_id, status, idp_entity_id, idp_sso_url, idp_signing_certs, metadata_xml, metadata_refreshed_at, email_attribute, jit_provisioning, default_role, tested_at), one connection per organisation, RLS.
- OpenAPI and handlers: `get_saml`, `delete_saml`, `allow_org_saml`, `disallow_org_saml`, `enable_saml`, `create_saml_link`, `set_saml_idp_metadata` under `/organizations/{org_id}/sso/saml…`; unscoped `/sso/saml/*` removed.
- Status transitions (allowed → configured → tested → enabled; disabled) enforced server side; `enable_saml` only from tested.
- Setup link: one-time, 7 days, scoped to `set_saml_idp_metadata` and read-only connection details for that organisation.
- Un-ignore `saml_track_a_contract` (org-mgmt half).

## Out of scope

- Metadata parsing (17.9)
- sign-in (17.10+)

## Design notes

- Default role validated against the organisation's roles and never an admin role.

## Acceptance criteria

- [ ] The org-mgmt SAML contract test passes and gates.
- [ ] A connection cannot be enabled without a successful test sign-in; a disallowed organisation's connection refuses every operation but get and delete.
- [ ] Cross-tenant and cross-organisation access returns 404; tenant header ignored.
- [ ] Every change emits an audit event.

## Tests

- Handler tests, BDD for authorisation and lifecycle, contract test.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
