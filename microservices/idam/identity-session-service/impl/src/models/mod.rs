//! Entity models for identity-session-service.

pub mod entity_registry {
    include!(concat!(env!("OUT_DIR"), "/entity_registry.rs"));
}

pub mod impersonation;
pub mod mcp_agent;
pub mod refresh_token;
pub mod session;
pub mod token;
pub mod user_profile;
