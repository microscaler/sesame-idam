//! Service layer for refresh token rotation.
//!
//! Core business logic for:
//! - Token rotation (Story 3.1)
//! - Reuse detection and family revocation
//! - Cross-session notification triggers

pub mod token_rotation;
