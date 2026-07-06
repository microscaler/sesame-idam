//! Service layer for session-service business logic.
//!
//! Core business logic for:
//! - Token rotation (Story 3.1): reuse detection, family revocation,
//!   cross-session notification triggers
//! - Current-user profile resolution (`/identity/me`)

pub mod profile_service;
pub mod token_rotation;
