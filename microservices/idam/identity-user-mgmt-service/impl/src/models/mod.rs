//! Entity models for identity-user-mgmt-service.

pub mod entity_registry {
    include!(concat!(env!("OUT_DIR"), "/entity_registry.rs"));
}

pub mod audit_event;
pub mod employee;
pub mod email_verification;
pub mod mfa_setup;
pub mod social_account;
pub mod user;
