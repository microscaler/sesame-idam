---
title: RLS Bridge
status: verified
updated: 2026-07-14
sources: [sql/rls/v1/install.sql, docs/ADR-005-first-class-rls-contract.md, ../lifeguard/src/executor.rs, ../lifeguard/src/pool/pooled.rs]
---

# RLS Bridge (Row-Level Security)

## Overview

Sesame provides SQL helper functions that inject RLS context into PostgreSQL sessions. The application never stores secrets in the database — no JWT ever enters the database.

## How It Works

1. Application middleware validates Sesame JWT (signatures, expiry)
2. The consumer maps validated claims to Lifeguard `SessionContext`
3. A base Lifeguard executor starts a transaction and calls `public.rls_set_session(...)`
4. Sesame helper functions set transaction-local `sesame.*` variables
5. Application ORM queries are scoped by PostgreSQL policies on the same connection

## Key Functions

- `rls_set_session(...)` — Lifeguard's fail-closed injection entry point
- `sesame_rls_contract_version()` — Return the installed contract version
- `sesame_current_tenant_id()` — Return the opaque hard tenant boundary as text
- `sesame_current_subject_id()` — Return the authenticated subject
- `sesame_current_organization_id()` — Return the active organization
- `sesame_current_session_id()` — Return the authenticated session
- `sesame_current_roles()` / `sesame_current_permissions()` — Return JSON arrays
- `sesame_has_role(...)` / `sesame_has_permission(...)` — Policy predicates

Every accessor returns `NULL` when no context exists. Policy expressions therefore fail closed.

## Security Guarantees

1. **No JWT in DB.** The JWT never leaves the application. RLS context is set via `SET LOCAL` session variables.
2. **Database-level enforcement.** RLS policies enforce both tenant and active-organization ownership.
3. **Transaction-scoped.** `SET LOCAL` only affects the current transaction — no cross-session leakage.
4. **Least privilege.** Helpers are `SECURITY INVOKER`, lock `search_path`, and revoke `PUBLIC` execution.
5. **One executor model.** There is no `SesameExecutor`; RLS is an optional capability of Lifeguard's base executors.

## Code Anchors

- `sql/rls/v1/install.sql` — Canonical install and accessor contract
- `sql/rls/v1/reference-hauliage.sql` — Executable reference policy
- `docs/ADR-005-first-class-rls-contract.md` — Ownership and abstraction decision
- `../lifeguard/src/executor.rs` — Direct/base executor context support
- `../lifeguard/src/pool/pooled.rs` — Pooled one-shot and pinned transaction support

## Delivered Evidence

- Hauliage's Company service maps only BRRTRouter-validated claims into `SessionContext` and runs
  every delivered `organization_profiles` read/write in `with_session_transaction`.
- The installed Company policy forces RLS on `organization_profiles`; the application query used
  by the acceptance suite has no organization predicate.
- Lifeguard's live PostgreSQL suite covers commit, returned error, panic, missing helper, repeated
  pool reuse, and two concurrent organization contexts with 100 interleaved reads each.
- The 2026-07-14 shared-stack run proved real Sesame login/JWKS → Hauliage BFF → Company → forced
  RLS for both seeded organizations: the shipper saw only AME Corp and the transporter saw only
  Transport Services.

## Gaps / Drift

> **Open:** The delivered Hauliage policy is the first production-shaped slice, not general policy
> generation or the complete Launch 1.0 compatibility/benchmark/recovery evidence set.
