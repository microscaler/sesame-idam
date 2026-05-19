# Authz-Core BDD Test Strategy

## Pattern Comparison: Hauliage vs Authz-Core

### Hauliage (consignments)
- **DB-backed**: Tests insert via ORM, query DB, verify persistence
- **Shared `common/mod.rs`**: `LazyLock<TestDatabase>` applies migrations
- **Steps interact with DB**: `#[when]` inserts via `ConsignmentRecord::insert()`, `#[then]` queries via ORM
- **HTTP-level or handler-level**: Steps call real handler with `TypedHandlerRequest`

### Authz-Core Tier 1 Controllers (already implemented)
- **Stateless**: No DB persistence, emits audit events, returns structured/dummy data
- **No shared DB**: No `common/mod.rs` needed (stateless)
- **Handler-level**: Direct `handle(typed_req)` calls, no server setup
- **Pattern**: `given` → set up Request, `when` → call handler, `then` → verify Response

---

## Controller Landscape (22 total controllers)

### Category A: Stateless Handlers (already done - Tier 1)
Controllers that emit audit events and return structured data with no DB.

1. `get_audit_event` - GET /authz/audit/events/{id} ✅ DONE
2. `get_audit_stats` - POST /authz/audit/events/stats ✅ DONE
3. `export_audit_events` - POST /authz/audit/events/export ✅ DONE
4. `check_export_status` - GET /authz/audit/events/export/{export_id} ✅ DONE
5. `update_retention_policy` - PUT /authz/audit/retention/{id} ✅ DONE

### Category B: JWT Token Validation (no DB, real logic)
6. `authorize` - POST /authz/authorize ✅ DONE (pre-existing)

### Category C: Principal Attribute Management (has DB models)
Controllers: `set_principal_attribute`, `principal_effective`, `assign_principal_role`, `revoke_principal_role`
- Have lifeguard entity models in `models/`
- Need DB for real testing
- Pattern: Insert entity → call handler → query DB to verify

### Category D: Retention Policy CRUD (has DB models)
Controllers: `create_retention_policy`, `list_retention_policies`, `delete_retention_policy`
- Have `AuditRetentionPolicy` entity model
- Need DB for real testing

### Category E: Audit Event Query (has DB models)
Controllers: `list_audit_events`, `search_audit_events`
- Need DB for real testing

---

## Implementation Phases

### Phase 1: Stateless Handler Tests (CONTINUED - Category A/E)
No DB needed. Pattern already established.

**Remaining to do:**
- [ ] Verify all Category A controllers have BDD tests
- [ ] Add `delete_retention_policy` test (if it has real handler logic)

**Pattern to follow:**
```rust
/// Scenario: Description.
#[test]
fn test_scenario() {
    let request_data = Request { ... };
    let typed_req = TypedHandlerRequest {
        method: Method::...,
        path: "...".to_string(),
        handler_name: "...".to_string(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: request_data,
    };
    let response = handle(typed_req);
    assert_eq!(response.field, expected_value);
}
```

### Phase 2: Principal Management Tests (Category C)
Need DB-backed approach. Two options:

**Option A: Handler-level with DB setup per-test** (Recommended)
- Each test creates its own request, inserts required entities manually via ORM
- Calls handler
- Verifies response structure + audit event emission
- Does NOT verify DB persistence (handler doesn't persist yet)

**Option B: Full DB-backed**
- Add `common/mod.rs` with `LazyLock<TestDatabase>`
- Each `#[given]` inserts entities via ORM
- Each `#[then]` queries DB to verify persistence
- Requires authz-core to have DB entity models AND persistence logic

**Decision**: Start with **Option A** (handler-level) since the controllers may not have persistence logic yet. This matches what we did for Tier 1.

### Phase 3: Audit Query Tests (Category E)
- `list_audit_events` and `search_audit_events` are query-only
- No persistence needed, just request/response validation
- Can be done as handler-level tests in Phase 1 style

### Phase 4: HTTP Integration Tests (Future)
Once controllers have full DB persistence:
- Spin up test server
- Use `curl`-like HTTP requests
- Verify full pipeline: middleware → handler → DB → response
- This is what hauliage does with real DB queries

---

## Recommended Approach: Hybrid Pattern

Since authz-core controllers are in mixed states (some stateless, some with partial DB), use:

**For stateless handlers** (Categories A, B): Handler-level `#[test]` functions
**For DB-backed handlers** (Categories C, D, E): Handler-level with manual entity setup per test

This keeps the approach consistent with Phase 1 (Tier 1 controllers) while avoiding the complexity of shared DB infrastructure that hauliage uses.

---

## File Structure (per controller)

```
impl/tests/bdd/{controller_name}.rs    # BDD step functions + scenarios
impl/tests/features/{controller_name}.feature  # Gherkin spec (optional but useful)
```

**Note**: For handler-level tests (no DB), the feature files serve as **documentation** since the step function names won't match Gherkin scenarios (we use plain `#[test]` instead of `#[scenario]` matching).

---

## Priority Order

1. **`list_audit_events`** - Stateless, structured response, audit emission
2. **`search_audit_events`** - Stateless, structured response, audit emission
3. **`create_retention_policy`** - Handler-level (no DB persist yet)
4. **`list_retention_policies`** - Handler-level (returns empty/dummy)
5. **`delete_retention_policy`** - Handler-level (emits audit event)
6. **`set_principal_attribute`** - Handler-level (emits audit event)
7. **`principal_effective`** - Handler-level (real JWT validation logic)
8. **`assign_principal_role`** - Handler-level (emits audit event)
9. **`revoke_principal_role`** - Handler-level (emits audit event)
