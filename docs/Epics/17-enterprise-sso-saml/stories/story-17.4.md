# Story 17.4 — Organisation domain API

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** A. Domains · **Depends on:** 17.2, 17.3 · **Size:** M  
> **Status:** Done (2026-10-11)

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

- Tokens through the configured keyring (Cloud KMS HSM in deployments; 17.1), the claim's id in the MAC input; a keyring failure answers 503 and leaves the claim unchanged; the response shows the record name and value; nothing in any response reveals another organisation's claim.

## Acceptance criteria

- [x] No request body or path can set a claim's state.
- [x] A member who is not an org admin cannot create, verify, update or delete; a user of another tenant gets 404.
- [x] Creating `gmail.com` or a public suffix returns `domain_unavailable`; the same response as a domain held by another organisation.
- [x] Publishing the shown record and calling verify moves the claim to VERIFIED (mock `dns-probe` in tests); a record for another organisation does not.
- [x] `update_org_domains` is gone from the spec and the code; the contract test for org-mgmt domain operations passes.

## Tests

- Handler unit tests with a mock `dns-probe`; BDD for authorisation and isolation; OpenAPI lint.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.

## Evidence (2026-10-11)

- OpenAPI (org-mgmt, tag `Domains`): `list_org_domains`, `create_org_domain`, `verify_org_domain`, `update_org_domain`,
  `delete_org_domain`, and the tenant console's `revoke_org_domain` (`POST /tenants/current/org-domains/{domain_id}/revoke`,
  fixed reason codes). `update_org_domains` and `OrgDomainsRequest` are gone from the spec, the gen crate and the impl.
  Request schemas are closed (`additionalProperties: false`) and none has a state field. BearerAuth only; the tenant
  comes from the validated token, never `X-Tenant-ID`.
- `org-mgmt::services::domain_api`: the operations, independent of HTTP, over the 17.3 service layer, each in one
  tenant RLS transaction that is rolled back when the operation fails. Members list; owners and admins create, verify,
  update and delete; a non-member, an organisation of another tenant or another organisation's claim is `not_found`.
  The record is derived before the claim is stored (the claim id is in the MAC input), so a key failure leaves no claim;
  `verify` asks `dns-probe` (`DnsProbe` trait, HTTP client through `sesame_common::http`), applies `quorum`, then the
  lifecycle; a key or probe failure answers 503 with the claim unchanged; one check per claim per minute
  (`last_checked_at`, `retry_after` in the body); a lost race is `domain_unavailable` with the claim CONFLICT.
  Ineligible domains and domains held by another organisation of the tenant get the same 422 `domain_unavailable`.
  `list` shows the record to keep published for every claim that is not final.
- Configuration: `SESAME_DOMAIN_VERIFY_KEYRING` (unset: create and verify answer 503, the rest of org-mgmt works),
  `SESAME_DNS_PROBE_URL` (default `http://dns-probe:8080/dns-probe/v1`), `SESAME_DOMAIN_RESERVED` (the deployment's own
  domains), `SESAME_DOMAIN_EXTRA_PROVIDERS`. Wiring them in the chart is story 17.17.
- Tests: `tests/bdd/domain_api.rs` against Postgres 16 with RLS, as a non-owner login, and a mock `dns-probe`: claim →
  publish → verify → auto-join → list → delete; authorisation and isolation (member, non-member, other tenant, other
  organisation's claim, tenant admin of another tenant); `gmail.com`, `co.uk`, `com` and a reserved domain answer exactly
  as a held domain; another organisation's record does not verify; the race loser becomes CONFLICT; a missing or failing
  key and a probe outage change nothing; the spec has no state-setting request field and no `update_org_domains`.
  Unit tests for error mapping and the tenant-admin check. `cargo test -p sesame_idam_org_mgmt` (49 BDD, all green),
  clippy clean for the touched code, `brrtrouter-gen lint` clean.
- Decisions: `claim_exists` (409) when the organisation already has an open claim on the domain (one open claim per
  organisation and domain, whatever the scope); deleting a final claim is a no-op 204; auto-join on a final claim is
  `not_verified`.
