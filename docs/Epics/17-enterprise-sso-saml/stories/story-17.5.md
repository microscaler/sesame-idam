# Story 17.5 — Daily re-verification job and notifications

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** A. Domains · **Depends on:** 17.4 · **Size:** M  
> **Status:** Ready

## Goal

Re-check every VERIFIED and GRACE claim daily and every PENDING claim with backoff, move claims through GRACE to REVOKED when the record disappears, and tell the organisation.

## Context

[DESIGN-org-domain-verification.md §5, §7](../../../DESIGN-org-domain-verification.md); tenant design §5.1.

## Scope

- Binary `sesame_domain_reverify` in the suite: selects due claims (jittered daily, PENDING with backoff, never faster than the record TTL), resolves (17.2), applies transitions (17.3), exits.
- Chart support for `workload.kind: cronjob` (schedule, concurrencyPolicy Forbid, no Service), used for this job.
- Notifications on VERIFIED, GRACE, REVOKED, EXPIRED: email to the organisation's admins (and tenant admins on REVOKED) through the existing mail path; audit events always.
- Metrics: claims by state, checks, failures by verdict.

## Out of scope

- Alert routing beyond email (later).

## Design notes

- Idempotent: a re-run after a crash produces no duplicate transitions (state plus `last_checked_at` guard); one run at a time.

## Acceptance criteria

- [ ] A VERIFIED claim whose record disappears is GRACE after the next due check and REVOKED once `grace_until` passes; restoring the record during grace returns it to VERIFIED.
- [ ] A PENDING claim not verified within 7 days becomes EXPIRED.
- [ ] Two concurrent runs cannot both transition a claim.
- [ ] Each transition sends exactly one notification and one audit event.

## Tests

- Job tests under a simulated clock with a mock resolver; chart render tests for the cronjob kind.

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
