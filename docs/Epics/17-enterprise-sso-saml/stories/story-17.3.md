# Story 17.3 — Domain claim schema and lifecycle

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** A. Domains · **Depends on:** 17.1 · **Size:** M  
> **Status:** Ready

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

- [ ] The migration applies from scratch and re-runs (sesame-migrate) as a non-superuser; existing rows end PENDING.
- [ ] Two organisations of one tenant cannot both hold a domain in VERIFIED or GRACE; two tenants can.
- [ ] Property test over random event sequences: VERIFIED is only reachable through a quorum-match event; GRACE never satisfies `verified_domain_for`; REVOKED satisfies neither query.
- [ ] `scope = Exact` matches only the domain itself; `Subdomains` matches the domain and its descendants, never a sibling.
- [ ] Every transition writes exactly one event; events cannot be updated or deleted by the service login.

## Tests

- Lifecycle unit and property tests; repository tests against Postgres (RLS on, two tenants).

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
