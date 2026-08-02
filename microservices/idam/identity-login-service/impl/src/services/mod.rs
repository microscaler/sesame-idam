//! Domain services using [`lifeguard::LifeExecutor`] — controllers call
//! services, never the database directly (hauliage pattern). Executors come
//! from `sesame_idam_database::db()` at the controller edge.

pub mod abuse_guard;
pub mod auth_code;
pub mod authz_client;
pub mod client_registry;
pub mod email;
pub mod envelope;
pub mod login_success;
pub mod oauth;
pub mod org_context;
pub mod otp;
pub mod sms;
pub mod sms_sender;
pub mod password;
pub mod platform_auth;
pub mod social_credential_service;
pub mod tenant_admin;
pub mod tenant_assurance;
pub mod tenant_gate;
pub mod tenant_oauth_service;
pub mod tenant_service;
pub mod tenant_sms_service;
pub mod token_issuer;
pub mod user_service;
