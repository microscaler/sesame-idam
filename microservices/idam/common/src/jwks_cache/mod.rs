//! JWKS (JSON Web Key Set) cache with background refresh, stale tolerance, and security protections.
//!
//! This module provides a service-level JWKS cache that eliminates the single-point-of-failure
//! caused by making HTTP calls to the JWKS endpoint on every JWT validation.
//!
//! # Design
//!
//! - **TTL**: Keys are cached for 5 minutes by default before background refresh.
//! - **Stale tolerance**: Even if the cache is stale (>5min), keys remain valid for 15 minutes
//!   after last refresh, providing resilience during transient JWKS endpoint outages.
//! - **Fallback**: If the requested `kid` is not found, any cached key can be used as fallback.
//! - **Atomic replacement**: Background refresh replaces the entire key set atomically — no
//!   partial state visible to concurrent readers.
//!
//! # Security
//!
//! Addresses HACK-711 through HACK-714:
//! - Size limits: max 10 keys, max 10KB per key, max 100KB total document
//! - Single-flight pattern: concurrent requests deduplicate to one fetch
//! - Stale key warning logs with metrics
//!
//! # Example
//!
//! ```rust,no_run
//! use crate::jwks_cache::JwksCache;
//! use std::thread;
//! use std::time::Duration;
//!
//! let cache = JwksCache::builder()
//!     .endpoint("https://idam.example.com/.well-known/jwks.json")
//!     .build();
//!
//! // Start background refresh (non-blocking, uses may::go!)
//! cache.start_background_refresh();
//!
//! // Wait for initial fill
//! thread::sleep(Duration::from_millis(500));
//!
//! // Fetch key by kid (now sync)
//! let key = cache.get_key("key-2026-05");
//!
//! // Or fallback to any available key (now sync)
//! let any_key = cache.get_any_valid_key();
//! ```

pub mod cache;
pub mod types;

#[cfg(test)]
mod tests;

pub use cache::JwksCache;
pub use types::{Jwk, JwksCacheError, JwksDocument, JwksHealthCheck};
