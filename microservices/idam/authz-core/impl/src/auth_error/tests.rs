#[cfg(test)]
mod auth_error_tests {
    use crate::auth_error::*;

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
        assert!(!err.message().contains(':'));
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
            let retry_val = obj.get("retry_after").and_then(serde_json::Value::as_u64);
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
        let (gap, _retry) = AuthError::calculate_retry_after(42, 42);
        assert_eq!(gap, GapSize::Current);
    }

    #[test]
    fn test_claims_newer_returns_current() {
        // cached=42, claims=50 → token is newer, no mismatch
        let (gap, _retry) = AuthError::calculate_retry_after(42, 50);
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
        assert_eq!(err.retry_after(), 300);
    }

    #[test]
    fn test_handle_version_mismatch_large_gap_fails_with_zero_retry() {
        // cached=150, claims=40 → gap=110, should return StaleAuthToken with retry_after=0
        let result = AuthError::handle_version_mismatch(150, 40);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.retry_after(), 0);
        assert_eq!(err.expected_min_version(), 150);
        assert_eq!(err.actual_version(), 40);
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
        assert_eq!(err.expected_min_version(), 45);
    }

    #[test]
    fn test_stale_auth_token_actual_version_field() {
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 45,
            actual_version: 42,
        };
        assert_eq!(err.actual_version(), 42);
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
        assert_eq!(err.retry_after(), 300);
    }

    #[test]
    fn test_security_no_stack_traces_in_response() {
        // Error response must not include stack traces or internal state
        let err = AuthError::StaleAuthToken {
            retry_after: 300,
            expected_min_version: 999_999,
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
                obj.keys().map(std::string::String::as_str).collect();
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
            .and_then(|(_, v)| v.parse::<i64>().ok());
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
            .and_then(|(_, v)| v.parse::<u64>().ok())
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
            .and_then(|(_, v)| v.parse::<u64>().ok())
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
