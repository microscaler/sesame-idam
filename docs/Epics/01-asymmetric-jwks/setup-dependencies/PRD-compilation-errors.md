# PRD: Sesame-IDAM Compilation Error Fix

Epic 01: Asymmetric JWT & JWKS (EdDSA/Ed25519 signing, JWKS key publication)
Story 1.1: Key management with rotation lifecycle, JWKS endpoint, admin revoke

---

## Problem Statement

The sesame-IDAM workspace has compilation errors preventing full build. Five of twelve crates compile cleanly; three have test-code failures. Production code (`src/`) is clean — all errors are in `tests/` or `#[cfg(test)]` modules.

---

## Workspace Dependency Audit

### CRITICAL: All microscaler deps come from our forks, NOT crates.io

We do not publish to crates.io. We consume from microscaler forks because our PRs have not been successfully merged upstream. The forks are stable enough to pin directly with git endpoints.

**This is a non-negotiable rule. Never assume crates.io versions are correct for any dependency that has a microscaler fork.**

### Microscaler Fork Inventory

The following repos exist in `microscaler/` as forks with custom changes. All must be consumed from git, not crates.io:

| Fork Repo | Upstream | Branch | Notes |
|-----------|----------|--------|-------|
| `microscaler/may` | Xudong-Huang/may | main | Core stackful coroutine runtime. Foundation of BRRTRouter. |
| `microscaler/may_minihttp` | Xudong-Huang/may_minihttp | `integration/microscaler-fork` | Mini HTTP server. BRRTRouter: "Using our fork until PR #21 is merged upstream". Provides `TestClient`. |
| `microscaler/may_postgres` | (no direct upstream) | `master` | Postgres driver for may coroutines. Custom features (`with-serde_json-1`, `with-chrono-0_4`). |
| `microscaler/generator-rs` | Xudong-Huang/generator-rs | main | Coroutine generator. BRRTRouter patches this via `[patch.crates-io]` for Rust 1.90 macOS thread-local bug fix. |
| `microscaler/mayfly` | (no upstream) | main | Separate project. |

### Current Sesame-IDAM State (broken — uses crates.io for forks)

```toml
# WRONG — these have microscaler forks but are declared as crates.io:
may = "0.3"
may_minihttp = "0.1"

# Correctly uses git fork:
may_postgres = { git = "https://github.com/microscaler/may_postgres.git", branch = "master", ... }

# Uses patch section to swap to local path for dev:
[patch."https://github.com/microscaler/may_postgres.git"]
may_postgres = { path = "../../may_postgres" }
```

### Required Workspace Cargo.toml Changes

**`may` and `may_minihttp` must be changed to git endpoints**:

```toml
# workspace.dependencies section:
may = { git = "https://github.com/microscaler/may.git" }
may_minihttp = { git = "https://github.com/microscaler/may_minihttp.git", branch = "integration/microscaler-fork" }
```

Rationale:
- BRRTRouter uses `may = "0.3"` from crates.io — this is the ONLY dep where crates.io might be acceptable, but since we don't publish and crates.io versions are stale/abandoned, we should pin to our fork for consistency and to catch any divergent changes.
- `may_minihttp` MUST use the microscaler fork because the `TestClient` type (required by authz-core BDD tests) only exists in the fork. The crates.io version `0.1` lacks it entirely.

**`may_postgres`** — already uses git endpoint. Two patterns exist:

**Option A (lifeguard style — consistent git pin everywhere):**
```toml
# workspace Cargo.toml:
may_postgres = { git = "https://github.com/microscaler/may_postgres.git", rev = "6c92ef8b1e756741f3726fb94cc49af54f0f8596", features = ["with-chrono-0_4", "with-serde_json-1"] }

# Remove all impl-level may_postgres deps; use workspace = true
# Remove [patch] section
```

**Option B (hauliage style — workspace git + local path patch for dev):**
```toml
# workspace Cargo.toml:
may_postgres = { git = "https://github.com/microscaler/may_postgres.git", branch = "master", features = ["with-chrono-0_4", "with-serde_json-1"] }

# [patch] section for local dev swap:
[patch."https://github.com/microscaler/may_postgres.git"]
may_postgres = { path = "../../may_postgres" }
```

Lifeguard uses Option A (rev pin, no patch). Hauliage and sesame-IDAM use Option B (branch + patch). The `[patch]` swap avoids GitHub network dependency during development — useful on NFS-mounted code where CI/CD runners may not have internet access.

### Sesame-jwt-common-path dev-dependency

The `sesame-jwt-common-path/Cargo.toml` had a dev-dep for `may` pointing to a local vendored copy (`../../../../BRRTRouter/lib/may` — path doesn't exist; was at `vendor/may`). This was fixed to use `workspace = true`, which resolves to crates.io `may = "0.3"` (currently wrong — should resolve to our fork once workspace is fixed).

---

## Per-Service Compilation Results

### Clean Services (0 errors)

| Service | Path | Status |
|---------|------|--------|
| identity-session-service | impl/ | PASS (lib + tests) |
| api-keys | impl/ | PASS (lib + tests) |
| org-mgmt | impl/ | PASS (lib + tests) |
| identity-login-service | impl/ (lib only) | PASS |
| identity-user-mgmt-service | impl/ (lib only) | PASS |
| authz-core | impl/ (lib only) | PASS |

### Failed Services (test compilation errors)

---

## 1. identity-login-service — 12 errors in `pii_entitlements.rs`

**File**: `idam/identity-login-service/impl/tests/bdd/pii_entitlements.rs`

**Errors**: Every error is `cannot find module/type/function in sesame_common`:

```
error[E0433]: cannot find module or crate `sesame_common` in this scope
  --> pii_entitlements.rs:10:5
   use sesame_common::jwt::*;

error[E0425]: cannot find function `compute_entitlements_hash` in this scope
  --> pii_entitlements.rs:153:25

error[E0425]: cannot find function `verify_entitlements_hash` in this scope
  --> pii_entitlements.rs:158:9, 168:9

error[E0433]: cannot find type `AccessClaimsBuilder` in this scope
  --> pii_entitlements.rs:24:18, 92:18, 187:18

error[E0433]: cannot find type `SesameAuthzClaims` in this scope
  --> pii_entitlements.rs:39:13, 107:13, 202:13

error[E0422]: cannot find struct `EntitlementsSnapshot` in this scope
  --> pii_entitlements.rs:145:24
```

**Diagnosis**: The test imports `sesame_common::jwt::*` which does not re-export `AccessClaimsBuilder`, `SesameAuthzClaims`, or the entitlement functions. Either:
- The types exist in `sesame_common` but aren't `pub use`d in the `jwt` module
- The types exist in `sesame_common::jwt` but need explicit `use` paths instead of glob import
- The types don't exist yet and need to be added to `sesame_common`

**Files to check**:
- `idam/common/src/jwt/mod.rs` — what is actually exported?
- `idam/common/src/lib.rs` — how is `sesame_common` organized?
- `idam/common/Cargo.toml` — feature flags affecting exports

---

## 2. identity-user-mgmt-service — 1 error in `jwt_ttl.rs`

**File**: `idam/identity-user-mgmt-service/impl/tests/bdd/jwt_ttl.rs:8`

**Error**:
```
error[E0433]: cannot find `jwt` in `sesame_idam_identity_user_mgmt_service`
   use sesame_idam_identity_user_mgmt_service::jwt::ttl::{...};
```

**Diagnosis**: The `jwt` module exists in the service's `impl/src/jwt/` directory but is not re-exported from the crate root. The test uses an absolute path import that requires `jwt` to be `pub use`d from `lib.rs` or `main.rs`.

**Possible fixes**:
- Add `pub use jwt::ttl::{...};` in the service's lib.rs
- Or change the test import to use the crate's own module path
- Or add a `dev-dependency` on `sesame_idam_identity_login_service` to share the JWT module

---

## 3. authz-core — 46 errors across 3 categories

### 3A. Missing fields on AuthError (12 errors)

Tests reference fields/methods that don't exist on the `AuthError` struct:

```
error[E0615]: attempted to take value of method `retry_after` on type `auth_error::AuthError`
  --> auth_error.rs:645:24
  assert_eq!(err.retry_after, 300);
  assert_eq!(err.retry_after, 0);
  assert_eq!(err.retry_after, 300);  // 5 total retry_after field accesses
                              (some in auth_error.rs unit tests, some in version_mismatch.rs BDD)

error[E0609]: no field `expected_min_version` on type `auth_error::AuthError`
  --> auth_error.rs:655:24, 714:24
  version_mismatch.rs:47:20, 579:20, 719:24  // 5 total

error[E0609]: no field `actual_version` on type `auth_error::AuthError`
  --> auth_error.rs:656:24, 724:24
  version_mismatch.rs:48:20, 580:20, 720:24  // 5 total

error[E0277]: `Option<std::string::String>` doesn't implement `std::fmt::Display`
  --> create_retention_policy.rs:102:9
  Format string expects a Display type but gets Option<String>
```

**Diagnosis**: The `AuthError` struct was refactored. Tests were written against an older version with fields `retry_after`, `expected_min_version`, `actual_version` as direct struct fields. The current code may have:
- These as methods instead of fields (the `retry_after` error says "attempted to take value of method" — suggesting it IS a method but tests access it as a field)
- Different field names or removed entirely

**Files to check**:
- `idam/authz-core/impl/src/auth_error.rs` — current `AuthError` struct definition
- `idam/authz-core/impl/src/auth_error.rs` — test module starting around line 610+

### 3B. Missing pagination fields on generated response types (17 errors)

BDD tests reference fields that generated API response types don't have:

```
list_audit_events::Response:
  error[E0609]: no field `items` (3 occurrences)
  error[E0609]: no field `total` (2 occurrences)
  error[E0609]: no field `page` (1 occurrence)
  error[E0609]: no field `limit` (1 occurrence)
  File: tests/bdd/list_audit_events.rs

list_retention_policies::Response:
  error[E0609]: no field `items` (1 occurrence)
  File: tests/bdd/list_retention_policies.rs

search_audit_events::Response:
  error[E0609]: no field `items` (5 occurrences)
  error[E0609]: no field `total` (2 occurrences)
  File: tests/bdd/search_audit_events.rs

create_retention_policy::Response:
  error[E0599]: no method named `is_empty` on Option<String> (3 occurrences)
  The `id` field is Option<String>, not String — can't call is_empty()
```

**Diagnosis**: Tests were written against an older OpenAPI spec or a different version of the generated code. The current generated response types use different field names or have different structures:
- Paginated responses may use `results` instead of `items`, or have no pagination fields at all
- The `id` field changed from `String` to `Option<String>`

**Files to check**:
- `openapi/idam/authz-core/openapi.yaml` — current response schemas
- `idam/authz-core/gen/src/handlers/list_audit_events.rs` — generated Response type
- `idam/authz-core/gen/src/handlers/list_retention_policies.rs`
- `idam/authz-core/gen/src/handlers/search_audit_events.rs`

### 3C. Type import errors (2 errors)

```
error[E0432]: unresolved import `may_minihttp::TestClient`
  --> tests/bdd/version_mismatch.rs:17:70
  use may_minihttp::{Request as MiniRequest, Response as MiniResponse, TestClient};

error[E0425]: cannot find type `Uuid` in this scope
  --> src/org_resolution.rs:34:64
  assert!("550e8400-...".parse::<Uuid>().is_ok());
```

**Diagnosis**:
- `TestClient` is a local fork type from `may_minihttp` that isn't available on crates.io
- `Uuid` isn't imported in scope in `org_resolution.rs` — needs `use uuid::Uuid;` or `use crate::uuid::Uuid;`

---

## Summary of Required Fixes

### Critical (workspace/build blockers)

1. **`may_minihttp::TestClient`** — The crates.io `may_minihttp` crate doesn't export `TestClient`. This is a local fork feature. Either:
   - Add a git dep pointing to the microscaler fork for test-only usage
   - Or replace `TestClient` usage with std `reqwest` or `hyper` testing equivalents
   - Or use `may_minihttp = { git = "..." }` in workspace deps for test targets

### AuthError tests (12 errors, 12 LOC changes)

2. **`retry_after`** — Change from field access to method call: `err.retry_after` → `err.retry_after()`
3. **`expected_min_version`** and **`actual_version`** — These fields don't exist on `AuthError`. Need to check if:
   - They were removed and tests should be deleted
   - They exist with different names
   - They need to be added to `AuthError` struct

### Generated response types (17 errors, ~20 LOC changes)

4. **Pagination fields** — Update BDD tests to match current generated response types:
   - `items` → may be `results` or `entries` or the response has no list field
   - `total`, `page`, `limit` — may not exist on current response schema
   - `id` field changed from `String` to `Option<String>` — use `.as_ref().map_or(false, |s| !s.is_empty())` or similar

### Type imports (3 errors, 1 LOC change each)

5. **`Uuid` import** in `org_resolution.rs` — add `use uuid::Uuid;`
6. **`pii_entitlements.rs`** — Fix `sesame_common` imports. Check what types are actually exported from `sesame_common::jwt`
7. **`jwt_ttl.rs`** (user-mgmt-service) — Fix module path or add re-export

---

## Fix Priority

1. **Critical path blockers**: `may_minihttp::TestClient`, `Uuid` missing import
2. **AuthError tests**: 12 errors, all in test code
3. **Pagination response fields**: 17 errors, BDD tests
4. **pii_entitlements.rs**: 12 errors, depends on `sesame_common` exports
5. **jwt_ttl.rs**: 1 error, module path

---

## Verification Steps

After fixes, run:
```bash
cargo check -p sesame_idam_identity_login_service --all-targets
cargo check -p sesame_idam_identity_user_mgmt_service --all-targets
cargo check -p sesame_idam_authz_core --all-targets
cargo check --workspace --all-targets
cargo test --workspace --no-run  # verify tests compile
cargo clippy --workspace --all-targets --all-features  # lint pass
```
