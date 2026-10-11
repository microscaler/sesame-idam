# Story 17.3 — Domain claim schema and lifecycle

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** A. Domains · **Depends on:** 17.1 · **Size:** M  
> **Status:** Done (2026-10-11)

## Goal

Persist organisation domain claims with their state and history, and make every state change go through one transition function.

## Context

[DESIGN-org-domain-verification.md §5, §8](../../../DESIGN-org-domain-verification.md).

## Scope

- Migration for `org_domains` (tenant_id, scope, state, epoch, dnssec, pending_until, verified_at, last_checked_at, last_verified_at, grace_until, auto_join, created_by) and `org_domain_events` (append-only); backfill tenant_id; every existing row to PENDING (the old flag was never proven); drop `verified`.
- Partial unique index: one VERIFIED/GRACE holder per (tenant_id, domain).
- Tenant RLS on both tables (ADR-005); Lifeguard models.
- `domain_lifecycle::apply(claim, event, now) -> (claim', audit_event)` for every transition in the design's state diagram, including CONFLICT.
- Queries the consumers need: `verified_domain_for(tenant, org, email_domain)` honouring scope, `discoverable_domain(tenant, email_domain)` (VERIFIED or GRACE).

## Out of scope

- API (17.4)
- DNS (17.2)

## Design notes

- The transition function is pure; the repository writes the claim and its event in one transaction; the unique index is the final arbiter of conflicts (a losing verification becomes CONFLICT).

## Acceptance criteria

- [x] The migration applies from scratch and re-runs (sesame-migrate) as a non-superuser; existing rows end PENDING.
- [x] Two organisations of one tenant cannot both hold a domain in VERIFIED or GRACE; two tenants can.
- [x] Property test over random event sequences: VERIFIED is only reachable through a quorum-match event; GRACE never satisfies `verified_domain_for`; REVOKED satisfies neither query.
- [x] `scope = Exact` matches only the domain itself; `Subdomains` matches the domain and its descendants, never a sibling.
- [x] Every transition writes exactly one event; events cannot be updated or deleted by the service login.

## Tests

- Lifecycle unit and property tests; repository tests against Postgres (RLS on, two tenants).

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.

## Evidence (2026-10-11)

- `sesame_common::domain_verify::lifecycle`: the transition function, unit tests and a property test over random event
  and time sequences (VERIFIED only through a quorum match, GRACE never vouches, REVOKED neither vouches nor is
  discoverable, final states refuse every event, one history event per applied event).
- Migrations `org-mgmt/20261011120000_org_domains.sql` (columns, backfill, PENDING for every old row, `verified` dropped,
  checks, the one-holder partial unique index), `org-mgmt/20261011120001_org_domain_events.sql` (append-only trigger,
  cascade allowed), `rls/20261011120002_org_domain_claims_rls.sql` (tenant RLS, FORCE; events have no UPDATE or DELETE
  policy). Applied from scratch and over the previous schema with data, then re-applied, as a non-superuser migrator
  login acting as its owner group (`sesame-migrate`); the Lifeguard migrator reports both tables identical to the models.
- `org-mgmt::services::domain_claims`: create, load, list, `apply_event` (claim and history in the caller's tenant
  transaction), `record_verdict` (a lost one-holder race becomes CONFLICT), `verified_domain_for`, `discoverable_domain`.
  Postgres tests (`tests/bdd/domain_claims.rs`): one holder per tenant, two tenants, RLS isolation, scope, GRACE and
  REVOKED in the queries, history not updatable or deletable by the service login.
- `dnssec` is `NOT NULL DEFAULT false` rather than nullable: Lifeguard binds a NULL `Option<bool>` with the wrong type on
  insert (to be fixed in Lifeguard).
