# Story 17.9 — IdP metadata intake, refresh and certificate rollover

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** C. Connections · **Depends on:** 17.6, 17.8 · **Size:** M  
> **Status:** Ready

## Goal

Accept IdP metadata as XML or an HTTPS URL, parse it through the verifier, keep it fresh, and roll certificates over without breaking sign-in.

## Context

[DESIGN-saml-sp.md §7](../../../DESIGN-saml-sp.md).

## Scope

- `set_saml_idp_metadata`: XML (≤ 1 MiB) or URL (HTTPS only, 10 s, no redirect to another host, ≤ 1 MiB) fetched by org-mgmt; parsed by `/metadata/parse`; stored.
- Daily refresh for URL connections (in the 17.5 job or its own schedule): add new certificates, drop removed ones after 7 days' overlap; entity ID or SSO host changes are not applied automatically (audited, admin confirms).
- Alerts to org admins for certificates expiring within 30 days and for unconfirmed changes.
- Response shows the values the IdP admin needs (SP entity ID, ACS URL, NameID guidance, attribute names).

## Out of scope

- Signed metadata verification (later).

## Design notes

- The verifier never fetches anything; org-mgmt never parses XML itself.

## Acceptance criteria

- [ ] Metadata from each of the three IdPs (fixtures) yields entity ID, SSO URL and certificates.
- [ ] A refresh that adds a certificate keeps the old one for 7 days; one that changes the entity ID changes nothing until confirmed.
- [ ] Non-HTTPS URLs, redirects to other hosts and oversize documents are refused.

## Tests

- Fixtures per IdP; refresh under a simulated clock; mock HTTP server.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
