use crate::jwks_cache::types::JwksCacheInner;
use crate::jwks_cache::{Jwk, JwksCacheError, JwksDocument, JwksHealthCheck};
use crate::http::{fetch_get, HttpFetchOptions};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

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
/// Default timeout for JWKS endpoint fetches.
const JWKS_FETCH_TIMEOUT: Duration = Duration::from_secs(10);
/// Maximum response body size (256 KB) — hard limit for `read_to_end`.
pub const MAX_BODY_READ_BYTES: usize = 256 * 1024;

// ---------------------------------------------------------------------------
// Cache metrics
// ---------------------------------------------------------------------------

#[cfg(feature = "metrics")]
mod metrics {
    use prometheus::{
        register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterVec,
    };

    lazy_static::lazy_static! {
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

/// JWKS (JSON Web Key Set) cache with background refresh, stale tolerance, and security protections.
///
/// This struct provides a service-level JWKS cache that eliminates the single-point-of-failure
/// caused by making HTTP calls to the JWKS endpoint on every JWT validation.
///
/// # Design
///
/// - **TTL**: Keys are cached for 5 minutes by default before background refresh.
/// - **Stale tolerance**: Even if the cache is stale (>5min), keys remain valid for 15 minutes
///   after last refresh, providing resilience during transient JWKS endpoint outages.
/// - **Fallback**: If the requested `kid` is not found, any cached key can be used as fallback.
/// - **Atomic replacement**: Background refresh replaces the entire key set atomically — no
///   partial state visible to concurrent readers.
pub struct JwksCache {
    /// JWKS endpoint URL (e.g., `https://idam.example.com/.well-known/jwks.json`).
    pub(crate) endpoint: String,
    /// How often to refresh the cache in the background.
    pub(crate) refresh_interval: Duration,
    /// Maximum age of cached keys before they are considered "expired" (not usable).
    pub(crate) stale_tolerance: Duration,
    /// Maximum number of keys in the JWKS response.
    max_keys: usize,
    /// Maximum size of a single JWK.
    max_key_size_bytes: usize,
    /// Maximum size of the entire JWKS document.
    max_jwks_size_bytes: usize,
    /// Service name for metrics.
    service_name: String,
    /// Cached keys, updated under an `RwLock`.
    pub(crate) inner: Arc<RwLock<JwksCacheInner>>,
    /// Whether background refresh is enabled.
    bg_refresh: bool,
    /// Single-flight gate: only one fetch in flight at a time.
    fetch_in_flight: FetchInFlight,
}

/// Tracks whether a fetch is in flight, shared across clones.
///
/// We use `Arc<AtomicBool>` so cloned caches share the same in-flight state.
struct FetchInFlight {
    in_flight: Arc<AtomicBool>,
}

impl FetchInFlight {
    fn new() -> Self {
        Self {
            in_flight: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Try to set the flag. Returns `true` if the flag was previously false (not in flight).
    fn try_set(&self) -> bool {
        self.in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
    }

    /// Reset the flag after a fetch completes.
    fn reset(&self) {
        self.in_flight.store(false, Ordering::Release);
    }
}

impl JwksCache {
    /// Create a builder for `JwksCache`.
    pub fn builder(endpoint: impl Into<String>) -> JwksCacheBuilder {
        JwksCacheBuilder::new(endpoint)
    }

    /// Create a default `JwksCache` with the given endpoint.
    ///
    /// Uses default TTL (5 min), stale tolerance (15 min), and background refresh enabled.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self::builder(endpoint).build()
    }

    /// Get a key by its `kid`.
    ///
    /// Returns `Err(JwksCacheError::NoKeysAvailable)` if the cache is empty,
    /// or the key is beyond stale tolerance.
    pub fn get_key(&self, kid: &str) -> Result<Jwk, JwksCacheError> {
        let inner = self.inner.read().unwrap();

        // Try cache hit first.
        if let Some(key) = inner.keys.get(kid) {
            #[cfg(feature = "metrics")]
            {
                metrics::JWKS_CACHE_HIT_TOTAL
                    .get_metric_with_label_values(&[&self.service_name])
                    .unwrap()
                    .inc();
            }
            return Ok(key.clone());
        }

        // Cache miss — check if cache is stale but within tolerance.
        let now = Instant::now();
        let is_stale = inner
            .last_refresh
            .is_none_or(|lr| now.duration_since(lr) > self.refresh_interval);

        let is_expired = inner
            .last_refresh
            .is_none_or(|lr| now.duration_since(lr) > self.stale_tolerance);

        if is_stale {
            // Stale but within tolerance — log warning and try background refresh.
            let age = inner
                .last_refresh
                .map_or(0, |lr| now.duration_since(lr).as_secs());

            tracing::warn!(
                kid,
                age_secs = age,
                stale_tolerance_secs = self.stale_tolerance.as_secs(),
                "Stale key used for validation"
            );

            #[cfg(feature = "metrics")]
            {
                metrics::JWKS_STALE_KEY_USAGE_TOTAL
                    .get_metric_with_label_values(&[&self.service_name])
                    .unwrap()
                    .inc();
            }

            // Trigger background refresh if not already in flight.
            if self.fetch_in_flight.try_set() {
                self.start_fetch();
            }

            // If still not found and not expired, fall back to any stale key.
            let stale_fallback = if is_expired {
                None
            } else {
                inner.keys.values().next().cloned()
            };

            // Try again with updated cache.
            drop(inner);
            let updated = self.inner.read().unwrap();
            if let Some(key) = updated.keys.get(kid) {
                return Ok(key.clone());
            }
            drop(updated);

            if let Some(key) = stale_fallback {
                tracing::warn!(
                    kid,
                    "Requested key not found, using fallback key: {}",
                    key.kid
                );
                return Ok(key);
            }

            return Err(JwksCacheError::KeyNotFound(kid.to_string()));
        }

        if is_expired {
            // Cache is expired — fetch immediately.
            drop(inner);
            self.refresh()?;

            let updated = self.inner.read().unwrap();
            if let Some(key) = updated.keys.get(kid) {
                return Ok(key.clone());
            }

            // Fallback to any key.
            let fallback = updated.keys.values().next().cloned();
            return match fallback {
                Some(key) => {
                    tracing::warn!(
                        kid,
                        "Cache expired, requested key not found, using fallback key: {}",
                        key.kid
                    );
                    Ok(key)
                }
                None => Err(JwksCacheError::KeyNotFound(kid.to_string())),
            };
        }

        // Not stale, key simply not in cache. Do NOT fall back to a different
        // key here — signature verification would fail anyway, and silently
        // substituting keys hides rotation problems. Callers that want a
        // best-effort key can use `get_any_valid_key()`.
        #[cfg(feature = "metrics")]
        {
            metrics::JWKS_CACHE_MISS_TOTAL
                .get_metric_with_label_values(&[&self.service_name])
                .unwrap()
                .inc();
        }

        Err(JwksCacheError::KeyNotFound(kid.to_string()))
    }

    /// Get any valid key from the cache (fallback for key rotation).
    ///
    /// Returns the first available key regardless of `kid`.
    /// Useful when the JWT header's `kid` is missing or unrecognized.
    pub fn get_any_valid_key(&self) -> Result<Jwk, JwksCacheError> {
        let inner = self.inner.read().unwrap();

        match inner.keys.values().next() {
            Some(key) => {
                #[cfg(feature = "metrics")]
                {
                    metrics::JWKS_CACHE_HIT_TOTAL
                        .get_metric_with_label_values(&[&self.service_name])
                        .unwrap()
                        .inc();
                }
                Ok(key.clone())
            }
            None => Err(JwksCacheError::NoKeysAvailable),
        }
    }

    /// Refresh the cache by fetching from the JWKS endpoint.
    ///
    /// This performs a full fetch and atomically replaces the cache.
    pub fn refresh(&self) -> Result<usize, JwksCacheError> {
        let now = std::time::Instant::now();

        // Fetch via BRRTRouter coroutine HTTP client
        let body_bytes = Self::fetch_from_endpoint(&self.endpoint, self.max_jwks_size_bytes)?;

        let status = 200u16;

        #[cfg(feature = "metrics")]
        {
            let elapsed = now.elapsed().as_millis() as f64;
            metrics::JWKS_FETCH_LATENCY_MS
                .get_metric_with_label_values(&[&self.service_name, &status.to_string()])
                .unwrap()
                .observe(elapsed);
        }

        // Validate document size before parsing (HACK-712).
        if body_bytes.len() > self.max_jwks_size_bytes {
            tracing::warn!(
                content_length = body_bytes.len(),
                max_size = self.max_jwks_size_bytes,
                "JWKS document exceeds maximum size, rejecting"
            );

            #[cfg(feature = "metrics")]
            {
                metrics::JWKS_REFRESH_FAILURE_TOTAL
                    .get_metric_with_label_values(&[&self.service_name])
                    .unwrap()
                    .inc();
            }

            return Err(JwksCacheError::DocumentTooLarge {
                max: self.max_jwks_size_bytes,
                actual: body_bytes.len(),
            });
        }

        // Parse JWKS.
        let jwks: JwksDocument = serde_json::from_slice(&body_bytes)
            .map_err(|e| JwksCacheError::ParseError(e.to_string()))?;

        // Validate key count (HACK-712).
        if jwks.keys.len() > self.max_keys {
            tracing::warn!(
                key_count = jwks.keys.len(),
                max_keys = self.max_keys,
                "JWKS document exceeds maximum key count"
            );

            #[cfg(feature = "metrics")]
            {
                metrics::JWKS_REFRESH_FAILURE_TOTAL
                    .get_metric_with_label_values(&[&self.service_name])
                    .unwrap()
                    .inc();
            }

            return Err(JwksCacheError::TooManyKeys {
                max: self.max_keys,
                actual: jwks.keys.len(),
            });
        }

        // Validate each key's size and collect into HashMap.
        let mut new_keys = HashMap::with_capacity(jwks.keys.len());
        for key in &jwks.keys {
            // Serialize key to JSON and check size.
            let key_json =
                serde_json::to_vec(key).map_err(|e| JwksCacheError::ParseError(e.to_string()))?;

            if key_json.len() > self.max_key_size_bytes {
                tracing::warn!(
                    kid = key.kid,
                    key_size = key_json.len(),
                    max_key_size = self.max_key_size_bytes,
                    "JWK exceeds maximum size, skipping"
                );
                continue; // Skip oversized keys instead of rejecting entire document.
            }

            new_keys.insert(key.kid.clone(), key.clone());
        }

        let new_keys_len = new_keys.len();

        // Atomic replacement (HACK-711).
        let new_inner = JwksCacheInner {
            keys: new_keys,
            last_refresh: Some(Instant::now()),
            initialized: true,
        };

        *self.inner.write().unwrap() = new_inner;

        tracing::info!(
            key_count = new_keys_len,
            "JWKS cache refreshed successfully"
        );

        Ok(new_keys_len)
    }

    /// Start background refresh loop (non-blocking).
    ///
    /// Spawns a may coroutine that refreshes the cache at the configured interval.
    /// Returns immediately without blocking the caller.
    pub fn start_background_refresh(&self) {
        if !self.bg_refresh {
            return;
        }

        // Initial fetch to populate cache.
        if let Err(e) = self.refresh() {
            tracing::warn!(
                error = %e,
                "Initial JWKS fetch failed, will retry on next interval"
            );
        }

        // Spawn background refresh loop.
        let endpoint = self.endpoint.clone();
        let interval = self.refresh_interval;
        let max_keys = self.max_keys;
        let max_key_size = self.max_key_size_bytes;
        let max_jwks_size = self.max_jwks_size_bytes;
        let service_name = self.service_name.clone();
        let inner = Arc::clone(&self.inner);
        let fetch_flag = FetchInFlight {
            in_flight: Arc::clone(&self.fetch_in_flight.in_flight),
        };

        may::go!(move || {
            loop {
                std::thread::sleep(interval);

                // Single-flight: only one fetch in flight.
                if fetch_flag.try_set() {
                    let now = std::time::Instant::now();

                    match Self::do_fetch(
                        &endpoint,
                        max_keys,
                        max_key_size,
                        max_jwks_size,
                        &service_name,
                    ) {
                        Ok((count, new_inner)) => {
                            tracing::info!(
                                key_count = count,
                                elapsed_ms = now.elapsed().as_millis(),
                                "Background JWKS refresh successful"
                            );

                            // Update the cache on the original instance.
                            if let Ok(mut guard) = inner.write() {
                                *guard = new_inner;
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                elapsed_ms = now.elapsed().as_millis(),
                                "Background JWKS refresh failed"
                            );

                            #[cfg(feature = "metrics")]
                            {
                                metrics::JWKS_REFRESH_FAILURE_TOTAL
                                    .get_metric_with_label_values(&[&service_name])
                                    .unwrap()
                                    .inc();
                            }
                        }
                    }
                }
            }
        });
    }

    /// Perform a fetch with single-flight deduplication.
    fn start_fetch(&self) {
        let result = self.refresh();
        self.fetch_in_flight.reset();
        match result {
            Ok(count) => {
                tracing::info!(key_count = count, "On-demand JWKS refresh successful");
            }
            Err(e) => {
                tracing::error!(error = %e, "On-demand JWKS refresh failed");
            }
        }
    }

    /// Internal fetch with single-flight deduplication for background tasks.
    fn do_fetch(
        endpoint: &str,
        max_keys: usize,
        max_key_size: usize,
        max_jwks_size: usize,
        service_name: &str,
    ) -> Result<(usize, JwksCacheInner), JwksCacheError> {
        let now = std::time::Instant::now();

        let body_bytes = Self::fetch_from_endpoint(endpoint, max_jwks_size)?;

        let status = 200u16; // Only reached on success

        #[cfg(feature = "metrics")]
        {
            let elapsed = now.elapsed().as_millis() as f64;
            metrics::JWKS_FETCH_LATENCY_MS
                .get_metric_with_label_values(&[service_name, &status.to_string()])
                .unwrap()
                .observe(elapsed);
        }

        if body_bytes.len() > max_jwks_size {
            return Err(Self::size_error(max_jwks_size, body_bytes.len()));
        }

        let jwks: JwksDocument = serde_json::from_slice(&body_bytes)
            .map_err(|e| JwksCacheError::ParseError(e.to_string()))?;

        if jwks.keys.len() > max_keys {
            return Err(JwksCacheError::TooManyKeys {
                max: max_keys,
                actual: jwks.keys.len(),
            });
        }

        let mut new_keys = HashMap::with_capacity(jwks.keys.len());
        for key in &jwks.keys {
            let key_json =
                serde_json::to_vec(key).map_err(|e| JwksCacheError::ParseError(e.to_string()))?;
            if key_json.len() > max_key_size {
                continue; // Skip oversized keys.
            }
            new_keys.insert(key.kid.clone(), key.clone());
        }

        let new_inner = JwksCacheInner {
            keys: new_keys,
            last_refresh: Some(Instant::now()),
            initialized: true,
        };

        Ok((new_inner.keys.len(), new_inner))
    }

    /// Fetch JWKS bytes from `endpoint` using [`crate::http::fetch_get`].
    fn fetch_from_endpoint(endpoint: &str, max_body: usize) -> Result<Vec<u8>, JwksCacheError> {
        let options = HttpFetchOptions {
            timeout: JWKS_FETCH_TIMEOUT,
            max_body_bytes: max_body.min(MAX_BODY_READ_BYTES),
            extra_headers: Vec::new(),
        };
        let (status, body) = fetch_get(endpoint, &options).map_err(|e| JwksCacheError::FetchError {
            status: 0,
            message: e.to_string(),
        })?;
        if !(200..300).contains(&status) {
            return Err(JwksCacheError::FetchError {
                status,
                message: format!("HTTP {status}"),
            });
        }
        Ok(body)
    }

    /// Helper for size error.
    fn size_error(max: usize, actual: usize) -> JwksCacheError {
        if actual > 10000 {
            JwksCacheError::DocumentTooLarge { max, actual }
        } else {
            JwksCacheError::KeyTooLarge { max, actual }
        }
    }

    /// Get the current number of keys in the cache.
    #[must_use]
    pub fn key_count(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.keys.len()
    }

    /// Get the key IDs currently in the cache.
    #[must_use]
    pub fn key_ids(&self) -> Vec<String> {
        let inner = self.inner.read().unwrap();
        inner.keys.keys().cloned().collect()
    }

    /// Get the last refresh time, if any.
    #[must_use]
    pub fn last_refresh(&self) -> Option<Instant> {
        let inner = self.inner.read().unwrap();
        inner.last_refresh
    }

    /// Check if the cache has been initialized (at least one successful fetch).
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        let inner = self.inner.read().unwrap();
        inner.initialized
    }

    /// Get cache statistics for health checks.
    #[must_use]
    pub fn health_check(&self) -> JwksHealthCheck {
        let inner = self.inner.read().unwrap();
        JwksHealthCheck {
            key_count: inner.keys.len(),
            key_ids: inner.keys.keys().cloned().collect(),
            last_refresh: inner.last_refresh,
            initialized: inner.initialized,
            stale_tolerance: self.stale_tolerance,
            refresh_interval: self.refresh_interval,
        }
    }

    /// Clear the cache (useful for testing or cache invalidation).
    pub fn clear(&self) {
        *self.inner.write().unwrap() = JwksCacheInner {
            keys: HashMap::new(),
            last_refresh: None,
            initialized: false,
        };
    }

    /// Manually update cache with provided keys (useful for testing).
    pub fn update_keys(&self, keys: HashMap<String, Jwk>) {
        *self.inner.write().unwrap() = JwksCacheInner {
            keys,
            last_refresh: Some(Instant::now()),
            initialized: true,
        };
    }
}

impl std::fmt::Debug for JwksCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwksCache")
            .field("endpoint", &self.endpoint)
            .field("refresh_interval", &self.refresh_interval)
            .field("stale_tolerance", &self.stale_tolerance)
            .field("max_keys", &self.max_keys)
            .field("key_count", &self.key_count())
            .finish()
    }
}

impl Clone for JwksCache {
    fn clone(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            refresh_interval: self.refresh_interval,
            stale_tolerance: self.stale_tolerance,
            max_keys: self.max_keys,
            max_key_size_bytes: self.max_key_size_bytes,
            max_jwks_size_bytes: self.max_jwks_size_bytes,
            service_name: self.service_name.clone(),
            inner: self.inner.clone(),
            bg_refresh: self.bg_refresh,
            fetch_in_flight: FetchInFlight {
                in_flight: Arc::clone(&self.fetch_in_flight.in_flight),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// JwksCacheBuilder
// ---------------------------------------------------------------------------

/// Builder for `JwksCache`.
///
/// Configures the endpoint, TTL, stale tolerance, and security limits.
pub struct JwksCacheBuilder {
    endpoint: String,
    refresh_interval: Duration,
    stale_tolerance: Duration,
    max_keys: usize,
    max_key_size_bytes: usize,
    max_jwks_size_bytes: usize,
    service_name: String,
    /// Enable background refresh. Default: true.
    background_refresh: bool,
}

impl JwksCacheBuilder {
    /// Create a new builder.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            refresh_interval: Duration::from_secs(DEFAULT_REFRESH_INTERVAL_SECS),
            stale_tolerance: Duration::from_secs(DEFAULT_STALE_TOLERANCE_SECS),
            max_keys: DEFAULT_MAX_KEYS,
            max_key_size_bytes: DEFAULT_MAX_KEY_SIZE_BYTES,
            max_jwks_size_bytes: DEFAULT_MAX_JWKS_SIZE_BYTES,
            service_name: "default".to_string(),
            background_refresh: true,
        }
    }

    /// Set the cache refresh interval (TTL). Default: 5 minutes.
    #[must_use]
    pub fn refresh_interval(mut self, interval: Duration) -> Self {
        self.refresh_interval = interval;
        self
    }

    /// Set the stale tolerance duration. Default: 15 minutes.
    ///
    /// Keys remain valid up to this duration after last refresh.
    #[must_use]
    pub fn stale_tolerance(mut self, tolerance: Duration) -> Self {
        self.stale_tolerance = tolerance;
        self
    }

    /// Set the maximum number of keys allowed in the cache.
    ///
    /// Responses with more keys are rejected (security: HACK-712).
    #[must_use]
    pub fn max_keys(mut self, max: usize) -> Self {
        self.max_keys = max;
        self
    }

    /// Set the maximum size (in bytes) for a single JWK.
    ///
    /// Oversized keys are rejected to prevent memory exhaustion (HACK-712).
    #[must_use]
    pub fn max_key_size_bytes(mut self, max: usize) -> Self {
        self.max_key_size_bytes = max;
        self
    }

    /// Set the maximum size (in bytes) for the entire JWKS document.
    ///
    /// Oversized documents are rejected (HACK-712).
    #[must_use]
    pub fn max_jwks_size_bytes(mut self, max: usize) -> Self {
        self.max_jwks_size_bytes = max;
        self
    }

    /// Set the service name for metrics labels.
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    /// Disable automatic background refresh.
    ///
    /// When disabled, the cache must be refreshed manually via `refresh()` or
    /// `get_key()` will trigger an on-demand fetch if the cache is empty.
    #[must_use]
    pub fn no_background_refresh(mut self) -> Self {
        self.background_refresh = false;
        self
    }

    /// Build the `JwksCache`.
    #[must_use]
    pub fn build(self) -> JwksCache {
        JwksCache {
            endpoint: self.endpoint,
            refresh_interval: self.refresh_interval,
            stale_tolerance: self.stale_tolerance,
            max_keys: self.max_keys,
            max_key_size_bytes: self.max_key_size_bytes,
            max_jwks_size_bytes: self.max_jwks_size_bytes,
            service_name: self.service_name,
            inner: Arc::new(RwLock::new(JwksCacheInner {
                keys: HashMap::new(),
                last_refresh: None,
                initialized: false,
            })),
            bg_refresh: self.background_refresh,
            fetch_in_flight: FetchInFlight::new(),
        }
    }
}

impl std::fmt::Debug for JwksCacheBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwksCacheBuilder")
            .field("endpoint", &self.endpoint)
            .field("refresh_interval", &self.refresh_interval)
            .field("stale_tolerance", &self.stale_tolerance)
            .field("max_keys", &self.max_keys)
            .field("service_name", &self.service_name)
            .field("background_refresh", &self.background_refresh)
            .finish()
    }
}
