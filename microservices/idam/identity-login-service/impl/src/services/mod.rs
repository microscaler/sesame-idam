//! Domain services using [`lifeguard::LifeExecutor`] — controllers call
//! services, never the database directly (hauliage pattern). Executors come
//! from `sesame_idam_database::db()` at the controller edge.

pub mod password;
pub mod token_issuer;
pub mod user_service;
