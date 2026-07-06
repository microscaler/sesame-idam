//! Identity session service — library target for migrator access.

// Suppressed for macro-generated code (LifeRecord derive + entity_registry.rs build output)
#![allow(clippy::pub_underscore_fields)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::uninlined_format_args)]
mod audit;
pub mod controllers;
pub mod jwks_client;
pub mod jwt;
pub mod key_manager;
pub mod middleware;
pub mod models;
// Raw (untyped) handler support for JWT-principal endpoints
pub mod raw_handler;
// Redis client and helpers for refresh token rotation
pub mod redis;
pub mod security;
// Token rotation service layer (Story 3.1)
pub mod services;
