# Design Doc: Stateful RLS Implementation for Hauliage

> **Status:** Draft — Comprehensive Design
> **Scope:** RLS architecture, Hauliage data model, policy definitions, and ORM integration
> **Date:** 2026-01-04

---

## 1. The Challenge: Stateful Visibility

Hauliage requires **stateful visibility** — the set of rows visible to a user changes based on the lifecycle status of a record, not just the user's organizational membership.

### 1.1 The Workflow

1.  **Job Posted:** A Shipper posts a job. The job is **globally visible** so all Transporters can discover it and submit bids.
2.  **Bidding:** Transporters submit bids. The bid is visible to the bidder and the shipper.
3.  **Allocation:** The Shipper selects a Transporter. The job is **now restricted**: only the Shipper and the Allocated Transporter can see the job details and manage the execution. All other Transporters must be locked out.

### 1.2 Why Standard `org_id` Scoping Fails

In a standard multi-tenant setup, RLS is static: `USING (org_id = current_setting('auth.user_org_id'))`.
This works perfectly for "rows belong to exactly one org," but it fails for Hauliage because a single `jobs` row must be visible to two different orgs (Shipper and Transporter) simultaneously, based on runtime logic.

**The Solution:** RLS policies must reference **row attributes** (e.g., `status`, `allocated_user_id`) alongside session variables to create "toggled" visibility.

---

## 2. RLS Architecture Recap (Sesame-IDAM Integration)

Sesame-IDAM provides the trust boundary between the JWT and the Database.

1.  **Application Layer:** The BRRTRouter middleware validates the JWT (RS256) and extracts claims (`user_id`, `org_id`, `user_org_type`).
2.  **ORM Layer (Lifeguard):** The `SesameExecutor` wrapper intercepts queries and runs `SELECT sesame_set_session(...)` at the start of every transaction.
3.  **Database Layer:** PostgreSQL Helper Functions expose these values as session variables (`auth.user_id`, etc.).
4.  **RLS Layer:** Policies read these session variables to filter rows transparently.

**Crucial Design Constraint:** The JWT never travels to the database. Only verified claims are injected as `SET LOCAL` session variables.

---

## 3. Hauliage Data Model for RLS

To implement stateful visibility, the `jobs` and `bids` tables must include specific tracking fields.

### 3.1 The `jobs` Table

| Field | Type | Description |
|---|---|---|
| `id` | `uuid` | Primary Key |
| `shipper_org_id` | `uuid` | The organization that posted the job |
| `shipper_user_id` | `uuid` | The specific shipper who posted it |
| `title` | `text` | Job title |
| `details` | `jsonb` | Job metadata (route, weight, etc.) |
| `status` | `text` | **Critical for RLS:** Values: `open`, `allocated`, `in_transit`, `completed` |
| `allocated_transporter_user_id` | `uuid` | **Critical for RLS:** The user ID of the winning transporter. Null if `status` is `open` |
| `created_at` | `timestamptz` | Record creation time |
| `updated_at` | `timestamptz` | Record update time |

### 3.2 The `bids` Table

| Field | Type | Description |
|---|---|---|
| `id` | `uuid` | Primary Key |
| `job_id` | `uuid` | FK to `jobs` |
| `transporter_user_id` | `uuid` | **Critical for RLS:** The user who submitted the bid |
| `transporter_org_id` | `uuid` | The transporter's organization |
| `amount` | `numeric` | Bid price |
| `message` | `text` | Bid notes |
| `created_at` | `timestamptz` | Record creation time |

---

## 4. RLS Policy Definitions

### 4.1 Policy Strategy: Toggled Visibility

We will use a single `CREATE POLICY` per table that evaluates conditions based on the **current state of the row** relative to the **current session**.

#### 4.1.1 Jobs Table Visibility Policy

This policy handles the global-to-restricted transition.

```sql
CREATE POLICY hauliage_job_visibility ON public.jobs
    FOR ALL
    USING (
        -- Rule 1: Open jobs are globally visible (allow bidding)
        status = 'open'

        UNION

        -- Rule 2: The Shipper (job owner) always sees their jobs
        shipper_org_id = sesame_current_user_org_id()

        UNION

        -- Rule 3: The allocated transporter can see the job once assigned
        -- Note: We check user_id to ensure we are targeting the specific user who won
        allocated_transporter_user_id = sesame_current_user_id()
    );
```

**Why this works:**
- When `status = 'open'`, the first rule passes for everyone.
- Once `status = 'allocated'`, Rule 1 fails.
- Rule 2 ensures the Shipper never loses access.
- Rule 3 ensures the winning Transporter gains access immediately upon allocation.
- **Security:** A random third-party Transporter will fail all three rules and see 0 rows.

#### 4.1.2 Bids Table Visibility Policy

This policy ensures only the bidder and the shipper see the financial details.

```sql
CREATE POLICY hauliage_bid_visibility ON public.bids
    FOR ALL
    USING (
        -- Rule 1: The bidder sees their own bids
        transporter_user_id = sesame_current_user_id()

        UNION

        -- Rule 2: The shipper sees all bids for their job
        EXISTS (
            SELECT 1 FROM public.jobs j
            WHERE j.id = job_id
            AND j.shipper_org_id = sesame_current_user_org_id()
        )
    );
```

---

## 5. Security & Performance Considerations

### 5.1 Connection Pooling & `SET LOCAL`

Hauliage will likely run inside a Kubernetes cluster using a connection pooler (e.g., `pgbouncer` or the built-in `LifeguardPool`).

- **Safety:** We use `SET LOCAL` inside the transaction. This variable is strictly scoped to the current transaction block.
- **Cleanup:** When the transaction commits or rolls back, PostgreSQL automatically destroys the session variable.
- **Isolation:** Even if two users share the same underlying TCP connection in the pool, their `SET LOCAL` scopes never bleed into each other.

### 5.2 Performance Indexing

PostgreSQL must evaluate the `USING` clause for every row it considers. To prevent full table scans:

1.  **Status Index:** `CREATE INDEX idx_jobs_status ON jobs(status);`
    *   Allows Postgres to quickly filter out all `allocated` jobs when a Transporter queries the list.
2.  **Allocation Index:** `CREATE INDEX idx_jobs_allocated ON jobs(allocated_transporter_user_id);`
    *   Allows Postgres to quickly find the specific job assigned to a transporter.
3.  **Composite Index (Optional):** `CREATE INDEX idx_jobs_status_org ON jobs(status, shipper_org_id);`
    *   Optimizes the query "Show me all my open jobs."

### 5.3 Trust Boundary Enforcement

- **`user_id` is authoritative:** The `allocated_transporter_user_id` in the DB is compared against `sesame_current_user_id()`. This value was extracted from a JWT that was verified using Sesame's public keys (RS256) in the BRRTRouter middleware.
- **No client-side tampering:** A client cannot change their `user_id` to match another transporter's ID because the middleware rejects any JWT with a mismatched signature or payload.

---

## 6. Lifeguard ORM Integration

Lifeguard abstracts the database interaction. The `SesameExecutor` wrapper ensures that the session variables are set before the query reaches the RLS engine.

### 6.1 Transaction Lifecycle

```rust
// In the "Bid on Job" handler
async fn bid_on_job(job_id: Uuid, amount: Decimal) -> Result<Bid> {
    let tx = sesame_executor.begin().await?; // Starts transaction

    // SesameExecutor ensures `SET LOCAL auth.user_id = '...'` is sent
    // before this raw SQL executes.
    let bid = Bid::create(&tx, job_id, amount, context.user_id, context.org_id);

    tx.commit().await?; // Transaction ends, SET LOCAL is cleared automatically
    Ok(bid)
}
```

### 6.2 Querying Jobs (The "Bidder View")

```rust
// In the "Available Jobs" endpoint
async fn list_available_jobs() -> Result<Vec<Job>> {
    let tx = sesame_executor.begin().await?;

    // Query: SELECT * FROM jobs WHERE status = 'open';
    // RLS Layer sees: SELECT * FROM jobs WHERE status = 'open'
    // AND (status = 'open' OR org_id = session_org_id OR allocated_id = session_user_id)
    let jobs = Job::filter(Job::Status::eq("open")).all(&tx).await?;

    // A Transporter sees all "open" jobs.
    // A Shipper only sees "open" jobs owned by their org.
    Ok(jobs)
}
```

---

## 7. Summary of Stateful Visibility Logic

| User Role | Requesting: `List Jobs` | Policy Execution | Result |
|---|---|---|---|
| **Transporter A** | All Jobs | `status='open'` is TRUE for all open jobs. `allocated_transporter_user_id` is checked for allocated jobs (fails unless they are A). | ✅ Sees all open jobs. |
| **Shipper X** | All Jobs | `status='open'` is FALSE for allocated jobs. `shipper_org_id` matches the job's org. | ✅ Sees their open jobs + their allocated jobs. |
| **Transporter B** | Allocated Job 123 | `status='open'` is FALSE. `org_id` doesn't match. `allocated_transporter_user_id` is Transporter A (not B). | ❌ Sees nothing (403/404). |

This design satisfies the requirement for **global visibility at start**, **restricted visibility after allocation**, and **secure handoff between parties** without requiring complex application-level locking or custom API filters.
