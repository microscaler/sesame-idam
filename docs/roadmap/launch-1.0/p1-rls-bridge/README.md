# P1 — The RLS Bridge

**Target:** Launch 1.0 GA

**Outcome:** validated Sesame identity context becomes transaction-local PostgreSQL context,
so tenant isolation is enforced even when application queries omit tenant predicates.

## Scope and dependencies

P1 depends on P0 claims validation and Lifeguard transaction hooks. It delivers versioned SQL,
`SesameExecutor`, policy examples, a zero-bleed proof suite, and a runnable demonstration.
Database portability beyond supported PostgreSQL versions and arbitrary policy generation are
out of scope.

## Functional requirements

| ID | Requirement |
|---|---|
| FR-P1-001 | A versioned, deploy-once SQL artifact MUST provide typed helpers for current user, organization, user type, roles, permissions, session, and tenant context required by supported policies. |
| FR-P1-002 | `SesameExecutor` MUST derive context only from already validated claims and inject it with transaction-local semantics before any protected application query. |
| FR-P1-003 | Missing, malformed, or conflicting required context MUST cause policies to return zero rows or reject the transaction; it MUST never broaden access. |
| FR-P1-004 | Commit, rollback, cancellation, panic, and pooled-connection reuse MUST clear the prior transaction's identity context. |
| FR-P1-005 | Reference RLS policies MUST cover organization ownership, platform/user-type separation, and role/permission checks without trusting client headers. |
| FR-P1-006 | The SQL artifact MUST support install, idempotent upgrade, version inspection, and documented rollback/forward-recovery procedures. |
| FR-P1-007 | A sample application MUST demonstrate an unqualified `SELECT` returning only rows authorized for the current organization. |

## Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-P1-001 | Tenant isolation MUST hold across concurrency, connection-pool reuse, transaction rollback, and deliberately malformed context. |
| NFR-P1-002 | Context injection overhead MUST be benchmarked separately from the protected query and remain within the accepted GA database budget. |
| NFR-P1-003 | Helper functions and policies MUST use least-privilege ownership/search-path settings and MUST resist SQL injection or caller-controlled identifiers. |
| NFR-P1-004 | The compatibility matrix MUST name supported PostgreSQL and Lifeguard/BRRTRouter versions. |
| NFR-P1-005 | Integration failures MUST identify migration/context/policy categories without logging claim payloads or PII. |

## Acceptance criteria

| ID | Observable evidence |
|---|---|
| AC-P1-001 | Given two organizations with interleaved rows, the same unqualified query returns only the caller's rows for each organization. |
| AC-P1-002 | Given a pooled connection previously used by another organization, a new transaction cannot observe any prior context or rows. |
| AC-P1-003 | Given absent, malformed, forged-header, or tenant-conflicting context, the query returns zero rows or an explicit authorization error. |
| AC-P1-004 | Property/concurrency tests run repeated tenant switches, commit, rollback, cancellation, and error paths with zero cross-tenant observations. |
| AC-P1-005 | Clean install, repeated install, supported upgrade, and recovery procedures pass against every PostgreSQL version in the compatibility matrix. |
| AC-P1-006 | A new sample consumer follows the published guide and demonstrates login-to-RLS enforcement without application authorization predicates. |

## Exit evidence

- Publish SQL artifact checksums/version, compatibility matrix, benchmark, threat model, and
  zero-bleed test report.
- Link the sample application and a reproducible command sequence for the demonstration.
- Record an independent security review of context derivation, connection reuse, and helper
  privileges.
