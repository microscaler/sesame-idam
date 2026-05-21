# sesame-jwt-common-path: Fix 68 test compilation errors

## Progress Tracker

| # | Task | Status | Commit Hash |
|---|------|--------|-------------|
| 0 | Initial setup / cargo check | DONE | abd7d48 |
| 1 | Add missing dev-dependencies (http, tokio, may, smallvec) | IN_PROGRESS | - |
| 2 | Fix dpop.rs tests (~20 errors) | PENDING | - |
| 3 | Fix jwt_validator.rs tests (~10 errors) | PENDING | - |
| 4 | Fix local_policy.rs tests (~15 errors) | PENDING | - |
| 5 | Fix middleware.rs tests (~23 errors) | PENDING | - |
| 6 | Add `SesameAuthzClaims::builder()` to common crate | PENDING | - |
| 7 | Final verification: cargo test --package sesame-jwt-common-path | PENDING | - |
| 8 | Commit all changes | PENDING | - |

## Error Inventory

### 1. `SesameAuthzClaims::builder()` doesn't exist (9 errors across 4 files)
- Fix: Either add builder method to common crate, or use `.sx(SesameAuthzClaims::new(tenant, portal, roles, perms))` directly

### 2. Async calls missing `.await` (5 errors in dpop.rs)
- `verify_dpop_proof()` returns Future, needs `.await`
- `store.record()` returns Future, needs `.await`

### 3. Wrong `HandlerRequest` construction (5 errors across dpop.rs, jwt_validator.rs)
- method: should be `http::Method`, not `String`
- query_params: should be `ParamVec`, not `HashMap`
- headers: should be `HeaderVec`, not `HashMap`
- Missing fields: request_id, handler_name, path_params, cookies, jwt_claims, reply_tx, queue_guard

### 4. Wrong `reply_tx` type (4 errors in middleware.rs)
- Uses `Arc<AtomicUsize>` but should be `mpsc::Sender<HandlerResponse>`

### 5. Missing dev-dependencies (2+ errors)
- `http` crate, `may` crate for `mpsc::channel`

### 6. Type conflicts in dpop.rs (2 errors)
- Local `DpopConfirmation` vs `sesame_common::dpop::DpopConfirmation`

### 7. Guard pattern experimental (1 error)

### 8. Missing type annotation (1 error)

### 9. Missing imports in local_policy.rs (3 errors)
- `RoutePolicy`, `AccessClaims` not in scope
