# ADR-005: First-class RLS on Lifeguard base executors

Date: 2026-07-14  
Status: accepted

## Context

Sesame and PostgreSQL RLS are intended to be normal Microscaler platform capabilities. Earlier
design documents proposed a `SesameExecutor` wrapper around Lifeguard. That adds a second executor
hierarchy, makes ORM APIs harder to compose, and creates an attractive place for identity-specific
behaviour to diverge from connection-pool safety.

The database needs only a small provider-neutral contract: validated identity values must be set
transaction-locally on the same connection that runs protected statements. Sesame owns the claim
semantics and SQL contract. Lifeguard owns connection and transaction lifecycle.

## Decision

1. Do not introduce `SesameExecutor`.
2. `MayPostgresExecutor` and `PooledLifeExecutor` remain the normal execution APIs.
3. `Option<SessionContext>` is the single-statement RLS toggle. `None` retains the original
   autocommit path. `Some(context)` injects context before the statement and fails closed.
4. `LifeguardPool::with_session_transaction` is the multi-statement path. It pins the existing
   `ExclusivePrimaryLifeExecutor`, begins a transaction, injects context, runs the closure, and
   commits or rolls back before releasing the pool slot.
5. The v1 context requires the opaque tenant ID (`text`), subject UUID, active organization UUID,
   session ID, roles, and permissions. User and organization classifications are optional. The
   database never receives a JWT or an unvalidated bearer payload.
6. Sesame publishes the versioned SQL contract at `sql/rls/v1/install.sql`. The Lifeguard entry
   point remains `public.rls_set_session(...)`; policy authors use typed `sesame_current_*`
   accessors.
7. Helper functions are `SECURITY INVOKER`, lock `search_path`, use transaction-local
   `set_config(..., true)`, and revoke default `PUBLIC` execution. Runtime grants are explicit.

## Raw SQL exception

Sesame normally declares schema through Lifeguard entities. PostgreSQL functions, RLS policies,
`ENABLE/FORCE ROW LEVEL SECURITY`, function privileges, and function-level `search_path` are not
representable by that entity model. The versioned files under `sql/rls/` are the narrow exception;
application tables and ordinary schema changes continue to use Lifeguard models and the migrator.

## Consequences

- Consumers use the same executor and ORM traits whether RLS is enabled or not.
- Context-free work has no additional wrapper or transaction cost.
- Protected multi-statement work cannot migrate between pool connections.
- Sesame claim parsing remains outside Lifeguard; Lifeguard has no JWT or BRRTRouter dependency.
- Existing design references to `SesameExecutor`, `auth.*` GUCs, or session-wide `SET` are
  superseded by this ADR and the v1 SQL contract.

## Verification

The Lifeguard RLS integration suite covers direct and pooled execution, multi-statement pinning,
missing helper failure, tenant switching, commit, rollback, application error, panic, and
context-free reuse of the same worker slot.
