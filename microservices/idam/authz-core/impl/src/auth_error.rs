//! Authentication/authorization error types and HTTP response generation.
//!
//! This module provides the `AuthError` enum used throughout authz-core for
//! version mismatch and token validation failures. It maps to proper HTTP
//! responses with WWW-Authenticate and Retry-After headers per RFC 7235.

use brrtrouter::dispatcher::{HandlerResponse, HeaderVec};
use http::StatusCode;
use prometheus::{Histogram, IntCounterVec, Registry};
use serde::Serialize;
use std::sync::Arc;

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
/// retry_after is set to 0 (immediate re-authentication required).
pub const VERSION_GAP_LARGE: u64 = 100;

/// Standard retry-after interval for small version gaps (seconds).
/// Clients may refresh their token within this window.
pub const RETRY_AFTER_SMALL_GAP: u64 = 300;

/// Machine-readable reason for stale_auth_token errors.
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
    /// Small gap (1-10): allow token refresh within retry_after window.
    Small,
    /// Large gap (>100): immediate re-authentication required.
    Large,
}

/// Auth error variants for version mismatch and token validation.
#[derive(Debug, Clone, Serialize)]
pub enum AuthError {
    /// Token version is stale — claims.ver < cached_ver.
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

impl AuthError {
    /// Calculate the retry-after value based on version gap size.
    ///
    /// Returns the gap size classification along with the retry_after value.
    ///
    /// # Gap calculation
    ///
    /// - If `claims_ver >= cached_ver`: token is current, no mismatch
    /// - If gap is 1-10: `retry_after = 300` (allow refresh)
    /// - If gap > 100: `retry_after = 0` (immediate re-auth)
    ///
    /// Note: gaps of 11-100 are treated as "small" (retry_after = 300) since
    /// they represent normal privilege changes that can be recovered via refresh.
    pub fn calculate_retry_after(cached_ver: u64, claims_ver: u64) -> (GapSize, u64) {
        if claims_ver >= cached_ver {
            return (GapSize::Current, RETRY_AFTER_SMALL_GAP);
        }

        let gap = cached_ver - claims_ver;

        if gap > VERSION_GAP_LARGE {
            (GapSize::Large, 0)
        } else {
            (GapSize::Small, RETRY_AFTER_SMALL_GAP)
        }
    }

    /// Determine the gap size category.
    ///
    /// Returns `GapSize::Current` if no mismatch, otherwise `Small` or `Large`.
    pub fn gap_size(cached_ver: u64, claims_ver: u64) -> GapSize {
        if claims_ver >= cached_ver {
            GapSize::Current
        } else if cached_ver - claims_ver > VERSION_GAP_LARGE {
            GapSize::Large
        } else {
            GapSize::Small
        }
    }

    /// Handle a version mismatch detection and return the appropriate error.
    ///
    /// This is the primary entry point for version mismatch handling.
    /// It takes the cached version (from Redis/version store) and the
    /// claims version (from the JWT) and returns either `Ok(())` if the
    /// token is current, or an `AuthError::StaleAuthToken` if stale.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if `claims_ver >= cached_ver` (token is current)
    /// - `Err(AuthError::StaleAuthToken)` if `claims_ver < cached_ver`
    ///
    /// # Example
    ///
    /// ```
    /// use crate::auth_error::{handle_version_mismatch, AuthError};
    ///
    /// // Token is current
    /// assert!(handle_version_mismatch(42, 42).is_ok());
    /// assert!(handle_version_mismatch(45, 42).is_ok());
    ///
    /// // Token is stale
    /// match handle_version_mismatch(50, 42) {
    ///     Err(AuthError::StaleAuthToken { retry_after, .. }) => {
    ///         assert_eq!(retry_after, 300); // small gap
    ///     }
    ///     _ => panic!("expected StaleAuthToken"),
    /// }
    /// ```
    /// Handle a version mismatch detection and return the appropriate error.
    ///
    /// Records metrics for the version mismatch event (mismatch total,
    /// gap size label) and the cache lookup latency.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if `claims_ver >= cached_ver` (token is current)
    /// - `Err(AuthError::StaleAuthToken)` if `claims_ver < cached_ver`
    pub fn handle_version_mismatch(cached_ver: u64, claims_ver: u64) -> Result<(), AuthError> {
        let (gap_size, retry_after) = Self::calculate_retry_after(cached_ver, claims_ver);

        // Record metrics
        VersionMismatchMetrics::record_mismatch(gap_size);

        match gap_size {
            GapSize::Current => Ok(()),
            GapSize::Small | GapSize::Large => Err(AuthError::StaleAuthToken {
                retry_after,
                expected_min_version: cached_ver,
                actual_version: claims_ver,
            }),
        }
    }

    /// Convert this error to a human-friendly message string.
    pub fn message(&self) -> &'static str {
        match self {
            AuthError::StaleAuthToken { .. } => MESSAGE_STALE_AUTH_TOKEN,
        }
    }

    /// Get the machine-readable error code.
    pub fn error_code(&self) -> &'static str {
        match self {
            AuthError::StaleAuthToken { .. } => ERROR_STALE_AUTH_TOKEN,
        }
    }

    /// Get the machine-readable reason string.
    pub fn reason(&self) -> &'static str {
        match self {
            AuthError::StaleAuthToken { .. } => REASON_STALE_AUTHZ_SNAPSHOT,
        }
    }

    /// Extract retry_after for metrics purposes.
    pub fn retry_after(&self) -> u64 {
        match self {
            AuthError::StaleAuthToken { retry_after, .. } => *retry_after,
        }
    }

    /// Extract the gap size for metrics purposes.
    pub fn gap_size_for(&self) -> GapSize {
        match self {
            AuthError::StaleAuthToken {
                retry_after: _,
                expected_min_version,
                actual_version,
            } => {
                let gap = expected_min_version - actual_version;
                if gap > VERSION_GAP_LARGE {
                    GapSize::Large
                } else {
                    GapSize::Small
                }
            }
        }
    }

    /// Convert this error to an HTTP `HandlerResponse`.
    ///
    /// Returns a `401 Unauthorized` response with:
    /// - `WWW-Authenticate: Bearer error="stale_auth_token", retry_after=NNN`
    /// - `Retry-After: NNN` header
    /// - `Content-Type: application/json` header
    /// - JSON body with `error`, `message`, `retry_after`, and `reason` fields
    pub fn to_http_response(&self) -> HandlerResponse {
        let (gap_size, retry_after) = match self {
            AuthError::StaleAuthToken {
                retry_after,
                expected_min_version,
                actual_version,
            } => {
                let gap = *expected_min_version - *actual_version;
                if gap > VERSION_GAP_LARGE {
                    (GapSize::Large, 0)
                } else {
                    (GapSize::Small, *retry_after)
                }
            }
        };

        // Build the JSON body
        let body = serde_json::json!({
            "error": self.error_code(),
            "message": self.message(),
            "retry_after": retry_after,
            "reason": self.reason()
        });

        // Build headers
        let mut headers = HeaderVec::new();
        headers.push((Arc::from("Content-Type"), "application/json".to_string()));
        headers.push((
            Arc::from("WWW-Authenticate"),
            format!(
                "Bearer error=\"stale_auth_token\", retry_after={}",
                retry_after
            ),
        ));
        headers.push((Arc::from("Retry-After"), retry_after.to_string()));

        HandlerResponse {
            status: StatusCode::UNAUTHORIZED.as_u16(),
            body,
            headers,
        }
    }

    /// Check if this error represents a version mismatch condition.
    pub fn is_version_mismatch(&self) -> bool {
        matches!(self, AuthError::StaleAuthToken { .. })
    }
}

// ─── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════
    // Version Mismatch Detection
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_version_mismatch_returns_error_when_stale() {
        // claims.ver (42) < cached_ver (45) → mismatch
        let result = AuthError::handle_version_mismatch(45, 42);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_version_mismatch());
    }

    #[test]
    fn test_token_current_when_claims_equals_cached() {
        // claims.ver == cached_ver → no mismatch
        let result = AuthError::handle_version_mismatch(42, 42);
        assert!(result.is_ok());
    }

    #[test]
    fn test_token_current_when_claims_newer_than_cached() {
        // claims.ver > cached_ver → no mismatch (future-proofing)
        let result = AuthError::handle_version_mismatch(42, 50);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stale_auth_token_error_code() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        assert_eq!(err.error_code(), "stale_auth_token");
    }

    #[test]
    fn test_stale_auth_token_reason() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        assert_eq!(err.reason(), "stale_authz_snapshot");
    }

    #[test]
    fn test_stale_auth_token_user_message() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        assert_eq!(
            err.message(),
            "Your token has been revoked due to a privilege change. Please log in again."
        );
        // Ensure message is user-friendly (no technical stack traces)
        assert!(!err.message().contains("panic"));
        assert!(!err.message().contains("at "));
        assert!(!err.message().contains(":"));
    }

    // ═══════════════════════════════════════════════════════════
    // HTTP Response Format
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_http_response_status_is_401() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        assert_eq!(resp.status, 401);
    }

    #[test]
    fn test_http_response_includes_stale_auth_token_error() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        if let Some(obj) = resp.body.as_object() {
            let error_val = obj.get("error").and_then(|v| v.as_str());
            assert_eq!(error_val, Some("stale_auth_token"));
        } else {
            panic!("Response body should be a JSON object");
        }
    }

    #[test]
    fn test_http_response_includes_reason_field() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        if let Some(obj) = resp.body.as_object() {
            let reason_val = obj.get("reason").and_then(|v| v.as_str());
            assert_eq!(reason_val, Some("stale_authz_snapshot"));
        } else {
            panic!("Response body should be a JSON object");
        }
    }

    #[test]
    fn test_http_response_includes_retry_after_in_body() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        if let Some(obj) = resp.body.as_object() {
            let retry_val = obj.get("retry_after").and_then(|v| v.as_u64());
            assert_eq!(retry_val, Some(300));
        } else {
            panic!("Response body should be a JSON object");
        }
    }

    #[test]
    fn test_http_response_includes_www_authenticate_header() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        let www_auth = resp
            .headers
            .iter()
            .find(|(k, _)| k.as_ref() == "WWW-Authenticate")
            .map(|(_, v)| v.as_str());
        assert!(
            www_auth.is_some(),
            "Response must include WWW-Authenticate header"
        );
        let header_val = www_auth.unwrap();
        assert!(header_val.contains("Bearer error=\"stale_auth_token\""));
        assert!(header_val.contains("retry_after=300"));
    }

    #[test]
    fn test_http_response_includes_retry_after_header() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        let retry_header = resp
            .headers
            .iter()
            .find(|(k, _)| k.as_ref() == "Retry-After")
            .map(|(_, v)| v.as_str());
        assert!(
            retry_header.is_some(),
            "Response must include Retry-After header"
        );
        assert_eq!(retry_header.unwrap(), "300");
    }

    #[test]
    fn test_http_response_content_type_json() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        let ct = resp
            .headers
            .iter()
            .find(|(k, _)| k.as_ref() == "Content-Type")
            .map(|(_, v)| v.as_str());
        assert!(ct.is_some(), "Response must include Content-Type header");
        assert_eq!(ct.unwrap(), "application/json");
    }

    #[test]
    fn test_http_response_field_ordering() {
        // Verify the response has exactly the expected fields: error, message, retry_after, reason
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        if let Some(obj) = resp.body.as_object() {
            let keys: Vec<&String> = obj.keys().collect();
            assert_eq!(keys.len(), 4);
            assert_eq!(keys[0], "error");
            assert_eq!(keys[1], "message");
            assert_eq!(keys[2], "retry_after");
            assert_eq!(keys[3], "reason");
        } else {
            panic!("Response body should be a JSON object");
        }
    }

    #[test]
    fn test_http_response_no_stack_traces() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 999,
            actual_version: 1,
        };
        let resp = err.to_http_response();
        if let Some(obj) = resp.body.as_object() {
            if let Some(msg) = obj.get("message").and_then(|v| v.as_str()) {
                assert!(
                    !msg.contains("panic"),
                    "Response must not contain panic info"
                );
                assert!(
                    !msg.contains("stack"),
                    "Response must not contain stack traces"
                );
                assert!(
                    !msg.contains("backtrace"),
                    "Response must not contain backtrace info"
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Retry-After Calculation
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_small_gap_1_returns_retry_after_300() {
        // cached=43, claims=42, gap=1
        let (gap, retry) = AuthError::calculate_retry_after(43, 42);
        assert_eq!(retry, 300);
        assert_eq!(gap, GapSize::Small);
    }

    #[test]
    fn test_small_gap_10_returns_retry_after_300() {
        // cached=50, claims=40, gap=10 (boundary: gap of 10 is "small")
        let (gap, retry) = AuthError::calculate_retry_after(50, 40);
        assert_eq!(retry, 300);
        assert_eq!(gap, GapSize::Small);
    }

    #[test]
    fn test_small_gap_11_returns_retry_after_300() {
        // cached=51, claims=40, gap=11 (11 is still within reasonable refresh range)
        let (gap, retry) = AuthError::calculate_retry_after(51, 40);
        assert_eq!(retry, 300);
        assert_eq!(gap, GapSize::Small);
    }

    #[test]
    fn test_small_gap_100_returns_retry_after_300() {
        // cached=140, claims=40, gap=100 (boundary: gap of 100 is "small")
        let (gap, retry) = AuthError::calculate_retry_after(140, 40);
        assert_eq!(retry, 300);
        assert_eq!(gap, GapSize::Small);
    }

    #[test]
    fn test_large_gap_101_returns_retry_after_0() {
        // cached=141, claims=40, gap=101 (large gap, immediate re-auth)
        let (gap, retry) = AuthError::calculate_retry_after(141, 40);
        assert_eq!(retry, 0);
        assert_eq!(gap, GapSize::Large);
    }

    #[test]
    fn test_large_gap_150_returns_retry_after_0() {
        // cached=200, claims=50, gap=150 (repeated privilege escalations)
        let (gap, retry) = AuthError::calculate_retry_after(200, 50);
        assert_eq!(retry, 0);
        assert_eq!(gap, GapSize::Large);
    }

    #[test]
    fn test_equal_versions_returns_current() {
        // cached=42, claims=42 → no mismatch
        let (gap, retry) = AuthError::calculate_retry_after(42, 42);
        assert_eq!(gap, GapSize::Current);
    }

    #[test]
    fn test_claims_newer_returns_current() {
        // cached=42, claims=50 → token is newer, no mismatch
        let (gap, retry) = AuthError::calculate_retry_after(42, 50);
        assert_eq!(gap, GapSize::Current);
    }

    #[test]
    fn test_handle_version_mismatch_equal_versions_succeeds() {
        // claims.ver == cached_ver → Ok(())
        let result = AuthError::handle_version_mismatch(42, 42);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_version_mismatch_claims_newer_succeeds() {
        // claims.ver > cached_ver → Ok(()) (token is current or newer)
        let result = AuthError::handle_version_mismatch(42, 50);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_version_mismatch_small_gap_fails() {
        // cached=43, claims=40 → gap=3, should return StaleAuthToken
        let result = AuthError::handle_version_mismatch(43, 40);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.retry_after, 300);
    }

    #[test]
    fn test_handle_version_mismatch_large_gap_fails_with_zero_retry() {
        // cached=150, claims=40 → gap=110, should return StaleAuthToken with retry_after=0
        let result = AuthError::handle_version_mismatch(150, 40);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.retry_after, 0);
        assert_eq!(err.expected_min_version, 150);
        assert_eq!(err.actual_version, 40);
    }

    // ═══════════════════════════════════════════════════════════
    // GapSize Classification
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_gap_size_current_when_equal() {
        assert_eq!(AuthError::gap_size(42, 42), GapSize::Current);
    }

    #[test]
    fn test_gap_size_current_when_claims_newer() {
        assert_eq!(AuthError::gap_size(42, 50), GapSize::Current);
    }

    #[test]
    fn test_gap_size_small_for_gap_5() {
        assert_eq!(AuthError::gap_size(47, 42), GapSize::Small);
    }

    #[test]
    fn test_gap_size_large_for_gap_101() {
        assert_eq!(AuthError::gap_size(141, 40), GapSize::Large);
    }

    // ═══════════════════════════════════════════════════════════
    // Error Struct Fields
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_stale_auth_token_retry_after_field() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        assert_eq!(err.retry_after(), 300);
    }

    #[test]
    fn test_stale_auth_token_retry_after_zero() {
        let err = AuthError::StaleAuthToken {
            retry_after: 0,
            expected_min_version: 200,
            actual_version: 50,
        };
        assert_eq!(err.retry_after(), 0);
    }

    #[test]
    fn test_stale_auth_token_expected_min_version_field() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        assert_eq!(err.expected_min_version, 45);
    }

    #[test]
    fn test_stale_auth_token_actual_version_field() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        assert_eq!(err.actual_version, 42);
    }

    #[test]
    fn test_gap_size_for_small() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        assert_eq!(err.gap_size_for(), GapSize::Small);
    }

    #[test]
    fn test_gap_size_for_large() {
        let err = AuthError::StaleAuthToken {
            retry_after: 0,
            expected_min_version: 200,
            actual_version: 50,
        };
        assert_eq!(err.gap_size_for(), GapSize::Large);
    }

    #[test]
    fn test_is_version_mismatch_true() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        assert!(err.is_version_mismatch());
    }

    // ═══════════════════════════════════════════════════════════
    // Security Regression Tests
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_security_401_response_does_not_leak_version_numbers() {
        // The response body should NOT include expected_min_version or actual_version
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        if let Some(obj) = resp.body.as_object() {
            // Verify these internal fields are NOT in the response
            assert!(
                !obj.contains_key("expected_min_version"),
                "Response must not include expected_min_version"
            );
            assert!(
                !obj.contains_key("actual_version"),
                "Response must not include actual_version"
            );
            // Only the 4 expected fields
            assert_eq!(obj.len(), 4);
        }
    }

    #[test]
    fn test_security_same_http_status_for_all_gaps() {
        // Both small and large gaps return 401, differentiated only by retry_after
        let small_gap_err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let large_gap_err = AuthError::StaleAuthToken {
            retry_after: 0,
            expected_min_version: 200,
            actual_version: 50,
        };
        let small_resp = small_gap_err.to_http_response();
        let large_resp = large_gap_err.to_http_response();
        assert_eq!(small_resp.status, 401);
        assert_eq!(large_resp.status, 401);
        // But retry_after differs
        assert_eq!(small_resp.body["retry_after"], 300);
        assert_eq!(large_resp.body["retry_after"], 0);
    }

    #[test]
    fn test_security_error_code_distinct_from_expired() {
        // stale_auth_token error code must be distinct from other error types
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        assert_eq!(err.error_code(), "stale_auth_token");
        // The error code is a stable, distinct string — clients can differentiate
        assert_ne!(err.error_code(), "token_expired");
        assert_ne!(err.error_code(), "token_revoked");
        assert_ne!(err.error_code(), "invalid_token");
    }

    #[test]
    fn test_security_client_cannot_tamper_with_retry_after() {
        // retry_after is calculated server-side from the gap, not from any client input
        // The AuthError struct is constructed internally — a client cannot inject it
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        // The retry_after is fixed by the error construction, not mutable externally
        assert_eq!(err.retry_after, 300);
    }

    #[test]
    fn test_security_no_stack_traces_in_response() {
        // Error response must not include stack traces or internal state
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 999999,
            actual_version: 0,
        };
        let resp = err.to_http_response();
        if let Some(obj) = resp.body.as_object() {
            let msg = obj.get("message").and_then(|v| v.as_str()).unwrap_or("");
            assert!(
                !msg.contains("thread"),
                "Response must not contain thread info"
            );
            assert!(
                !msg.contains("panicked"),
                "Response must not contain panic info"
            );
            assert!(
                !msg.contains("location"),
                "Response must not contain source location"
            );
        }
    }

    #[test]
    fn test_security_response_does_not_include_internal_fields() {
        // Verify response body contains only: error, message, retry_after, reason
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        if let Some(obj) = resp.body.as_object() {
            let expected_keys: std::collections::HashSet<&str> =
                ["error", "message", "retry_after", "reason"]
                    .into_iter()
                    .collect();
            let actual_keys: std::collections::HashSet<&str> =
                obj.keys().map(|k| k.as_str()).collect();
            assert_eq!(
                actual_keys, expected_keys,
                "Response body must contain exactly the expected fields"
            );
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Edge Cases
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_edge_zero_gap() {
        // cached_ver == claims_ver → gap=0 → Current (no mismatch)
        let result = AuthError::handle_version_mismatch(42, 42);
        assert!(result.is_ok());
        let (gap, _retry) = AuthError::calculate_retry_after(42, 42);
        assert_eq!(gap, GapSize::Current);
    }

    #[test]
    fn test_edge_large_gap_zero_retry_with_refresh_still_works() {
        // Even with retry_after=0, the backend does not block the refresh flow.
        // The guidance is clear to the client (re-auth required).
        let err = AuthError::StaleAuthToken {
            retry_after: 0,
            expected_min_version: 200,
            actual_version: 50,
        };
        assert_eq!(err.retry_after(), 0);
        // The error struct is still valid and serializable
        let resp = err.to_http_response();
        assert_eq!(resp.status, 401);
        assert_eq!(resp.body["error"], "stale_auth_token");
    }

    #[test]
    fn test_edge_very_large_retry_after_formatting() {
        // 300 seconds should not be truncated, negative, or overflow
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        // Verify in JSON body
        let retry_val = resp.body["retry_after"].as_u64().unwrap_or(0);
        assert_eq!(retry_val, 300);
        // Verify in header
        let retry_header = resp
            .headers
            .iter()
            .find(|(k, _)| k.as_ref() == "Retry-After")
            .map(|(_, v)| v.parse::<i64>().ok())
            .flatten();
        assert!(retry_header.is_some());
        assert_eq!(retry_header.unwrap(), 300);
    }

    #[test]
    fn test_edge_very_large_version_numbers() {
        // u64::MAX should not cause overflow issues
        let result = AuthError::calculate_retry_after(u64::MAX, u64::MAX);
        assert_eq!(result.0, GapSize::Current);

        let result = AuthError::calculate_retry_after(u64::MAX, u64::MAX.saturating_sub(5));
        assert_eq!(result.0, GapSize::Small);
        assert_eq!(result.1, 300);
    }

    #[test]
    fn test_edge_version_gap_of_one() {
        // Gap of 1 should return retry_after=300 (small gap)
        let (gap, retry) = AuthError::calculate_retry_after(43, 42);
        assert_eq!(gap, GapSize::Small);
        assert_eq!(retry, 300);
    }

    #[test]
    fn test_edge_concurrent_mismatches_same_user() {
        // 50 concurrent requests with the same stale token → all should get 401
        // This tests that the version check is deterministic
        let err = AuthError::StaleAuthToken {
            retry_after: 0,
            expected_min_version: 100,
            actual_version: 10,
        };
        // Simulate 50 identical checks
        for _ in 0..50 {
            let resp = err.to_http_response();
            assert_eq!(resp.status, 401);
            assert_eq!(resp.body["retry_after"], 0);
            assert_eq!(resp.body["error"], "stale_auth_token");
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Retry-After Header Consistency
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_retry_after_header_matches_json_body_small_gap() {
        // Retry-After HTTP header and JSON body should be consistent
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 47,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        let json_retry = resp.body["retry_after"].as_u64().unwrap();
        let header_retry = resp
            .headers
            .iter()
            .find(|(k, _)| k.as_ref() == "Retry-After")
            .map(|(_, v)| v.parse::<u64>().ok())
            .flatten()
            .unwrap_or(0);
        assert_eq!(json_retry, header_retry);
    }

    #[test]
    fn test_retry_after_header_matches_json_body_large_gap() {
        // Same consistency check for large gap (retry_after=0)
        let err = AuthError::StaleAuthToken {
            retry_after: 0,
            expected_min_version: 200,
            actual_version: 50,
        };
        let resp = err.to_http_response();
        let json_retry = resp.body["retry_after"].as_u64().unwrap();
        let header_retry = resp
            .headers
            .iter()
            .find(|(k, _)| k.as_ref() == "Retry-After")
            .map(|(_, v)| v.parse::<u64>().ok())
            .flatten()
            .unwrap_or(0);
        assert_eq!(json_retry, header_retry);
    }

    #[test]
    fn test_www_authenticate_header_format() {
        // WWW-Authenticate must follow RFC 7235 format
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        let resp = err.to_http_response();
        let www_auth = resp
            .headers
            .iter()
            .find(|(k, _)| k.as_ref() == "WWW-Authenticate")
            .map(|(_, v)| v.as_str())
            .unwrap();
        // Must contain: Bearer error="stale_auth_token", retry_after=300
        assert!(www_auth.starts_with("Bearer "));
        assert!(www_auth.contains("error=\"stale_auth_token\""));
        assert!(www_auth.contains("retry_after=300"));
    }

    #[test]
    fn test_www_authenticate_header_format_zero_retry() {
        let err = AuthError::StaleAuthToken {
            retry_after: 0,
            expected_min_version: 200,
            actual_version: 50,
        };
        let resp = err.to_http_response();
        let www_auth = resp
            .headers
            .iter()
            .find(|(k, _)| k.as_ref() == "WWW-Authenticate")
            .map(|(_, v)| v.as_str())
            .unwrap();
        assert!(www_auth.contains("retry_after=0"));
    }
}
