# Design Doc: PostgreSQL RLS Integration for Sesame-IDAM — Stage 1

> **Status:** Draft — First Stage
> **Inspired by:** Supabase's approach, adapted for Sesame's bolt-on architecture and 3-persona B2B model
> **The JWT never enters the database — but its claims do, via session variables.**
> **Date:** 2026-01-04

---

## 1. Context & Problem Statement

Sesame-IDAM issues enriched JWTs containing `user_id`, `org_id`, `user_type`, `org_type`, roles, and permissions. The consuming application uses these JWTs for auth. But the application also has its own PostgreSQL database with rows that need to be scoped to the current user's organisation.

The consuming application exists in a **3-persona B2B SaaS model**:

| Persona | Role | Example |
|---|---|---|
| **Platform** | SaaS operator | Sesame-IDAM itself |
| **Service Provider** | Delivers services through the platform | Employment agency, transporter, broker |
| **Service Consumer** | Consumes services | Employing company, shipper, buyer |

Each persona has different data access expectations:

- **Platform users** see all rows across all tables and all organisations
- **Provider users** see their own org's data + data shared with/consumed by their partner consumer orgs
- **Consumer users** see their own org's data + data assigned/shared by their provider

**Supabase solves this by** providing SQL functions that parse the JWT from the HTTP request header and set PostgreSQL session variables, which RLS policies then reference.

**Sesame's challenge is** that it's a bolt-on IDAM — the consuming application has its own database, its own application server, and Sesame doesn't control the auth middleware. The JWT signature verification must happen in the application layer (where we have RS256 public keys), not in Postgres (where we can't do crypto without extensions).

Additionally, the **3-persona model introduces a second dimension of scoping**: org_id alone is insufficient when provider and consumer orgs share data. We need `org_type` as a trusted JWT claim, flowing downstream to enable org_type-aware RLS policies.

**The solution:** Sesame provides SQL helper functions that the application calls AFTER validating the JWT. These helpers set session-scoped variables (`SET LOCAL`) so RLS policies can filter rows transparently — including by both `org_id` and `org_type`.

---

## 2. org_type Trust Boundary

### 2.1 org_type must never appear in URIs

`org_type` is a server-side classification. It must never be part of the request path (`/provider-orgs/`, `/consumer-orgs/`) because URIs are client-controlled and mutable. A malicious consumer could reclassify itself by changing the URL.

### 2.2 org_type flows via JWT claim

org_type is authoritative in the platform service. When a user authenticates, the platform service injects `org_type` into the JWT:

```json
{
  "user_id": "abc-123",
  "org_id": "def-456",
  "org_type": "provider",
  "user_type": "customer",
  "roles": ["admin"],
  "permissions": ["invoices:write"]
}
```

This is the same trust model as `user_type`: written by the platform service, read by downstream services, immutable by consumers.

### 2.3 org_type in the database

The JWT claim becomes a PostgreSQL session variable via `SET LOCAL`, accessible to RLS policies through helper functions. The database never receives the raw JWT string — only the extracted, verified claim values.

```
Platform Service (writes org_type)
    → JWT issued to user
        → Downstream app reads org_type from validated JWT
            → App calls sesame_set_session(..., org_type, ...)
                → SET LOCAL auth.user_org_type = 'provider'
                    → RLS policy reads via sesame_current_user_org_type()
```

---

## 3. The Access Matrix & RLS Implications

With 3 personas, org_id alone is insufficient. Consider a table `public.shipments` that contains records shared across provider-consumer relationships:

| User Persona | org_id | org_type | Should see rows where |
|---|---|---|---|
| Platform | platform-org | platform | `org_id IS NOT NULL` (all rows) |
| Provider | provider-A | provider | `org_id = provider-A` OR `org_id IN (consumer-orgs of provider-A)` |
| Consumer | consumer-B | consumer | `org_id = consumer-B` OR `shipment.org_id = consumer-B` |

**Implication for RLS:** Policies cannot rely on a single `org_id` filter. They need org_type to determine which scoping rules apply.

### 3.1 Policy Complexity Levels

| Level | Complexity | When to use |
|---|---|---|
| **L1: org_id-only** | Simple: `org_id = sesame_current_user_org_id()` | Apps with strict org silos, no cross-org sharing |
| **L2: org_type + org_id** | Moderate: depends on org_type, may join lookup tables | Apps with provider↔consumer relationships, platform admin full access |
| **L3: org_type + org_id + dynamic** | Complex: org_type-aware policies with multiple lookup tables, multi-org joins | Apps with many-to-many org relationships, hierarchical data, platform tenant isolation |

Stage 1 focuses on **L2** — org_type-aware policies that handle the common provider↔consumer model.

---

## 4. Sesame's RLS Architecture

### 4.1 The Three-Layer Model

```mermaid
flowchart TB
    subgraph L1[\"Layer 1: Application Server\"]
        L1A[\"Receives request\\nwith Authorization: Bearer ***\"]
        L1B[\"Validates JWT\\n(signature, expiry)\"]
        L1C[\"Extracts claims:\\nuser_id, org_id, org_type,\\nuser_type, permissions\"]
        L1D[\"Calls Sesame RLS helper\\nto set session variables\"]
        L1E[\"Executes database query\"]
        L1A --> L1B --> L1C --> L1D --> L1E
    end

    SET_VARS[\"SET LOCAL auth.user_id = 'uuid'\\nSET LOCAL auth.user_org_id = 'uuid'\\nSET LOCAL auth.user_org_type = 'provider'\\nSET LOCAL auth.user_type = 'customer'\\nSET LOCAL auth.permissions = '{invoices:write,...}'\"]

    subgraph L2[\"Layer 2: PostgreSQL Helper Functions\\n(Sesame-provided SQL)\"]
        L2A[\"SET LOCAL is session-scoped\\n— lasts for the duration of\\nthe current transaction\"]
        L2B[\"RLS policies read via:\\ncurrent_setting('auth.user_org_id', true)\\ncurrent_setting('auth.user_org_type', true)\"]
        L2C[\"The 'RLS Bridge'\\n— one line of SQL needed\\nbefore every query\"]
        L2A --> L2B --> L2C
    end

    subgraph L3[\"Layer 3: PostgreSQL RLS Policies\"]
        L3A[\"Defined on each\\norg-scoped table\"]
        L3B[\"Automatically filter rows\\nbased on session variables\"]
        L3C[\"Platform users bypass\\norg-scoped policies\"]
        L3D[\"org_type determines\\nwhich policy variant applies\"]
        L3A --> L3B --> L3C
        L3B --> L3D
    end

    L1E --> SET_VARS
    SET_VARS --> L2
    L2B -.->|\"USING (org_id =\\ncurrent_setting('auth.user_org_id'))\"| L3
    L2B -.->|\"AND user_org_type =\\ncurrent_setting('auth.user_org_type')\"| L3
```

### 4.2 Why `SET LOCAL` Instead of `SET`

`SET` persists for the entire session/connection. `SET LOCAL` is scoped to the current transaction only. This is critical because:

- **Connection pooling:** If we used `SET`, a leaked session variable from one user would be visible to the next user reusing the same DB connection.
- **Multiple orgs per request:** Some requests might need to query across orgs (admin operations). `SET LOCAL` ensures the scoping is isolated per transaction.
- **Automatic cleanup:** No need for explicit `RESET` or `SET` back to empty — Postgres handles it.

### 4.3 Transaction Boundary

```mermaid
flowchart TD
    START([Transaction START])

    SET1[\"SET LOCAL\\nauth.user_id = '...'\"]
    SET2[\"SET LOCAL\\nauth.user_org_id = '...'\"]
    SET3[\"SET LOCAL\\nauth.user_org_type = '...'\"]
    SET4[\"SET LOCAL\\nauth.user_type = '...'\"]
    SET5[\"SET LOCAL\\nauth.permissions = '{...}'\"]
    QUERY[\"Database queries\\nRLS policies fire\"]
    END_T([Transaction END])
    CLEAR[All SET LOCAL\\nvariables auto-cleared]

    START --> SET1 --> SET2 --> SET3 --> SET4 --> SET5 --> QUERY --> END_T --> CLEAR
```

---

## 5. Sesame's SQL Helpers (Extended with org_type)

These are the exact SQL functions Sesame will generate and deploy into the consuming application's database.

### 5.1 Core Session Variables

```sql
-- =============================================================================
-- Sesame-IDAM: RLS Helper Functions — Stage 1
-- Deploy once per consuming application's database.
--
-- These functions read session variables set by the application server
-- BEFORE each database query. They do NOT validate JWTs — that happens
-- in the application layer.
-- =============================================================================

-- Set all RLS session variables from decoded JWT claims.
-- Called by the application AFTER validating the JWT.
-- org_type is the 4th parameter (added in Stage 1 for 3-persona B2B support).
CREATE OR REPLACE FUNCTION public.sesame_set_session(
    p_user_id        uuid,
    p_user_org_id    uuid,
    p_user_org_type  text DEFAULT 'consumer',
    p_user_type      text DEFAULT 'customer',
    p_permissions    text[] DEFAULT '{}',
    p_user_email     text DEFAULT NULL
)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER  -- Runs with the privilege of the function owner
AS $$             -- because we validate inputs, not trust the caller
BEGIN
    SET LOCAL auth.user_id        := p_user_id;
    SET LOCAL auth.user_org_id    := p_user_org_id;
    SET LOCAL auth.user_org_type  := p_user_org_type;
    SET LOCAL auth.user_type      := p_user_type;
    SET LOCAL auth.permissions    := p_permissions;
    SET LOCAL auth.user_email     := p_user_email;
END;
$$;

-- Get the current user's ID from session (null if not set)
CREATE OR REPLACE FUNCTION public.sesame_current_user_id()
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
    SELECT NULLIF(current_setting('auth.user_id', true), '');
$$;

-- Get the current user's org ID from session
CREATE OR REPLACE FUNCTION public.sesame_current_user_org_id()
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
    SELECT NULLIF(current_setting('auth.user_org_id', true), '');
$$;

-- Get the current user's org type from session  (NEW in Stage 1)
CREATE OR REPLACE FUNCTION public.sesame_current_user_org_type()
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT NULLIF(current_setting('auth.user_org_type', true), '');
$$;

-- Get the current user type from session
CREATE OR REPLACE FUNCTION public.sesame_current_user_type()
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT NULLIF(current_setting('auth.user_type', true), '');
$$;

-- Get the current user's permissions as an array
CREATE OR REPLACE FUNCTION public.sesame_current_permissions()
RETURNS text[]
LANGUAGE sql
STABLE
AS $$
    SELECT NULLIF(current_setting('auth.permissions', true), '');
$$;

-- Get the current user's email
CREATE OR REPLACE FUNCTION public.sesame_current_user_email()
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT NULLIF(current_setting('auth.user_email', true), '');
$$;
```

### 5.2 RLS Policy Templates (org_type-aware)

#### L1: org_id-only (for strict silo apps)

```sql
ALTER TABLE public.invoices ENABLE ROW LEVEL SECURITY;

-- Customer users: only see rows in their org
CREATE POLICY org_scope_customers ON public.invoices
    FOR ALL
    USING (
        org_id = COALESCE(
            sesame_current_user_org_id(),
            gen_random_uuid()  -- failsafe: match nothing if org_id not set
        )
    );

-- Platform users: see all rows
CREATE POLICY platform_all_access ON public.invoices
    FOR ALL
    USING (sesame_current_user_type() = 'platform');

-- Deny unauthenticated
CREATE POLICY deny_unauthenticated ON public.invoices
    FOR ALL
    USING (sesame_current_user_id() IS NOT NULL);
```

#### L2: org_type + org_id (provider↔consumer model)

```sql
-- Provider users: see own org rows + rows linked to their consumer orgs
CREATE POLICY provider_cross_org_access ON public.shipments
    FOR ALL
    USING (
        sesame_current_user_org_type() = 'provider'
        AND (
            org_id = sesame_current_user_org_id()
            OR org_id IN (
                SELECT consumer_org_id
                FROM public.provider_consumer_links
                WHERE provider_org_id = sesame_current_user_org_id()
            )
        )
    );

-- Consumer users: see own org rows
CREATE POLICY consumer_org_scope ON public.shipments
    FOR ALL
    USING (
        sesame_current_user_org_type() = 'consumer'
        AND org_id = COALESCE(
            sesame_current_user_org_id(),
            gen_random_uuid()
        )
    );
```

#### L2b: Platform admin bypass (unified policy)

```sql
-- Platform users bypass all org-type-specific policies
CREATE POLICY platform_all_access ON public.shipments
    FOR ALL
    USING (sesame_current_user_type() = 'platform');
```

### 5.3 Hard Boundary: No PostgREST-Style Auto-Generated API

**This is a hard architectural boundary: Sesame will NOT provide a PostgREST-style
auto-generated REST interface like Supabase.**

Supabase's RLS story is tightly coupled to PostgREST — PostgREST passes the raw
`Authorization` header to PostgreSQL, and `auth.jwt()` / `auth.uid()` parse claims
directly from the JWT string inside the database. This works because PostgREST is
the *only* route to the database.

Sesame takes the opposite approach:

- **All access to consuming-application data flows through the application server.**
  The application server is the sole gateway to the database. There is no direct
  SQL-to-REST bridge that bypasses the app layer.
- **JWT signature verification happens in the application layer** using RS256 public
  keys from Sesame's JWKS endpoint. The JWT itself never travels to PostgreSQL.
- **The application server is the RLS injection point.** After validating the JWT,
  the middleware calls `sesame_set_session()` which `SET LOCAL`s the extracted claims
  into the current database transaction.
- **RLS is defense-in-depth, not a self-service API boundary.** RLS policies exist
  because database-level security is non-negotiable, but they are not the primary
  authorization mechanism. The application layer (BRRTRouter middleware + business
  logic) is the primary authorization boundary.

This means Sesame's RLS integration is **not a drop-in "install and forget" like
Supabase** — the consuming application *must* implement the Sesame middleware
integration. This is by design: it keeps Sesame a bolt-on IDAM that
respects the consuming application's existing architecture rather than imposing a
PostgREST-like layer on top.

---

## 6. Lifeguard ORM Integration

Because Sesame controls both BRRTRouter (the web framework) and Lifeguard (the ORM),
the Sesame middleware integration is **tight, automatic, and idiomatic** — not a
manual `SELECT sesame_set_session(...)` call in every handler.

### 6.1 BRRTRouter Middleware: JWT Validation + Session Setup

BRRTRouter's middleware system (`Middleware` trait with `before`/`after` hooks) is
used to create a `SesameAuthMiddleware`:

```rust
// In the consuming application's BRRTRouter setup:

let mut service = AppService::new(router, spec);

// Sesame middleware validates JWT, extracts claims, sets DB session
service.add_middleware(SesameAuthMiddleware::new(
    JWKS_URL,              // Sesame's JWKS endpoint for key fetching
    RS256_VERIFY,          // Use RS256 with public key (NOT HMAC)
    SESSION_TIMEOUT_MS,    // Token expiry tolerance
));

// Metrics and CORS still work — middleware ordering is unchanged
service.add_middleware(MetricsMiddleware::new());
service.add_middleware(CorsMiddleware::new());
```

The middleware's `before` hook:

1. Extracts the `Authorization: Bearer` header
2. Fetches/validates the JWT signature against Sesame's JWKS (RS256)
3. Extracts claims: `user_id`, `org_id`, `org_type`, `user_type`, `permissions`, `email`
4. Injects the claims into the request context (stored in `HandlerRequest` extensions)
5. Returns `None` to continue, or a 401 `HandlerResponse` to short-circuit

### 6.2 Lifeguard ORM Enrichment: Automatic `SET LOCAL` Injection

Lifeguard's `LifeExecutor` trait abstracts database execution over `may_postgres`.
Lifeguard's `Transaction` type implements `LifeExecutor` and wraps a `may_postgres::Client`
with full transaction semantics (commit, rollback, savepoints, isolation levels).

**Enrichment strategy:** Add a `SesameExecutor` wrapper to Lifeguard that implements
`LifeExecutor` and automatically runs `SELECT sesame_set_session(...)` at the start
of each transaction, when Sesame session context is present.

```rust
// Lifeguard-enriched executor (added to lifeguard crate)

pub struct SesameExecutor<E> {
    inner: E,
    context: Arc<RwLock<Option<SesameContext>>>,
}

pub struct SesameContext {
    pub user_id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub org_type: String,      // NEW in Stage 1
    pub user_type: String,
    pub permissions: Vec<String>,
    pub email: Option<String>,
}

impl<E: LifeExecutor> LifeExecutor for SesameExecutor<E> {
    fn execute(&self, query: &str, params: &[&dyn ToSql]) -> Result<u64, LifeError> {
        if let Some(ctx) = self.context.read().unwrap().clone() {
            let session_sql = format!(
                "SELECT public.sesame_set_session({}, {}, {}, {}, {}, {})",
                ctx.user_id, ctx.org_id, ctx.org_type, ctx.user_type,
                format!("{:?}", &ctx.permissions),
                ctx.email.map(|e| format!("'{}'", e)).unwrap_or("NULL")
            );
            self.inner.execute(&session_sql, &[])?;
        }
        self.inner.execute(query, params)
    }

    fn query_one(&self, query: &str, params: &[&dyn ToSql]) -> Result<Row, LifeError> {
        // Session context is injected once per transaction via `execute` on the first call
        self.inner.query_one(query, params)
    }

    fn query_all(&self, query: &str, params: &[&dyn ToSql]) -> Result<Vec<Row>, LifeError> {
        self.inner.query_all(query, params)
    }
}
```

The consuming application creates the enriched executor once at startup:

```rust
// Application bootstrap

let client = LifeguardPool::connect("postgresql://...").await?;
let base_executor = MayPostgresExecutor::new(client);

// Wrap with Sesame session injection
let sesame_executor = SesameExecutor::new(
    base_executor,
    request_context,  // Shared context populated by SesameAuthMiddleware
);

// Now ALL Lifeguard queries go through SesameExecutor:
let shipments = Shipment::find().all(&sesame_executor)?;
// <- SesameExecutor automatically runs sesame_set_session() so RLS policies fire.
```

### 6.3 Flow: Middleware → Context → ORM → RLS

```mermaid
flowchart TB
    subgraph PIPELINE[\"BRRTRouter Request Pipeline\"]
        direction TB
        REQ[\"Request:\\nAuthorization: Bearer ***\"]
        MW[\"SesameAuthMiddleware::before()\\n  - Validate JWT (RS256, JWKS)\\n  - Extract claims: user_id, org_id,\\n    org_type, user_type, perms\\n  - Store in context\\n    RwLock<Option<SesameContext>>\"]
        HANDLER[\"Handler: list_shipments()\\n  - Shipment::find().all(&sesame_executor)\"]
        SESAME_EXEC[\"SesameExecutor::query_all()\\n  - Check context for SesameContext\\n  - Found -> execute\\n    SELECT sesame_set_session(...)\\n  - Forward query to inner executor\"]
        REQ --> MW --> HANDLER --> SESAME_EXEC
    end

    subgraph PG[\"PostgreSQL (with RLS enabled)\"]
        RLS_POL[\"RLS policy:\\nUSING (org_id =\\n  sesame_current_user_org_id())\\n  AND sesame_current_user_org_type() = 'consumer'\"]
        FILTERED[\"Returns filtered rows\"]
        RLS_POL --> FILTERED
    end

    SESAME_EXEC --> PG
```

### 6.4 Transaction Boundary Guarantee

Because Lifeguard's `Transaction` type implements `LifeExecutor`, the `SesameExecutor`
wrapper works seamlessly with Lifeguard's transaction semantics:

```rust
// Within a Lifeguard transaction:
let tx = sesame_executor.begin()?;

// All queries inside the transaction inherit the SesameContext:
// - The first query triggers sesame_set_session()
// - Subsequent queries reuse the SET LOCAL (transaction-scoped)
// - Transaction commit/rollback clears all SET LOCAL automatically
Shipments::find()
    .filter(Expr::col(Shipments::Column::Status).eq("pending"))
    .all(&tx)?;
```

### 6.5 Lifeguard-Specific Features That Work With RLS

All existing Lifeguard features continue to work unchanged:

- **Identity Map (`Session`/`ModelIdentityMap`):** Still operates on model instances;
  RLS filtering only affects which rows are loaded from the database, not the identity
  map semantics.
- **Scopes:** Scopes are composable query predicates — RLS adds an additional
  `USING` clause on top; scopes are unaffected.
- **`flush_dirty` / `flush_dirty_in_transaction`:** Dirty model tracking is unchanged;
  `SesameExecutor` transparently wraps the executor used during flush.
- **Raw SQL (`execute`, `query_one`, `query_all`):** Raw SQL queries also go through
  `SesameExecutor`, so RLS context is automatically present for raw SQL too.
- **Connection pooling (`LifeguardPool`, `PooledLifeExecutor`):** `SesameExecutor`
  wraps any `LifeExecutor` implementation, including pooled executors.

---

## 7. Open Decisions

### 7.1 Does org_type flow via JWT to downstream services?

**Proposed: Yes.** org_type is a trusted claim written by the platform service, read by downstream services. This enables org_type-aware RLS policies at the database layer (defense-in-depth). If org_type stays internal to the platform service, downstream apps cannot enforce org-type-specific access rules at the database level.

**Trade-off:** Adds one field to the JWT schema. Increases downstream service awareness of Sesame's data model slightly. But aligns with the existing `user_type` pattern.

### 7.2 Who writes the RLS CREATE POLICY statements?

**Proposed:** Sesame ships SQL helper functions (`sesame_set_session`, `sesame_current_user_org_type`, etc.) and provides policy template libraries. The consuming application's DBA/engineering team writes and applies the `CREATE POLICY` statements for their own tables.

**Rationale:** Each app has a different schema. Sesame cannot auto-generate policies for arbitrary tables. Templates + guidance is the right balance of ease-of-use and flexibility.

### 7.3 What if an app doesn't need org_type?

**Proposed:** The session variable `auth.user_org_type` has a sensible default (`'consumer'`). If the app doesn't use org_type-aware policies, the default policy L1 (org_id-only) works fine. org_type is additive, not required.

### 7.4 Should the platform service have its own RLS?

**Proposed: Yes, but separate.** The platform service's own database tables need org_type-aware RLS too (platform admin sees everything, provider org admins see their org, etc.). This is solved within Sesame-IDAM's own service boundary and is a separate implementation task from the bolt-on RLS contract.

### 7.5 Policy template delivery format

**Options:**
- A) `.sql` migration files the app applies manually
- B) Helm chart snippets for the app's K8s deployment
- C) A `sesame-cli` tool that generates policy SQL from a config file

**Proposed (Stage 1):** Option A — SQL migration files. Simplest, most transparent, aligns with existing Sesame conventions.

---

## 8. Summary of Stage 1 Changes from Previous Design

| Element | Previous Design | Stage 1 Design |
|---|---|---|
| Session vars | 5: `user_id`, `org_id`, `user_type`, `permissions`, `email` | 6: + `org_type` |
| JWT claims | No `org_type` | `org_type` flows from platform service via JWT |
| RLS policies | org_id-only or user_type-only | org_type-aware (L1: org_id only, L2: org_type + org_id) |
| SesameExecutor | Wraps 5 session variables | Wraps 6 session variables |
| org_type location | Not defined | JWT claim → SET LOCAL → RLS policy (same trust boundary as user_type) |
| Policy templates | Generic org_id-only | org_type-aware templates for provider↔consumer model |
