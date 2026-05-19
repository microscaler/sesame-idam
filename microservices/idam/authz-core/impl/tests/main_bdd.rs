// BDD test hub for authz-core
//
// Single test crate that unifies BDD specs via rstest_bdd_macros.
// Each controller has its own module following the hauliage pattern:
//   1. TestContext struct holds last_response for scenario sharing
//   2. #[fixture] fn context() returns Arc<Mutex<TestContext>>
//   3. #[given(...)] steps set up request/fixture state
//   4. #[when(...)] steps call the handler, cache response in context
//   5. #[then(...)] steps assert on the cached response
//   6. #[scenario(path = "tests/features/*.feature")] tests verify the struct

pub mod common;

pub mod bdd {
    // Epic 1 auth/role — existing modules
    pub mod all_endpoints;
    pub mod authorize;
    pub mod jwt_validation;
    pub mod principal_effective;
    pub mod set_principal_attribute;

    // Epic 1 audit/retention — per-controller BDD specs
    pub mod get_audit_event;
    pub mod get_audit_stats;
    pub mod export_audit_events;
    pub mod check_export_status;
    pub mod update_retention_policy;
}
