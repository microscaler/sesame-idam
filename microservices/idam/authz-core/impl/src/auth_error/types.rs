//! Auth error types, metrics, and constants for version mismatch handling.
//!
//! This module provides the `AuthError` enum used throughout authz-core for
//! version mismatch and token validation failures. It maps to proper HTTP
//! responses with WWW-Authenticate and Retry-After headers per RFC 7235.

use prometheus::{Histogram, IntCounterVec, Registry};
use serde::Serialize;

// ─── Metrics ─────────────────────────────────────────────────────────────────

/// Metrics registry singleton for version mismatch tracking.
static VERSION_METRICS: std::sync::LazyLock<Result<VersionMismatchMetrics, String>> =
    std::sync::LazyLock::new(|| {
        let registry = Registry::new();
        VersionMismatchMetrics::register(&registry).map_err(|e| e.to_string())
    });

/// Version mismatch metrics counters and histograms.
pub struct VersionMismatchMetrics {
    /// Total version mismatch events, labeled by gap size category.
    pub version_mismatch_total: IntCounterVec,
    /// Latency of the version cache lookup (milliseconds).
    pub version_lookup_latency_ms: Histogram,
}

impl VersionMismatchMetrics {
    /// Create and register metrics with a Prometheus registry.
    pub fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        let version_mismatch_total = IntCounterVec::new(
            prometheus::Opts::new(
                "version_mismatch_total",
                "Total number of version mismatch events, labeled by gap size",
            ),
            &["result"],
        )?;

        let version_lookup_latency_ms = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "version_lookup_latency_ms",
                "Latency of the version cache lookup in milliseconds",
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        )?;

        registry.register(Box::new(version_mismatch_total.clone()))?;
        registry.register(Box::new(version_lookup_latency_ms.clone()))?;

        Ok(Self {
            version_mismatch_total,
            version_lookup_latency_ms,
        })
    }

    /// Record a version mismatch event (thread-safe via global lazy).
    pub fn record_mismatch(gap: GapSize) {
        let label = match gap {
            GapSize::Small => "small",
            GapSize::Large => "large",
            GapSize::Current => "current",
        };
        if let Ok(metrics) = VERSION_METRICS.as_ref() {
            metrics
                .version_mismatch_total
                .with_label_values(&[label])
                .inc();
        }
    }

    /// Record version cache lookup latency (thread-safe via global lazy).
    pub fn record_latency_ms(latency_ms: f64) {
        if let Ok(metrics) = VERSION_METRICS.as_ref() {
            metrics.version_lookup_latency_ms.observe(latency_ms);
        }
    }
}

/// Version mismatch gap threshold.
/// When the gap between cached and claimed version exceeds this value,
/// `retry_after` is set to 0 (immediate re-authentication required).
pub const VERSION_GAP_LARGE: u64 = 100;

/// Standard retry-after interval for small version gaps (seconds).
/// Clients may refresh their token within this window.
pub const RETRY_AFTER_SMALL_GAP: u64 = 300;

/// Machine-readable reason for `stale_auth_token` errors.
pub const REASON_STALE_AUTHZ_SNAPSHOT: &str = "stale_authz_snapshot";

/// Error code for version mismatch.
pub const ERROR_STALE_AUTH_TOKEN: &str = "stale_auth_token";

/// Human-friendly message for version mismatch.
pub const MESSAGE_STALE_AUTH_TOKEN: &str =
    "Your token has been revoked due to a privilege change. Please log in again.";

/// Version mismatch gap size categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapSize {
    /// Token version equals or exceeds cached version (no mismatch).
    Current,
    /// Small gap (1-10): allow token refresh within `retry_after` window.
    Small,
    /// Large gap (>100): immediate re-authentication required.
    Large,
}

/// Auth error variants for version mismatch and token validation.
#[derive(Debug, Clone, Serialize)]
pub enum AuthError {
    /// Token version is stale — claims.ver < `cached_ver`.
    ///
    /// The JWT contains a version snapshot that is older than the current
    /// authorization state. The client must refresh or re-authenticate
    /// to obtain a fresh token with the updated version.
    StaleAuthToken {
        /// Seconds to wait before retrying.
        /// 0 = immediate re-authentication required.
        retry_after: u64,
        /// The cached (current) version from the version store.
        expected_min_version: u64,
        /// The version claim embedded in the JWT.
        actual_version: u64,
    },
}
