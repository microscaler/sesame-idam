# BDD Testing Plan for Authz-Core

## Phase 1: Infrastructure Setup (the boilerplate)

**Goal**: Get /health and /metrics working as BDD tests, establishing the pattern for everything else.

### 1.1: Wire up `common/mod.rs`

Copy the hauliage consignments pattern. Create a `LazyLock<TestDatabase>` that:
- Reads `TEST_DB_HOST`, `TEST_DB_PORT`, `TEST_DB_USER`, `TEST_DB_PASS`, `TEST_DB_NAME` (defaults to `127.0.0.1:5432/postgres/postgres/idam`)
- Connects to postgres with `may_postgres`
- Creates the `sesame_idam` schema and sets search_path
- Runs any seed SQL needed for authz-core (the `seeds/` dir has placeholder SQL)

### 1.2: Add dev dependencies to `Cargo.toml`

Verify `may_minihttp` and `reqwest` (if needed) are in dev-dependencies for in-process HTTP testing. Authz-core is stateless (no DB needed for /health, /metrics), so `may_minihttp` TestClient is sufficient.

### 1.3: Write `tests/bdd/smoke.rs` — real smoke tests

Replace the placeholder with:
- Test: health endpoint returns 200 (using `may_minihttp` TestClient or direct handler call)
- Test: metrics endpoint returns prometheus-format text (checking headers, status code)
- Feature file `tests/features/smoke.feature` with scenarios:
  - "Service healthcheck returns 200"
  - "Metrics endpoint returns prometheus format"

### 1.4: Register `smoke` in `main_bdd.rs`

Already registered. Just verify it compiles and the tests actually run.

---

## Phase 2: Stateless Controller BDD (Epic 1 — what's actually implemented)

**Goal**: Write BDD specs for the 5 controllers currently in `controllers/mod.rs`, exercising the full handler pipeline (not just schema validation).

### Controllers to test:

| # | Controller | Method | Path | Status |
|---|-----------|--------|------|--------|
| 1 | `authorize` | POST | /authz/authorize | Stub (always returns allowed=true) |
| 2 | `principal_effective` | POST | /authz/principals/effective | Stub (returns empty roles/permissions) |
| 3 | `set_principal_attribute` | POST | /authz/principals/attributes | Stub (emits audit, returns error field) |
| 4 | `assign_principal_role` | POST | /authz/principals/roles | Stub (emits audit, returns assigned role) |
| 5 | `revoke_principal_role` | DELETE | /authz/principals/roles | Stub (emits audit, returns empty) |

### Per-controller BDD pattern (matching hauliage `list_jobs.rs`):

```rust
pub struct ControllerTestContext {
    pub last_response: Option<gen::handlers::...::Response>,
}

#[fixture]
fn context() -> Arc<Mutex<ControllerTestContext>> { ... }

// Given steps: setup test data (if any)
#[given("a valid {controller} request")]
fn given_valid_request(...) { ... }

#[given("the database is empty for {controller} tests")]
fn given_empty_db(...) { ... }

// When steps: call controller handler directly
#[when("I send a valid request to {controller}")]
fn when_call_handler(...) {
    let handler = ControllerName;
    let typed_req = make_typed_request(...);
    let response = handler.handle(typed_req);
    context.lock().unwrap().last_response = Some(response);
}

// Then steps: assert on Response or DB
#[then("the response has field \"{field}\"")]
fn then_response_has_field(...) { ... }

#[then("the response is valid JSON with allowed=true")]
fn then_allowed_is_true(...) { ... }

// Scenario runner tied to feature file
#[scenario(path = "tests/features/{controller}.feature")]
#[rstest]
fn scenario_name(context: Arc<Mutex<ControllerTestContext>>) {
    let ctx = context.lock().unwrap();
    let resp = ctx.last_response.as_ref().expect("No response cached");
    // assertions on resp
}
```

### Feature files (one per controller, under `tests/features/`):

Each file follows Gherkin format:

```gherkin
Feature: Controller Name (METHOD /path)
  As a service consumer
  I want to use the controller
  So that I can perform authorization operations

  Scenario: Valid request returns success
    Given a valid request
    When I send a valid request to {controller}
    Then the response status is 200
    And the response body has field "allowed" set to true

  Scenario: Missing required field returns 400
    Given an incomplete request (missing action)
    When I send a valid request to {controller}
    Then the response status is 400
```

---

## Phase 3: Refactor Existing Tests

The current `bdd/authorize.rs`, `bdd/principal_effective.rs`, `bdd/set_principal_attribute.rs` are **unit tests** (schema validation only via serde). They never call handlers. They are valuable but serve a different purpose.

**Action**: Move them to `tests/unit/` as `unit_schema.rs`, `unit_principal.rs`, `unit_attribute.rs`. Keep them — they catch API contract regressions.

The BDD files (`tests/bdd/*.rs`) should focus on **handler pipeline** testing: request -> handler -> response -> audit event emission.

---

## Phase 4: JWT Validation BDD (already partially done)

The current `bdd/jwt_validation.rs` has 10 tests for JWT validation. These are actually good — they test the middleware pipeline (building JWTs with Ed25519, sending them, checking if they pass).

**Improvements**:
- Convert to proper `#[given]`/`#[when]`/`#[then]` structure matching hauliage
- Add feature file `tests/features/jwt_validation.feature`
- Add scenario for: valid JWT accepted, expired JWT rejected, wrong algorithm rejected, missing kid rejected, alg:none attack rejected

---

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `tests/common/mod.rs` | **Rewrite** | `LazyLock<TestDatabase>` for DB setup (copy hauliage pattern) |
| `tests/bdd/smoke.rs` | **Rewrite** | Real /health and /metrics BDD tests |
| `tests/features/smoke.feature` | **Rewrite** | Gherkin spec for smoke tests |
| `tests/bdd/authorize.rs` | **Rewrite** | BDD tests for authorize controller (call handler, not just serde) |
| `tests/features/authorize.feature` | **Keep + update** | Existing feature file, add missing scenarios |
| `tests/bdd/principal_effective.rs` | **Rewrite** | BDD tests for principal_effective controller |
| `tests/features/principal_effective.feature` | **Keep + update** | Existing feature file, add missing scenarios |
| `tests/bdd/set_principal_attribute.rs` | **Rewrite** | BDD tests for set_principal_attribute controller |
| `tests/features/set_principal_attribute.feature` | **Keep + update** | Existing feature file, add missing scenarios |
| `tests/bdd/role_management.rs` | **Create** | BDD for assign_principal_role + revoke_principal_role (group together) |
| `tests/features/role_management.feature` | **Create** | Gherkin spec for role assignment/revocation |
| `tests/bdd/jwt_validation.rs` | **Refactor** | Convert to hauliage `#[given]`/`#[when]`/`#[then]` pattern |
| `tests/features/jwt_validation.feature` | **Create** | Gherkin spec for JWT validation |
| `tests/unit/` | **Create** | Move existing unit-only schema validation tests here |
| `tests/main_bdd.rs` | **Update** | Register new `unit` and `role_management` modules |
| `impl/Cargo.toml` | **Verify** | Ensure dev-deps are complete (`may_minihttp`, `rstest`, `rstest-bdd-*`) |

---

## Execution Order

1. **Smoke tests first** — establish the `common/mod.rs` pattern, verify it compiles and runs
2. **Authorize controller** — the most used controller, best test case for the pattern
3. **Principal effective** — exercises Response struct with arrays (roles, permissions)
4. **Set principal attribute** — exercises HandlerRequest construction pattern
5. **Role management** (assign + revoke) — group together, similar Request/Response shapes
6. **JWT validation** — refactor existing tests to hauliage pattern
7. **Unit tests** — move schema validation tests to `tests/unit/`

---

## What "Done" Looks Like

- `cargo test --package sesame_idam_authz_core` passes all BDD tests
- `just nt` passes (if nextest is configured for this crate)
- Each of the 5 controllers has: feature file (human-readable spec) + bdd file (Rust implementation calling the handler)
- `/health` and `/metrics` smoke tests verify service reachability
- Existing unit tests preserved in `tests/unit/` for schema contract testing
- JWT validation tests cover: valid token, expired, wrong alg, missing kid, alg:none attack, missing Bearer prefix
