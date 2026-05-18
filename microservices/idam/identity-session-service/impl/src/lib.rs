//! Identity session service — library target for migrator access.

// Suppressed for macro-generated code (LifeRecord derive + entity_registry.rs build output)
#![allow(clippy::pub_underscore_fields)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::uninlined_format_args)]
mod audit;
pub mod jwks_client;
pub mod key_manager;
pub mod models;
