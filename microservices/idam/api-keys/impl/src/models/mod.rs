//! Entity models for api-keys service.

pub mod entity_registry {
    include!(concat!(env!("OUT_DIR"), "/entity_registry.rs"));
}

pub mod api_key;
pub mod api_key_usage;
pub mod archived_api_key;
