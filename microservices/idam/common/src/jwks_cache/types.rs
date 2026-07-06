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
//! - TLS with certificate validation via `brrtrouter::http` (rustls for HTTPS)
//! - Fetch rate limiting: max 1 fetch per second per instance
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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants & Configuration
// ---------------------------------------------------------------------------

/// Default JWKS cache TTL (5 minutes).
pub const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 300;
/// Default stale tolerance (15 minutes). Keys remain valid up to this age.
pub const DEFAULT_STALE_TOLERANCE_SECS: u64 = 900;
/// Maximum number of keys allowed in the JWKS cache.
pub const DEFAULT_MAX_KEYS: usize = 10;
/// Maximum size of a single JWK in bytes (10 KB).
pub const DEFAULT_MAX_KEY_SIZE_BYTES: usize = 10 * 1024;
/// Maximum total size of the JWKS document (100 KB).
pub const DEFAULT_MAX_JWKS_SIZE_BYTES: usize = 100 * 1024;
/// Minimum fetch interval (1 second) — prevents refresh storms.
pub const MIN_FETCH_INTERVAL_SECS: u64 = 1;

// ---------------------------------------------------------------------------
// JWK types
// ---------------------------------------------------------------------------

/// A JSON Web Key (JWK) representation.
///
/// Minimal representation covering the fields needed for JWT signature verification.
/// Supports OKP (Ed25519), RSA, and EC key types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Jwk {
    /// Key type: "OKP", "RSA", "EC", etc.
    #[serde(rename = "kty")]
    pub kty: String,

    /// Key ID — unique identifier for this key.
    #[serde(rename = "kid")]
    pub kid: String,

    /// Key usage: "sig", "enc", etc.
    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_claim: Option<String>,

    /// Algorithm for the key (e.g., "`EdDSA`", "RS256").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alg: Option<String>,

    /// Elliptic curve (e.g., "Ed25519", "P-256").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crv: Option<String>,

    /// RSA modulus.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,

    /// RSA public exponent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,

    /// EC X coordinate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,

    /// EC Y coordinate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,

    /// RSA public key as base64url-encoded PEM (alternative format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5c: Option<Vec<String>>,

    /// Raw JSON value for any additional fields.
    #[serde(flatten)]
    pub additional: serde_json::Map<String, serde_json::Value>,
}

/// JWKS document structure (RFC 7517).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksDocument {
    /// Array of JWKs.
    pub keys: Vec<Jwk>,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during JWKS cache operations.
#[derive(Debug, Clone, Error)]
pub enum JwksCacheError {
    #[error("JWKS endpoint returned {status}: {message}")]
    FetchError { status: u16, message: String },

    #[error("JWKS document exceeds maximum size ({max} bytes, got {actual} bytes)")]
    DocumentTooLarge { max: usize, actual: usize },

    #[error("JWK exceeds maximum size ({max} bytes, got {actual} bytes)")]
    KeyTooLarge { max: usize, actual: usize },

    #[error("JWKS document contains too many keys ({max}, got {actual})")]
    TooManyKeys { max: usize, actual: usize },

    #[error("Invalid JWKS JSON: {0}")]
    ParseError(String),

    #[error("No keys available in cache")]
    NoKeysAvailable,

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Cache is empty and endpoint unreachable: {0}")]
    CacheEmpty(String),

    #[error("Request cancelled (background refresh not started)")]
    NotStarted,
}

// ---------------------------------------------------------------------------
// Cache metrics
// ---------------------------------------------------------------------------

#[cfg(feature = "metrics")]
mod metrics {
    use prometheus::{
        register_gauge_vec, register_histogram_vec, register_int_counter_vec, GaugeVec,
        HistogramVec, IntCounterVec,
    };

    lazy_static::lazy_static! {
        /// Ratio of cache hits to total lookups.
        pub static ref JWKS_CACHE_HIT_RATIO: GaugeVec = register_gauge_vec!(
            "jwks_cache_hit_ratio",
            "Cache hit ratio (hits / total lookups)",
            &["service"]
        )
        .unwrap();

        /// Total number of cache misses (key not found in cache).
        pub static ref JWKS_CACHE_MISS_TOTAL: IntCounterVec = register_int_counter_vec!(
            "jwks_cache_miss_total",
            "Total number of cache misses",
            &["service"]
        )
        .unwrap();

        /// Latency of JWKS fetches from the endpoint (milliseconds).
        pub static ref JWKS_FETCH_LATENCY_MS: HistogramVec = register_histogram_vec!(
            "jwks_fetch_latency_ms",
            "Time taken to fetch JWKS from endpoint",
            &["service", "status"],
            vec![1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0]
        )
        .unwrap();

        /// Total number of cache hits.
        pub static ref JWKS_CACHE_HIT_TOTAL: IntCounterVec = register_int_counter_vec!(
            "jwks_cache_hit_total",
            "Total number of cache hits",
            &["service"]
        )
        .unwrap();

        /// Number of stale keys in use (keys older than TTL but within tolerance).
        pub static ref JWKS_STALE_KEY_USAGE_TOTAL: IntCounterVec = register_int_counter_vec!(
            "jwks_stale_key_usage_total",
            "Total number of stale key uses",
            &["service"]
        )
        .unwrap();

        /// Number of background refresh failures.
        pub static ref JWKS_REFRESH_FAILURE_TOTAL: IntCounterVec = register_int_counter_vec!(
            "jwks_refresh_failures_total",
            "Total number of background refresh failures",
            &["service"]
        )
        .unwrap();
    }
}

// ---------------------------------------------------------------------------
// JwksCache
// ---------------------------------------------------------------------------

/// Internal state of the JWKS cache, wrapped in `ArcSwap` for lock-free reads.
pub(crate) struct JwksCacheInner {
    /// Keys indexed by `kid`.
    pub(crate) keys: HashMap<String, Jwk>,
    /// Timestamp of the last successful refresh.
    pub(crate) last_refresh: Option<Instant>,
    /// Whether the cache has been populated at least once.
    pub(crate) initialized: bool,
}

/// Builder for `JwksCache`.
///
/// Configures the endpoint, TTL, stale tolerance, and security limits.
pub struct JwksHealthCheck {
    /// Number of keys currently cached.
    pub key_count: usize,
    /// Key IDs in the cache.
    pub key_ids: Vec<String>,
    /// Last successful refresh time.
    pub last_refresh: Option<Instant>,
    /// Whether the cache has been initialized.
    pub initialized: bool,
    /// Stale tolerance duration.
    pub stale_tolerance: Duration,
    /// Refresh interval (TTL).
    pub refresh_interval: Duration,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
