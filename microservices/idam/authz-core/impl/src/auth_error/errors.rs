//! Auth error methods and HTTP response generation.
//!
//! Implements `AuthError` methods for version mismatch detection, retry-after
//! calculation, and HTTP 401 response generation per RFC 7235.

use super::types::{
    AuthError, GapSize, VersionMismatchMetrics, ERROR_STALE_AUTH_TOKEN, MESSAGE_STALE_AUTH_TOKEN,
    REASON_STALE_AUTHZ_SNAPSHOT, RETRY_AFTER_SMALL_GAP, VERSION_GAP_LARGE,
};
use brrtrouter::dispatcher::{HandlerResponse, HeaderVec};
use http::StatusCode;
use std::sync::Arc;

impl AuthError {
    /// Calculate the retry-after value based on version gap size.
    ///
    /// Returns the gap size classification along with the `retry_after` value.
    ///
    /// # Gap calculation
    ///
    /// - If `claims_ver >= cached_ver`: token is current, no mismatch
    /// - If gap is 1-10: `retry_after = 300` (allow refresh)
    /// - If gap > 100: `retry_after = 0` (immediate re-auth)
    ///
    /// Note: gaps of 11-100 are treated as "small" (`retry_after` = 300) since
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
    /// use crate::auth_error::AuthError;
    ///
    /// // Token is current
    /// assert!(AuthError::handle_version_mismatch(42, 42).is_ok());
    /// assert!(AuthError::handle_version_mismatch(45, 42).is_ok());
    ///
    /// // Token is stale
    /// match AuthError::handle_version_mismatch(45, 42) {
    ///     Err(AuthError::StaleAuthToken { retry_after, .. }) => {
    ///         assert_eq!(retry_after, 300); // small gap
    ///     }
    ///     _ => panic!("expected StaleAuthToken"),
    /// }
    /// ```
    ///
    /// Records metrics for the version mismatch event (mismatch total,
    /// gap size label) and the cache lookup latency.
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

    /// Extract `retry_after` for metrics purposes.
    pub fn retry_after(&self) -> u64 {
        match self {
            AuthError::StaleAuthToken { retry_after, .. } => *retry_after,
        }
    }

    /// Extract the expected (cached) minimum version.
    pub fn expected_min_version(&self) -> u64 {
        match self {
            AuthError::StaleAuthToken {
                expected_min_version,
                ..
            } => *expected_min_version,
        }
    }

    /// Extract the actual (claimed) token version.
    pub fn actual_version(&self) -> u64 {
        match self {
            AuthError::StaleAuthToken { actual_version, .. } => *actual_version,
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
        let (_gap_size, retry_after) = match self {
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
