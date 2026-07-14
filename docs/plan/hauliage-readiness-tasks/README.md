# Hauliage readiness completion task list

Date: 2026-07-14
Status: active
Parent: [Hauliage readiness plan](../hauliage-readiness-plan.md)

## Completion outcome

Initial Hauliage test users can complete the delivered identity journeys, every protected service
uses validated claims, and one production-shaped east-west/database path proves transaction-local
PostgreSQL RLS with no connection-pool identity leakage. This list closes delivered-code gaps; it
does not add the deferred SDK, hosted UI, enterprise identity, or curl-like HTTP-client roadmap.

## Track A — trusted request context

- [ ] **A1 Remove unverified JWT payload fallback in Hauliage company.** Tenant, user, and
  organization selection must consume only BRRTRouter-populated `jwt_claims`.
  - Acceptance: a bearer token with a syntactically valid but unverified payload supplies no
    principal or organization; validated claims still resolve both.
- [ ] **A2 Inventory protected Hauliage entry points.** Classify each as validated local claims,
  public, service credential, or explicit development-only route.
  - Acceptance: no production controller derives tenant/org/user from headers, environment
    fallback, or an undecoded/unchecked bearer payload.

## Track B — transaction-local RLS (H1.5)

- [x] **B1 Freeze the RLS context contract.** Required: tenant, subject, active organization,
  session, roles, permissions, and optional organization/user type. Context originates only from
  cryptographically validated claims; request headers may only be cross-checked.
  - Acceptance: missing/malformed required claims and tenant conflicts return a typed error before
    a protected query starts; logs contain field names/categories, never token payloads.
  - Evidence: Hauliage's typed adapter requires text `tenant_id`, UUID `sub = user_id`, UUID
    `org_id`, `sid`, and namespaced tenant/roles/permissions; its unit suite covers missing,
    malformed, conflicting, and non-echoing error cases.
- [x] **B2 Publish versioned helper SQL.** Provide idempotent install/version functions and typed
  `sesame_current_*` accessors using a locked `search_path` and least-privilege grants.
  - Acceptance: context uses `set_config(..., true)`/`SET LOCAL`; helpers return `NULL` when unset;
    caller-controlled identifiers are never interpolated into SQL.
- [x] **B3 Complete first-class RLS on Lifeguard's base executors.** Do not add a
  Sesame-specific executor wrapper. `MayPostgresExecutor` and `PooledLifeExecutor` use
  `Option<SessionContext>` as the RLS toggle; add the missing base API that pins one primary pool
  slot, begins a contextual transaction, executes all ORM work on that connection, then
  commits/rolls back.
  - Acceptance: no protected query can run before injection; injection failure rolls back and
    fails closed; `Drop`/error paths do not release a live transaction; non-RLS callers do not
    acquire a new wrapper type or pay contextual transaction overhead.
- [x] **B4 Add reference policies.** Cover tenant ownership, active organization ownership, and a
  role/permission example on a production-shaped Hauliage table.
  - Acceptance: an unqualified `SELECT` returns only authorized rows; insert/update with a
    mismatched tenant or organization is rejected by `WITH CHECK`.
- [x] **B5 Wire the first Hauliage path.** Use the base executor capability in a representative BFF → backend →
  PostgreSQL journey; retain application predicates only as defense in depth during migration.
  - Acceptance: removing the application tenant predicate in the test fixture does not broaden
    database results.
  - Evidence: all five delivered Company reads/writes of `organization_profiles` run in
    `LifeguardPool::with_session_transaction`; Hauliage installs the Sesame v1 contract and a
    forced tenant/organization policy. The executable acceptance query intentionally has no
    organization predicate, sees one row, rejects cross-organization insert, and sees zero rows
    without context in the next transaction.
- [ ] **B6 Zero-bleed proof suite.** Exercise two tenants/organizations, concurrency, commit,
  rollback, injected error, missing context, forged header, and repeated pool-slot reuse.
  - Acceptance: zero cross-tenant observations over the repeat matrix; context is unset after the
    transaction and a context-free query cannot see protected rows.
  - Current evidence: Lifeguard covers commit, returned error, panic, missing helper, and pool-slot
    reuse; Hauliage covers two rows, unqualified read, cross-org write rejection, and next-
    transaction reset. The concurrent two-context repeat matrix remains open.

## Track C — east-west client completion

- [ ] **C1 Pool BFF → Sesame typed calls.** Replace fresh connection per request with the
  host-keyed `may_minihttp` pool; preserve per-origin isolation, bounded body, and deadlines.
- [ ] **C2 Measure actual calls.** Capture cold/warm login, active-org, membership/invite, JWKS
  cold start, scheduled refresh, unknown-kid refresh, last-good outage, and Redis outage timing.
- [ ] **C3 Record transport inventory.** Edge ALPN is evidence only for client → HAProxy. Current
  BFF → Sesame and BFF → Hauliage HTTP:8080 hops are HTTP/1.1 with ALPN not applicable.

Acceptance for C1–C3: zero request errors in the bounded test matrix; connection creation/reuse,
pool wait, latency percentiles, response sizes, timeout behavior, and hop protocol are recorded
without credentials or tokens.

## Track D — readiness evidence

- [ ] **D1 Complete the existing H1.6 database integration target** and include the RLS suite.
- [ ] **D2 Run real cross-repo E2E:** login/register → active org → protected Hauliage path →
  refresh → logout → denylisted access token rejected.
- [ ] **D3 Run no-retry quality gates** for Sesame, BRRTRouter, Hauliage, and the live shared stack.
- [ ] **D4 Record the go/no-go bundle:** exact commits/images, commands/output, reset/reseed steps,
  known limits, rollback, and owner.

The milestone is complete only when A1–D4 have linked evidence. A design document, compiled-only
executor, manually filtered query, or edge-only HTTP/2 observation does not satisfy completion.
