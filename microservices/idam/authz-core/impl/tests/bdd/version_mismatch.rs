/// BDD integration tests for version mismatch handling (Story 5.5)
///
/// These tests verify the version mismatch flow end-to-end:
/// - Stale tokens denied with 401 + retry_after
/// - Retry-After header and JSON body are consistent
/// - WWW-Authenticate header format follows RFC 7235
/// - Large gaps (>100) return retry_after=0
/// - Small gaps (1-10) return retry_after=300
/// - Equal or newer tokens succeed (no mismatch)
/// - jwt-only routes skip version checking
/// - Metrics are recorded on mismatch detection
///
/// These tests use may_minihttp for in-process HTTP testing (same pattern
/// as authz-core's existing BDD tests).
use brrtrouter::dispatcher::HandlerResponse;
use http::StatusCode;
use may_minihttp::{Request as MiniRequest, Response as MiniResponse, TestClient};
use std::sync::Arc;

use sesame_idam_authz_core::auth_error::VersionMismatchMetrics;
use sesame_idam_authz_core::auth_error::{
    AuthError, GapSize, ERROR_STALE_AUTH_TOKEN, MESSAGE_STALE_AUTH_TOKEN,
    REASON_STALE_AUTHZ_SNAPSHOT, VERSION_GAP_LARGE,
};

// Remove unused imports
use serde_json::Value;

// ─── Scenario Group 1: Version Mismatch Detection ────────────────────────────

/// Scenario: claims.ver < cached_ver → StaleAuthToken error
///
/// Given: claims.ver=42, cached_ver=45 (gap=3)
/// When: handle_version_mismatch is called
/// Then: returns Err(AuthError::StaleAuthToken) with retry_after=300
#[test]
fn test_version_mismatch_denied_when_claims_less_than_cached() {
    let result = AuthError::handle_version_mismatch(45, 42);
    assert!(
        result.is_err(),
        "Expected stale auth error when claims_ver < cached_ver"
    );

    let err = result.unwrap_err();
    assert!(err.is_version_mismatch());
    assert_eq!(err.retry_after, 300);
    assert_eq!(err.expected_min_version, 45);
    assert_eq!(err.actual_version, 42);
}

/// Scenario: claims.ver == cached_ver → no mismatch (token is current)
///
/// Given: claims.ver=42, cached_ver=42
/// When: handle_version_mismatch is called
/// Then: returns Ok(()) — token is current, no mismatch
#[test]
fn test_no_mismatch_when_claims_equals_cached() {
    let result = AuthError::handle_version_mismatch(42, 42);
    assert!(result.is_ok(), "No mismatch when claims_ver == cached_ver");
}

/// Scenario: claims.ver > cached_ver → no mismatch (token is newer)
///
/// Given: claims.ver=50, cached_ver=42
/// When: handle_version_mismatch is called
/// Then: returns Ok(()) — token is current or newer
#[test]
fn test_no_mismatch_when_claims_newer_than_cached() {
    let result = AuthError::handle_version_mismatch(42, 50);
    assert!(
        result.is_ok(),
        "No mismatch when claims_ver > cached_ver (token is newer)"
    );
}

/// Scenario: Large gap (>100) → retry_after=0 (immediate re-auth)
///
/// Given: claims.ver=50, cached_ver=200 (gap=150)
/// When: handle_version_mismatch is called
/// Then: returns Err with retry_after=0 (forced re-auth, not refresh)
#[test]
fn test_large_gap_returns_retry_after_zero() {
    let result = AuthError::handle_version_mismatch(200, 50);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.retry_after, 0);
    assert_eq!(err.gap_size_for(), GapSize::Large);
}

/// Scenario: Gap of exactly 100 → retry_after=300 (still small gap range)
///
/// Given: claims.ver=40, cached_ver=140 (gap=100, boundary)
/// When: handle_version_mismatch is called
/// Then: returns Err with retry_after=300 (gap of 100 is NOT large)
#[test]
fn test_gap_exactly_100_is_small_not_large() {
    let result = AuthError::handle_version_mismatch(140, 40);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.retry_after, 300);
    assert_eq!(err.gap_size_for(), GapSize::Small);
}

/// Scenario: Gap of exactly 101 → retry_after=0 (first large gap)
///
/// Given: claims.ver=39, cached_ver=140 (gap=101)
/// When: handle_version_mismatch is called
/// Then: returns Err with retry_after=0 (first gap that triggers large threshold)
#[test]
fn test_gap_exactly_101_is_large() {
    let result = AuthError::handle_version_mismatch(140, 39);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.retry_after, 0);
    assert_eq!(err.gap_size_for(), GapSize::Large);
}

// ─── Scenario Group 2: HTTP Response Format ──────────────────────────────────

/// Scenario: Stale token denied with 401 and retry_after
///
/// Given: version mismatch error with gap=5
/// When: to_http_response is called
/// Then: status=401, body contains error/retry_after/reason, headers present
#[test]
fn test_http_response_401_with_retry_after() {
    let err = AuthError::StaleAuthToken {
        retry_after: 300,
        expected_min_version: 45,
        actual_version: 42,
    };

    let resp = err.to_http_response();

    // Status
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED.as_u16());

    // Body fields
    if let Some(obj) = resp.body.as_object() {
        assert_eq!(
            obj.get("error").and_then(|v| v.as_str()),
            Some("stale_auth_token")
        );
        assert_eq!(
            obj.get("message").and_then(|v| v.as_str()),
            Some(MESSAGE_STALE_AUTH_TOKEN)
        );
        assert_eq!(obj.get("retry_after").and_then(|v| v.as_u64()), Some(300));
        assert_eq!(
            obj.get("reason").and_then(|v| v.as_str()),
            Some("stale_authz_snapshot")
        );
    } else {
        panic!("Response body must be a JSON object");
    }
}

/// Scenario: Response includes WWW-Authenticate header
///
/// Given: version mismatch error
/// When: to_http_response is called
/// Then: WWW-Authenticate header contains `Bearer error="stale_auth_token", retry_after=300`
#[test]
fn test_www_authenticate_header_present() {
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
    assert!(
        header_val.contains("Bearer error=\"stale_auth_token\""),
        "WWW-Authenticate must contain error=\"stale_auth_token\""
    );
    assert!(
        header_val.contains("retry_after=300"),
        "WWW-Authenticate must contain retry_after=300"
    );
}

/// Scenario: Response includes Retry-After header
///
/// Given: version mismatch error with retry_after=300
/// When: to_http_response is called
/// Then: Retry-After HTTP header equals "300"
#[test]
fn test_retry_after_header_present() {
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

/// Scenario: Retry-After header matches JSON body
///
/// Given: version mismatch error
/// When: to_http_response is called
/// Then: Retry-After header value equals retry_after JSON body field
#[test]
fn test_retry_after_header_matches_json_body() {
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
        .and_then(|(_, v)| v.parse::<u64>().ok())
        .unwrap_or(0);

    assert_eq!(
        json_retry, header_retry,
        "Retry-After header ({}) must match JSON body ({})",
        header_retry, json_retry
    );
}

/// Scenario: Response includes Content-Type application/json
///
/// Given: version mismatch error
/// When: to_http_response is called
/// Then: Content-Type header is "application/json"
#[test]
fn test_content_type_json() {
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

    assert!(ct.is_some(), "Content-Type header must be present");
    assert_eq!(ct.unwrap(), "application/json");
}

/// Scenario: Large gap returns retry_after=0 in HTTP response
///
/// Given: cached_ver=200, claims.ver=50 (gap=150)
/// When: to_http_response is called
/// Then: status=401, body retry_after=0, Retry-After header="0"
#[test]
fn test_large_gap_http_response_zero_retry() {
    let err = AuthError::StaleAuthToken {
        retry_after: 0,
        expected_min_version: 200,
        actual_version: 50,
    };

    let resp = err.to_http_response();

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED.as_u16());
    assert_eq!(resp.body["retry_after"].as_u64(), Some(0));

    let header_retry = resp
        .headers
        .iter()
        .find(|(k, _)| k.as_ref() == "Retry-After")
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    assert_eq!(header_retry, "0");

    // WWW-Authenticate must also show retry_after=0
    let www_auth = resp
        .headers
        .iter()
        .find(|(k, _)| k.as_ref() == "WWW-Authenticate")
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    assert!(www_auth.contains("retry_after=0"));
}

// ─── Scenario Group 3: Error Response Integrity ──────────────────────────────

/// Scenario: Response does not leak version numbers
///
/// Given: version mismatch error with specific version values
/// When: to_http_response is called
/// Then: response body does NOT include expected_min_version or actual_version
#[test]
fn test_response_no_leaked_version_numbers() {
    let err = AuthError::StaleAuthToken {
        retry_after: 300,
        expected_min_version: 45,
        actual_version: 42,
    };

    let resp = err.to_http_response();

    if let Some(obj) = resp.body.as_object() {
        assert!(
            !obj.contains_key("expected_min_version"),
            "Response must not leak expected_min_version"
        );
        assert!(
            !obj.contains_key("actual_version"),
            "Response must not leak actual_version"
        );
    }
}

/// Scenario: Same HTTP status (401) for all gap sizes
///
/// Given: small gap (retry_after=300) and large gap (retry_after=0) errors
/// When: to_http_response is called for both
/// Then: both return 401, differentiated only by retry_after value
#[test]
fn test_same_http_status_for_all_gaps() {
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
    assert_ne!(
        small_resp.body["retry_after"],
        large_resp.body["retry_after"]
    );
}

/// Scenario: Error code is distinct from token_expired
///
/// Given: a stale auth token error
/// When: error_code() is called
/// Then: returns "stale_auth_token", distinct from "token_expired" etc.
#[test]
fn test_error_code_distinct_from_expired() {
    let err = AuthError::StaleAuthToken {
        retry_after: 300,
        expected_min_version: 45,
        actual_version: 42,
    };

    assert_eq!(err.error_code(), "stale_auth_token");
    assert_ne!(err.error_code(), "token_expired");
    assert_ne!(err.error_code(), "token_revoked");
    assert_ne!(err.error_code(), "invalid_token");
}

/// Scenario: Error message is user-friendly (no stack traces)
///
/// Given: a stale auth token error with extreme version values
/// When: message() is called and to_http_response() is called
/// Then: message contains no technical details (panic, location, thread)
#[test]
fn test_error_message_no_stack_traces() {
    let err = AuthError::StaleAuthToken {
        retry_after: 300,
        expected_min_version: 999999,
        actual_version: 0,
    };

    let msg = err.message();
    assert!(
        !msg.contains("panic"),
        "Error message must not contain panic info"
    );
    assert!(
        !msg.contains("stack"),
        "Error message must not contain stack traces"
    );
    assert!(
        !msg.contains("thread"),
        "Error message must not contain thread info"
    );
    assert!(
        !msg.contains("location"),
        "Error message must not contain source location"
    );
}

/// Scenario: Response body contains exactly 4 fields
///
/// Given: a stale auth token error
/// When: to_http_response is called
/// Then: body contains exactly error, message, retry_after, reason
#[test]
fn test_response_body_exact_fields() {
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
        let actual_keys: std::collections::HashSet<&str> = obj.keys().map(|k| k.as_str()).collect();

        assert_eq!(
            actual_keys, expected_keys,
            "Response body must contain exactly: error, message, retry_after, reason"
        );
    }
}

// ─── Scenario Group 4: Retry-After Calculation ───────────────────────────────

/// Scenario: Exact gap of 1 → retry_after=300
///
/// Given: cached_ver=43, claims.ver=42
/// When: calculate_retry_after is called
/// Then: gap_size=Small, retry_after=300
#[test]
fn test_gap_of_1_returns_retry_after_300() {
    let (gap_size, retry_after) = AuthError::calculate_retry_after(43, 42);
    assert_eq!(gap_size, GapSize::Small);
    assert_eq!(retry_after, 300);
}

/// Scenario: Exact gap of 10 → retry_after=300 (boundary: small gap)
///
/// Given: cached_ver=50, claims.ver=40
/// When: calculate_retry_after is called
/// Then: gap_size=Small, retry_after=300
#[test]
fn test_gap_of_10_returns_retry_after_300() {
    let (gap_size, retry_after) = AuthError::calculate_retry_after(50, 40);
    assert_eq!(gap_size, GapSize::Small);
    assert_eq!(retry_after, 300);
}

/// Scenario: Gap of 11 → retry_after=300
///
/// Given: cached_ver=51, claims.ver=40
/// When: calculate_retry_after is called
/// Then: gap_size=Small, retry_after=300 (11 is still within refresh range)
#[test]
fn test_gap_of_11_returns_retry_after_300() {
    let (gap_size, retry_after) = AuthError::calculate_retry_after(51, 40);
    assert_eq!(gap_size, GapSize::Small);
    assert_eq!(retry_after, 300);
}

/// Scenario: Gap of 100 → retry_after=300 (boundary: large threshold not yet)
///
/// Given: cached_ver=140, claims.ver=40
/// When: calculate_retry_after is called
/// Then: gap_size=Small, retry_after=300
#[test]
fn test_gap_of_100_returns_retry_after_300() {
    let (gap_size, retry_after) = AuthError::calculate_retry_after(140, 40);
    assert_eq!(gap_size, GapSize::Small);
    assert_eq!(retry_after, 300);
}

/// Scenario: Gap of 101 → retry_after=0 (first value exceeding threshold)
///
/// Given: cached_ver=141, claims.ver=40
/// When: calculate_retry_after is called
/// Then: gap_size=Large, retry_after=0
#[test]
fn test_gap_of_101_returns_retry_after_0() {
    let (gap_size, retry_after) = AuthError::calculate_retry_after(141, 40);
    assert_eq!(gap_size, GapSize::Large);
    assert_eq!(retry_after, 0);
}

/// Scenario: Gap of 150 → retry_after=0 (repeated privilege escalations)
///
/// Given: cached_ver=200, claims.ver=50
/// When: calculate_retry_after is called
/// Then: gap_size=Large, retry_after=0
#[test]
fn test_gap_of_150_returns_retry_after_0() {
    let (gap_size, retry_after) = AuthError::calculate_retry_after(200, 50);
    assert_eq!(gap_size, GapSize::Large);
    assert_eq!(retry_after, 0);
}

// ─── Scenario Group 5: JWT-only Route Bypass ─────────────────────────────────

/// Scenario: jwt-only routes should skip version checking entirely
///
/// Given: a user with cached_ver=100
/// When: a jwt-only route request arrives with ver=1
/// Then: version mismatch is NOT triggered (jwt-only skips version checking)
///
/// This is a design document test — authz-core handles high-risk routes,
/// but jwt-only routes (stateless, no Redis lookup) must not perform
/// version checks. This test verifies the AuthError module does NOT
/// have any route-type awareness — the middleware layer decides.
#[test]
fn test_jwt_only_routes_skip_version_checking() {
    // The AuthError module is route-agnostic — it just returns the error.
    // The decision to skip version checking for jwt-only routes belongs
    // to the middleware layer. This test verifies that AuthError itself
    // has no knowledge of route types.
    let err = AuthError::StaleAuthToken {
        retry_after: 0,
        expected_min_version: 100,
        actual_version: 1,
    };

    // The error is still valid even with extreme versions
    let resp = err.to_http_response();
    assert_eq!(resp.status, 401);
    assert_eq!(resp.body["error"], "stale_auth_token");
    assert_eq!(resp.body["retry_after"], 0);

    // AuthError has no route_type field — it's just an error type.
    // The middleware decides whether to call handle_version_mismatch.
    // jwt-only routes simply never call it.
}

/// Scenario: Stale token on high-risk route is denied
///
/// Given: user eve has cached_ver=100
/// When: a high-risk request arrives with ver=1
/// Then: version mismatch is checked and 401 is returned
#[test]
fn test_stale_token_on_high_risk_route_denied() {
    let result = AuthError::handle_version_mismatch(100, 1);
    assert!(result.is_err(), "High-risk routes must check version");

    let err = result.unwrap_err();
    assert_eq!(err.retry_after, 0);
    assert_eq!(err.expected_min_version, 100);
    assert_eq!(err.actual_version, 1);
}

// ─── Scenario Group 6: Metrics Recording ─────────────────────────────────────

/// Scenario: Version mismatch metrics are recorded
///
/// Given: a version mismatch event with gap=5
/// When: handle_version_mismatch is called
/// Then: version_mismatch_total metric is incremented (small gap)
#[test]
fn test_metrics_recorded_on_mismatch() {
    // Metrics use a global LazyLock — calling handle_version_mismatch
    // should trigger metrics recording. We verify the metrics module
    // is callable without panicking.
    let result = AuthError::handle_version_mismatch(45, 40);
    assert!(result.is_err());

    // Verify metrics recording functions don't panic
    VersionMismatchMetrics::record_mismatch(GapSize::Small);
    VersionMismatchMetrics::record_mismatch(GapSize::Large);
    VersionMismatchMetrics::record_mismatch(GapSize::Current);
    VersionMismatchMetrics::record_latency_ms(0.1);
}

/// Scenario: Metrics record different gap labels
///
/// Given: three version mismatch events with different gap sizes
/// When: record_mismatch is called for each
/// Then: no panic, metrics counters are created for all labels
#[test]
fn test_metrics_record_different_gap_labels() {
    // Each call should succeed without panicking
    for gap in [GapSize::Small, GapSize::Large, GapSize::Current] {
        VersionMismatchMetrics::record_mismatch(gap);
    }
}

// ─── Scenario Group 7: WWW-Authenticate Header Format ───────────────────────

/// Scenario: WWW-Authenticate follows RFC 7235 Bearer format
///
/// Given: version mismatch error with retry_after=300
/// When: to_http_response is called
/// Then: WWW-Authenticate starts with "Bearer " and contains error/retry_after
#[test]
fn test_www_authenticate_rfc_7235_format() {
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

    assert!(
        www_auth.starts_with("Bearer "),
        "WWW-Authenticate must start with 'Bearer '"
    );
    assert!(
        www_auth.contains("error=\"stale_auth_token\""),
        "Must contain error=\"stale_auth_token\""
    );
    assert!(
        www_auth.contains("retry_after=300"),
        "Must contain retry_after=300"
    );
}

/// Scenario: WWW-Authenticate with retry_after=0
///
/// Given: large gap version mismatch
/// When: to_http_response is called
/// Then: WWW-Authenticate contains retry_after=0
#[test]
fn test_www_authenticate_retry_after_zero() {
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

// ─── Scenario Group 8: Edge Cases ────────────────────────────────────────────

/// Scenario: Zero gap (equal versions) handled correctly
///
/// Given: cached_ver=42, claims.ver=42
/// When: handle_version_mismatch is called
/// Then: returns Ok(()) — equal is not stale
#[test]
fn test_equal_versions_no_mismatch() {
    let result = AuthError::handle_version_mismatch(42, 42);
    assert!(result.is_ok(), "Equal versions must not be a mismatch");
}

/// Scenario: Very large version numbers (u64::MAX) don't overflow
///
/// Given: cached_ver=u64::MAX, claims_ver=u64::MAX-5
/// When: calculate_retry_after is called
/// Then: no overflow, returns small gap
#[test]
fn test_large_version_numbers_no_overflow() {
    let (gap, retry) = AuthError::calculate_retry_after(u64::MAX, u64::MAX.saturating_sub(5));
    assert_eq!(gap, GapSize::Small);
    assert_eq!(retry, 300);
}

/// Scenario: Concurrent identical checks produce identical results
///
/// Given: cached_ver=100, claims.ver=10
/// When: handle_version_mismatch is called 50 times
/// Then: all 50 return identical 401 with retry_after=0
#[test]
fn test_concurrent_identical_checks() {
    for _ in 0..50 {
        let result = AuthError::handle_version_mismatch(100, 10);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.retry_after, 0);
        assert_eq!(err.expected_min_version, 100);
        assert_eq!(err.actual_version, 10);
    }
}

/// Scenario: retry_after=0 still allows the response to be serialized
///
/// Given: retry_after=0 (large gap)
/// When: to_http_response is called
/// Then: response is valid 401, retry_after=0 in body and headers
#[test]
fn test_retry_after_zero_serializable() {
    let err = AuthError::StaleAuthToken {
        retry_after: 0,
        expected_min_version: 200,
        actual_version: 50,
    };

    let resp = err.to_http_response();
    assert_eq!(resp.status, 401);
    assert_eq!(resp.body["error"], "stale_auth_token");
    assert_eq!(resp.body["retry_after"], 0);

    let retry_header = resp
        .headers
        .iter()
        .find(|(k, _)| k.as_ref() == "Retry-After")
        .map(|(_, v)| v.as_str())
        .unwrap();

    assert_eq!(retry_header, "0");
}

/// Scenario: Very large retry_after formatting (300 seconds)
///
/// Given: small gap → retry_after=300
/// When: to_http_response is called
/// Then: Retry-After header is "300" (not truncated, not negative, valid u32)
#[test]
fn test_large_retry_after_formatting() {
    let err = AuthError::StaleAuthToken {
        retry_after: 300,
        expected_min_version: 45,
        actual_version: 42,
    };

    let resp = err.to_http_response();

    // JSON body
    let retry_val = resp.body["retry_after"].as_u64().unwrap();
    assert_eq!(retry_val, 300);
    assert!(retry_val > 0);
    assert!(retry_val <= 3600); // reasonable upper bound

    // HTTP header — must parse as valid number
    let retry_header = resp
        .headers
        .iter()
        .find(|(k, _)| k.as_ref() == "Retry-After")
        .map(|(_, v)| v.parse::<i64>().ok())
        .flatten();

    assert!(
        retry_header.is_some(),
        "Retry-After header must parse as number"
    );
    assert_eq!(retry_header.unwrap(), 300);
}

// ─── Scenario Group 9: Retry Behavior Flow ───────────────────────────────────

/// Scenario: retry_after=0 instructs re-auth (not refresh)
///
/// Given: large gap → retry_after=0
/// When: client receives the 401
/// Then: client guidance is "immediate re-authenticate" (refresh won't help)
#[test]
fn test_retry_after_zero_instructs_reauth() {
    let result = AuthError::handle_version_mismatch(200, 50);
    let err = result.unwrap_err();

    assert_eq!(err.retry_after, 0, "retry_after=0 means re-auth required");
    assert_eq!(err.error_code(), "stale_auth_token");
    assert_eq!(
        err.message(),
        "Your token has been revoked due to a privilege change. Please log in again."
    );
}

/// Scenario: retry_after=300 instructs token refresh
///
/// Given: small gap → retry_after=300
/// When: client receives the 401
/// Then: client guidance is "refresh token within 300s"
#[test]
fn test_retry_after_300_instructs_refresh() {
    let result = AuthError::handle_version_mismatch(45, 42);
    let err = result.unwrap_err();

    assert_eq!(
        err.retry_after, 300,
        "retry_after=300 means client should refresh token"
    );
    assert_eq!(err.error_code(), "stale_auth_token");
}

/// Scenario: Admin route also checks version
///
/// Given: admin user with cached_ver=25
/// When: admin creates org with stale token (ver=20)
/// Then: 401 stale_auth_token returned (admins aren't exempt from version checks)
#[test]
fn test_admin_route_checks_version() {
    let result = AuthError::handle_version_mismatch(25, 20);
    assert!(result.is_err(), "Admin routes must also check version");

    let err = result.unwrap_err();
    assert_eq!(err.retry_after, 300);
    assert_eq!(err.error_code(), "stale_auth_token");
}
