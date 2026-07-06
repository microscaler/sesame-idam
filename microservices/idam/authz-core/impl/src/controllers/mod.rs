// The `#[handler]` macro requires `handle(req: TypedHandlerRequest<Request>)`
// by value — suppress clippy::needless_pass_by_value for all controllers.
#![allow(clippy::needless_pass_by_value)]
pub mod assign_principal_role;
pub mod authorize;
pub mod check_export_status;
pub mod export_audit_events;
pub mod get_audit_event;
pub mod get_audit_stats;
pub mod principal_effective;
pub mod revoke_principal_role;
pub mod set_principal_attribute;
pub mod update_retention_policy;

// Epic 1 audit/retention — additional controllers
pub mod create_retention_policy;
pub mod delete_retention_policy;
pub mod list_audit_events;
pub mod list_retention_policies;
pub mod search_audit_events;
