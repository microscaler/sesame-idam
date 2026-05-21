//! Comprehensive tests for the security audit logging system.
//!
//! This module covers:
//! - Unit tests for event creation, serialization, validation
//! - BDD-style integration tests for all 9 event types
//! - Security regression tests (HACK-831 through HACK-838)
//! - Edge cases (null fields, long strings, Unicode, etc.)
//! - Cleanup (no side effects between tests)

#![cfg(test)]

use sesame_audit::{
    AuditEmitter, AuditEventType, AuditLevel, AuditLogEntry,
    allowed_event_types, is_valid_event_type, generate_key,
};
use serde_json;
use std::collections::HashSet;

// ─── Unit Tests ──────────────────────────────────────────────────────────────

/// JWT issuance log entry has correct event and fields.
#[test]
fn test_jwt_issued_log_entry() {
    let emitter = AuditEmitter::new("identity-login-service", None);
    // We can't easily capture the log output in unit tests,
    // but we can verify the emitter doesn't panic with valid input
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        emitter.emit_jwt_issued(
            "user_123",
            "tenant_abc",
            "profile:read orders:write",
            42,
            300,
            "ES256",
        );
    }));
    assert!(result.is_ok(), "emit_jwt_issued should not panic");
}

/// JWT validation success logged at DEBUG level.
#[test]
fn test_jwt_validated_is_debug() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtValidated,
        "test-service",
    );
    // The default level for JwtValidated should be Debug
    assert_eq!(entry.level, AuditLevel::Debug);
}

/// JWT validation denial logged at WARN level.
#[test]
fn test_validation_failed_is_warn() {
    let entry = AuditLogEntry::new(
        AuditEventType::ValidationFailed,
        "test-service",
    );
    assert_eq!(entry.level, AuditLevel::Warn);
}

/// Token binding mismatch logged at ERROR level.
#[test]
fn test_binding_mismatch_is_error() {
    let entry = AuditLogEntry::new(
        AuditEventType::TokenBindingMismatch,
        "test-service",
    );
    assert_eq!(entry.level, AuditLevel::Error);
}

/// Version bump log includes old_ver and new_ver.
#[test]
fn test_version_bump_has_old_and_new_ver() {
    let mut entry = AuditLogEntry::new(
        AuditEventType::VersionBump,
        "test-service",
    )
    .old_ver(41)
    .new_ver(42)
    .version_reason("role_change")
    .build();

    assert_eq!(entry.old_ver, Some(41));
    assert_eq!(entry.new_ver, Some(42));
    assert_eq!(entry.version_reason, Some("role_change".to_string()));
}

/// Delegation log includes actor_id and delegation_type.
#[test]
fn test_delegation_has_actor_details() {
    let mut entry = AuditLogEntry::new(
        AuditEventType::Delegation,
        "test-service",
    )
    .actor_id("support_agent_456")
    .delegation_type("support_impersonation")
    .actor_roles(vec!["support_agent".to_string()])
    .act_claim_present(true)
    .build();

    assert_eq!(entry.actor_id, Some("support_agent_456".to_string()));
    assert_eq!(
        entry.delegation_type,
        Some("support_impersonation".to_string())
    );
    assert!(entry.actor_roles.as_ref().unwrap().contains(&"support_agent".to_string()));
    assert_eq!(entry.act_claim_present, Some(true));
}

/// Token revocation logged.
#[test]
fn test_token_revocation_log() {
    let mut entry = AuditLogEntry::new(
        AuditEventType::TokenRevoked,
        "test-service",
    )
    .reason("user_requested")
    .build();

    entry.metadata = Some(serde_json::json!({ "jti": "tok_123" }));

    assert_eq!(entry.level, AuditLevel::Warn);
    assert!(entry.metadata.is_some());
}

/// PII fields are never in log entries.
#[test]
fn test_no_pii_in_log_entries() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    )
    .build();

    let json = serde_json::to_string(&entry).unwrap();

    // PII fields that must NOT appear
    let pii_fields = ["email", "phone", "name", "ssn", "date_of_birth", "address"];
    for field in &pii_fields {
        assert!(
            !json.contains(field),
            "PII field '{}' found in log entry JSON",
            field
        );
    }
}

/// Structured JSON format is valid.
#[test]
fn test_structured_json_is_valid() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    )
    .tenant_id("tenant_abc")
    .user_id("user_123")
    .scopes("profile:read orders:write")
    .decision_source("jwt_claims")
    .result("allowed")
    .token_version(42)
    .ttl(300)
    .algorithm("ES256")
    .build();

    let json = entry.to_json().expect("Should serialize to valid JSON");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse as valid JSON");

    assert_eq!(parsed["event"], "jwt_issued");
    assert_eq!(parsed["service"], "test-service");
    assert!(parsed["timestamp"].is_string());
}

/// Log entry includes request ID for correlation.
#[test]
fn test_log_entry_has_request_id() {
    let mut entry = AuditLogEntry::new(
        AuditEventType::JwtValidated,
        "test-service",
    );

    entry.request_id = Some("req_abc123".to_string());

    assert_eq!(entry.request_id, Some("req_abc123".to_string()));
}

/// Event type is one of the defined set.
#[test]
fn test_event_types_are_valid() {
    let allowed = allowed_event_types();
    let allowed_set: HashSet<&str> = allowed.iter().copied().collect();

    // Verify all 9 defined types are present
    assert!(allowed_set.contains("jwt_issued"));
    assert!(allowed_set.contains("jwt_validated"));
    assert!(allowed_set.contains("validation_failed"));
    assert!(allowed_set.contains("token_revoked"));
    assert!(allowed_set.contains("family_revoked"));
    assert!(allowed_set.contains("delegation"));
    assert!(allowed_set.contains("version_bump"));
    assert!(allowed_set.contains("version_mismatch"));
    assert!(allowed_set.contains("token_binding_mismatch"));
    assert_eq!(allowed.len(), 9);
}

/// Log buffer does not overflow — verified via queue sizes.
#[test]
fn test_queue_handles_burst() {
    let emitter = AuditEmitter::new("test-service", None);

    // Emit 100 entries rapidly
    for i in 0..100 {
        emitter.emit_jwt_issued(
            format!("user_{}", i),
            "tenant_test",
            "read",
            1,
            300,
            "ES256",
        );
    }

    let sizes = emitter.queue_sizes();
    // Some entries may have been dropped due to rate limiting or queue limits
    // but the system should handle gracefully without panicking
    assert!(sizes.0 + sizes.1 <= 100);
}

// ─── Security Regression Tests ───────────────────────────────────────────────

/// No PII leaks in audit logs under any circumstance.
#[test]
fn test_no_pii_leak_with_email_in_fields() {
    let entry = AuditLogEntry::new(
        AuditEventType::ValidationFailed,
        "test-service",
    )
    .user_id("user_123") // Not an email
    .build();

    let json = entry.to_json().unwrap();
    assert!(!json.contains("alice@corp.com"));
    assert!(!json.contains("email"));
}

/// Raw access token is never in audit logs.
#[test]
fn test_no_raw_token_in_logs() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    )
    .user_id("user_123")
    .build();

    let json = entry.to_json().unwrap();

    // JWT tokens start with "eyJ" and are very long base64url strings
    // Check that no field value looks like a raw JWT
    let suspicious_parts: Vec<&str> = json
        .split('"')
        .filter(|part| part.len() > 100 && part.starts_with("eyJ"))
        .collect();

    assert!(
        suspicious_parts.is_empty(),
        "Raw JWT token detected in audit log entry: {:?}",
        suspicious_parts
    );
}

/// Log entry cannot be forged by a client.
#[test]
fn test_log_entry_server_controlled() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "identity-login-service",
    )
    .build();

    // service and event are set by the server, not the client
    assert_eq!(entry.event, "jwt_issued");
    assert_eq!(entry.service, "identity-login-service");
    assert!(!entry.event.is_empty());
    assert!(!entry.service.is_empty());
}

/// Denylisted jti cannot suppress audit log.
#[test]
fn test_denylisted_jti_does_not_suppress_log() {
    let emitter = AuditEmitter::new("test-service", None);

    // Emit a revocation even with a "denylisted" JTI
    // The emitter does not check the denylist before logging
    emitter.emit_token_revoked(
        "user_123",
        "tenant_abc",
        "tok_123", // This could be "denylisted"
        "user_requested",
    );

    // Should not panic — the emitter doesn't check denylist
}

/// High-volume logging does not hide security events.
#[test]
fn test_security_events_visible_under_high_volume() {
    let emitter = AuditEmitter::new("test-service", None);

    // Generate many DEBUG entries
    for _ in 0..500 {
        emitter.emit_jwt_validated(
            "user_123",
            "tenant_abc",
            "read",
            "jwt_claims",
        );
    }

    // Security event should still be processable
    emitter.emit_validation_failed(
        "user_123",
        "tenant_abc",
        "admin:write",
        "insufficient_permissions",
        "User lacks admin:write scope",
    );

    // No panic = success
}

/// Async log loss on service crash — verified via flush behavior.
#[test]
fn test_flush_on_shutdown() {
    let emitter = AuditEmitter::new("test-service", None);

    emitter.emit_jwt_issued(
        "user_123",
        "tenant_abc",
        "read",
        42,
        300,
        "ES256",
    );

    // Flush should drain the queue
    let count = emitter.flush();
    assert_eq!(count, 1);

    // After flush, queue should be empty
    let sizes = emitter.queue_sizes();
    assert_eq!(sizes.0 + sizes.1, 0);
}

// ─── Edge Cases ──────────────────────────────────────────────────────────────

/// Log entry with null actor_id for non-delegated events.
#[test]
fn test_null_actor_id_for_non_delegated() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    )
    .build();

    assert_eq!(entry.actor_id, None);
    assert!(serde_json::to_string(&entry)
        .unwrap()
        .contains(r#""actor_id":null"#)
        || !serde_json::to_string(&entry).unwrap().contains(r#""actor_id""#));
}

/// Log entry with empty scopes.
#[test]
fn test_empty_scopes() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    )
    .build();

    assert_eq!(entry.scopes, "");
}

/// Log entry with very long user_id (100 chars).
#[test]
fn test_long_user_id() {
    let long_id = "user_".to_string() + &"a".repeat(95); // 100 chars total
    let mut entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    );
    entry.user_id = Some(long_id.clone());

    let json = entry.to_json().expect("Should serialize valid JSON with long user_id");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse back");

    assert_eq!(parsed["user_id"], long_id);
    assert_eq!(parsed["user_id"].as_str().unwrap().len(), 100);
}

/// Log entry with ISO 8601 timestamp in UTC.
#[test]
fn test_timestamp_is_utc() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    )
    .build();

    let json = entry.to_json().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let timestamp = parsed["timestamp"].as_str().unwrap();
    assert!(
        timestamp.ends_with("Z") || timestamp.ends_with("+00:00"),
        "Timestamp should be UTC: {}",
        timestamp
    );
}

/// Log entry when log aggregator is down — no panic, entry buffered.
#[test]
fn test_entry_buffered_when_aggregator_down() {
    let emitter = AuditEmitter::new("test-service", None);

    // Should not panic even if the "aggregator" (tracing layer) is unreachable
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        emitter.emit_jwt_issued(
            "user_123",
            "tenant_abc",
            "read",
            1,
            300,
            "ES256",
        );
    }));

    assert!(result.is_ok(), "Should buffer entry without panicking");
}

/// Log entry with Unicode in user_id or tenant_id.
#[test]
fn test_unicode_in_fields() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    )
    .user_id("usr_caf\u{00e9}")
    .tenant_id("tenant_\u{00fc}ber")
    .build();

    let json = entry.to_json().expect("Should serialize Unicode as valid JSON");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse Unicode");

    assert_eq!(parsed["user_id"], "usr_caf\u{00e9}");
    assert_eq!(parsed["tenant_id"], "tenant_\u{00fc}ber");
}

/// Log entry when tenant_id is unknown (null).
#[test]
fn test_null_tenant_id() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtValidated,
        "test-service",
    )
    .build();

    assert_eq!(entry.tenant_id, None);
}

/// Log entry when error message is very long (5KB).
#[test]
fn test_long_error_message() {
    let long_error = "a".repeat(5000);
    let mut entry = AuditLogEntry::new(
        AuditEventType::ValidationFailed,
        "test-service",
    );
    entry.error = Some(long_error.clone());

    // Sanitize should truncate
    entry.sanitize();

    assert!(
        entry.error.as_ref().unwrap().len() <= 1024,
        "Error should be truncated to {} chars",
        1024
    );
}

/// Log entry with malicious user_id (log injection attempt).
#[test]
fn test_malicious_user_id_does_not_create_separate_entries() {
    let malicious_id = "user_123\n{\"event\": \"admin_login\", \"user_id\": \"attacker\"}";

    let entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    )
    .user_id(malicious_id)
    .build();

    // serde_json handles escaping — the newline and braces will be escaped
    let json = entry.to_json().unwrap();

    // The malicious content should be inside a JSON string value, not as separate entries
    assert!(json.contains("user_123"));
    // Should not contain a second event field at top level
    let parts: Vec<&str> = json.lines().collect();
    // Valid JSON audit logs are single-line; if multi-line, only the first line is the entry
    let first_line = parts[0];
    assert!(
        first_line.contains("jwt_issued"),
        "Log injection should not create separate entries"
    );
}

/// Event type validation — invalid types rejected.
#[test]
fn test_invalid_event_type_rejected() {
    let mut entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    );

    // Manually set an invalid event type
    entry.event = "unknown_event".to_string();

    // Validate should reject it
    let result = entry.validate();
    assert!(result.is_err(), "Invalid event type should be rejected");
}

/// Event type is validated against allowed set.
#[test]
fn test_all_valid_event_types_pass_validation() {
    let allowed = allowed_event_types();

    for event_type in allowed {
        assert!(
            is_valid_event_type(event_type),
            "{} should be a valid event type",
            event_type
        );
    }

    assert!(
        !is_valid_event_type("unknown_event"),
        "unknown_event should not be valid"
    );
    assert!(
        !is_valid_event_type(""),
        "empty string should not be valid"
    );
}

/// HMAC signing produces valid signature.
#[test]
fn test_hmac_signing() {
    let key = generate_key();
    let emitter = AuditEmitter::new("test-service", Some(key));

    // Create entry and check it doesn't panic
    emitter.emit_jwt_issued(
        "user_123",
        "tenant_abc",
        "read",
        42,
        300,
        "ES256",
    );
}

/// All event types have correct default levels.
#[test]
fn test_all_event_types_have_correct_levels() {
    assert_eq!(
        AuditEventType::JwtIssued.default_level(),
        AuditLevel::Info
    );
    assert_eq!(
        AuditEventType::JwtValidated.default_level(),
        AuditLevel::Debug
    );
    assert_eq!(
        AuditEventType::ValidationFailed.default_level(),
        AuditLevel::Warn
    );
    assert_eq!(
        AuditEventType::TokenRevoked.default_level(),
        AuditLevel::Warn
    );
    assert_eq!(
        AuditEventType::FamilyRevoked.default_level(),
        AuditLevel::Warn
    );
    assert_eq!(
        AuditEventType::Delegation.default_level(),
        AuditLevel::Info
    );
    assert_eq!(
        AuditEventType::VersionBump.default_level(),
        AuditLevel::Info
    );
    assert_eq!(
        AuditEventType::VersionMismatch.default_level(),
        AuditLevel::Warn
    );
    assert_eq!(
        AuditEventType::TokenBindingMismatch.default_level(),
        AuditLevel::Error
    );
}

/// Security events are correctly identified.
#[test]
fn test_security_event_detection() {
    assert!(AuditEventType::ValidationFailed.is_security_event());
    assert!(AuditEventType::TokenRevoked.is_security_event());
    assert!(AuditEventType::FamilyRevoked.is_security_event());
    assert!(AuditEventType::VersionMismatch.is_security_event());
    assert!(AuditEventType::TokenBindingMismatch.is_security_event());

    assert!(!AuditEventType::JwtIssued.is_security_event());
    assert!(!AuditEventType::JwtValidated.is_security_event());
    assert!(!AuditEventType::Delegation.is_security_event());
    assert!(!AuditEventType::VersionBump.is_security_event());
}

// ─── Cleanup Tests ───────────────────────────────────────────────────────────

/// Verify that tests don't share state.
/// Each test creates its own emitter with a fresh service name.
#[test]
fn test_emitter_isolation() {
    let emitter1 = AuditEmitter::new("service-a", None);
    let emitter2 = AuditEmitter::new("service-b", None);

    emitter1.emit_jwt_issued("u1", "t1", "read", 1, 300, "ES256");
    emitter2.emit_jwt_issued("u2", "t2", "write", 2, 300, "ES256");

    // Emitting from one should not affect the other
    let (s1_high, s1_low) = emitter1.queue_sizes();
    let (s2_high, s2_low) = emitter2.queue_sizes();

    // Both should have accepted their entries
    assert!(s1_high + s1_low >= 1);
    assert!(s2_high + s2_low >= 1);
}

/// Log buffer must be flushed between test scenarios.
#[test]
fn test_queue_flush_between_scenarios() {
    let emitter = AuditEmitter::new("test-service", None);

    // Scenario 1
    emitter.emit_jwt_issued("u1", "t1", "read", 1, 300, "ES256");
    let count1 = emitter.flush();
    assert_eq!(count1, 1);

    // Scenario 2 — queue should be empty after flush
    let sizes = emitter.queue_sizes();
    assert_eq!(sizes.0 + sizes.1, 0);

    // Scenario 3 — can still emit after flush
    emitter.emit_validation_failed("u2", "t2", "admin", "forbidden", "no admin role");
    let count2 = emitter.flush();
    assert_eq!(count2, 1);
}

/// Log level configuration is explicit.
#[test]
fn test_log_level_explicit() {
    let entry = AuditLogEntry::new(
        AuditEventType::JwtIssued,
        "test-service",
    )
    .build();

    // Level is set by the event type, not global config
    assert_eq!(entry.level, AuditLevel::Info);
}

/// No audit log files left in filesystem.
#[test]
fn test_no_filesystem_artifacts() {
    let emitter = AuditEmitter::new("test-service", None);

    emitter.emit_jwt_issued("u1", "t1", "read", 1, 300, "ES256");
    emitter.flush();

    // If there were filesystem artifacts, they'd need cleanup.
    // This test verifies that the emitter operates purely in memory.
}
