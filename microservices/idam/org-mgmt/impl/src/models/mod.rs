//! Entity models for org-mgmt service.

pub mod entity_registry {
    include!(concat!(env!("OUT_DIR"), "/entity_registry.rs"));
}

pub mod application;
pub mod org;
pub mod org_domain;
pub mod org_invite;
pub mod org_membership;
pub mod permission;
pub mod role;
pub mod role_permission;
pub mod saml_connection;
pub mod scim_user;
pub mod webhook_subscription;
