/// Controller handlers for Session management (JWT, refresh, OIDC, JWKS, step-up, impersonation, MCP).
///
/// Each controller corresponds to a single API endpoint. Controllers audit every
/// request via the global `EMITTER`, then delegate to the service layer.

pub mod admin_impersonate;
pub mod admin_issue_token;
pub mod admin_jwks_revoke;
pub mod admin_restore_impersonation;
pub mod auth_refresh;
pub mod jwks;
pub mod mcp_create_agent;
pub mod mcp_delete_agent;
pub mod mcp_get_agent;
pub mod mcp_list_agents;
pub mod mcp_token;
pub mod mcp_validate;
pub mod oauth_userinfo;
pub mod openid_configuration;
pub mod step_up_verify;
pub mod users_me_get;
pub mod users_me_patch;
