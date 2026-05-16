//! Entity models for authz-core.

pub mod entity_registry {
    include!(concat!(env!("OUT_DIR"), "/entity_registry.rs"));
}

pub mod audit_retention_policy;
pub mod authorization;
pub mod principal_attribute;
pub mod role_assignment;
