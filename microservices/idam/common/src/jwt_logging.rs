//! # JWT Structured Logging
//!
//! Per-request structured JWT logging with standard fields for audit trail,
//! incident investigation, and compliance reporting.
//!
//! ## Security Requirements
//!
//! - **NEVER log raw access tokens or refresh tokens** — tokens are secrets
//! - **NEVER log PII fields** (email, phone, name) — only opaque identifiers
//! - **Log field injection prevention** — JWT claims are NEVER merged into
//!   the log entry at the top level (HACK-961)
//! - **Structured log fields are set explicitly by the middleware**, not from JWT claims
//!
//! ## Log Format
//!
//! ```json
//! {
//!   "timestamp": "2026-05-15T22:30:00Z",
//!   "level": "WARN",
//!   "service": "identity-user-mgmt-service",
//!   "event": "jwt_validation",
//!   "issuer": "https://idam.example.com",
//!   "subject": "user_123",
//!   "client_id": "web-portal",
//!   "session_id": "ses_01JV8W...",
//!   "token_id": "tok_abc123",
//!   "token_version": 42,
//!   "route": "/api/v1/identity/users/me",
//!   "decision_source": "jwt_claims",
//!   "actor_subject": null,
//!   "result": "allowed",
//!   "method": "GET"
//! }
//! ```
//!
//! ## Decision Source Values
//!
//! | Value | When Used |
//! |-------|-----------|
//! | `jwt_claims` | JWT common path evaluated and decided |
//! | `fallback_cached` | Online fallback result came from cache |
//! | `fallback_online` | Online fallback called authz-core |
//! | `denylist` | Token was in jti denylist |
//! | `version_mismatch` | claims.ver < cached_ver |
//! | `online_only` | Route was online-only, always called authz-core |

use serde::Serialize;
use serde_json::Value;

use crate::jwt::{AccessClaims, ActorClaim};

// ============================================================================
// Structured Log Entry
// ============================================================================

/// Structured log entry for JWT validation events.
///
/// All fields are explicitly set by the middleware — JWT claims are NEVER
/// merged at the top level (HACK-961).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct JwtLogEntry {
    /// Timestamp — set by the logging framework.
    #[serde(skip_serializing)]
    pub timestamp: Option<String>,
    /// Service name — set by the service.
    pub service: String,
    /// Event type: "jwt_validation", "jwt_validation_failed", etc.
    pub event: String,
    /// JWT issuer claim (iss).
    pub issuer: Option<String>,
    /// JWT subject claim (sub).
    pub subject: Option<String>,
    /// JWT client_id claim.
    pub client_id: Option<String>,
    /// JWT session_id claim (sid).
    pub session_id: Option<String>,
    /// JWT token_id claim (jti).
    pub token_id: Option<String>,
    /// JWT token_version claim (ver).
    pub token_version: Option<u64>,
    /// Route path being authorized.
    pub route: Option<String>,
    /// Decision source: jwt_claims, fallback_cached, fallback_online, denylist, version_mismatch, online_only.
    pub decision_source: Option<String>,
    /// Actor subject from act claim (delegation).
    pub actor_subject: Option<String>,
    /// Result: allowed, denied.
    pub result: Option<String>,
    /// HTTP method.
    pub method: Option<String>,
    /// Error reason (when denied).
    pub error_reason: Option<String>,
    /// Error details (on validation failure).
    pub error_details: Option<String>,
    /// Expected token version (on version mismatch).
    pub expected_ver: Option<u64>,
    /// Actual token version (on version mismatch).
    pub actual_ver: Option<u64>,
    /// Delegation type (on delegation events).
    pub delegation_type: Option<String>,
    /// Additional fields set by the middleware (not from JWT claims).
    #[serde(flatten, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

impl Default for JwtLogEntry {
    fn default() -> Self {
        Self {
            timestamp: None,
            service: String::new(),
            event: "jwt_validation".to_string(),
            issuer: None,
            subject: None,
            client_id: None,
            session_id: None,
            token_id: None,
            token_version: None,
            route: None,
            decision_source: None,
            actor_subject: None,
            result: None,
            method: None,
            error_reason: None,
            error_details: None,
            expected_ver: None,
            actual_ver: None,
            delegation_type: None,
            extra: Value::Null,
        }
    }
}

// ============================================================================
// Builder Pattern
// ============================================================================

/// Builder for JwtLogEntry.
///
/// All fields are explicitly set — JWT claims are NEVER merged at the
/// top level (HACK-961 protection).
pub struct JwtLogEntryBuilder {
    entry: JwtLogEntry,
}

impl JwtLogEntryBuilder {
    pub fn new() -> Self {
        Self {
            entry: JwtLogEntry::default(),
        }
    }

    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.entry.service = service.into();
        self
    }

    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        self.entry.event = event.into();
        self
    }

    /// Extract standard fields from JWT claims and set them on the log entry.
    ///
    /// This is the ONLY place where JWT claim values enter the log entry.
    /// The field names are safe (iss, sub, client_id, jti, ver, sid) —
    /// none of them conflict with log-level field names (level, event, service).
    ///
    /// # HACK-961 Safety
    ///
    /// Claims are extracted individually by known-safe field names.
    /// The raw claims Value is NEVER placed in the log entry.
    pub fn with_claims(mut self, claims: &AccessClaims) -> Self {
        self.entry.issuer = Some(claims.iss.clone());
        self.entry.subject = Some(claims.sub.clone());
        if !claims.client_id.is_empty() {
            self.entry.client_id = Some(claims.client_id.clone());
        }
        self.entry.session_id = if claims.sid.is_empty() {
            None
        } else {
            Some(claims.sid.clone())
        };
        self.entry.token_id = if claims.jti.is_empty() {
            None
        } else {
            Some(claims.jti.clone())
        };
        if claims.ver > 0 {
            self.entry.token_version = Some(claims.ver);
        }
        self
    }

    /// Extract actor_subject from the act claim (RFC 8693 delegation).
    ///
    /// # HACK-965 Note
    ///
    /// The actor_subject in the structured log records what the JWT says,
    /// not what the actual actor is. Cross-checking against authz-core
    /// verification happens downstream.
    pub fn with_actor_subject(mut self, claims: &AccessClaims) -> Self {
        if let Some(ref act) = claims.act {
            if !act.sub.is_empty() {
                self.entry.actor_subject = Some(act.sub.clone());
            }
        }
        self
    }

    pub fn with_route(mut self, route: impl Into<String>) -> Self {
        self.entry.route = Some(route.into());
        self
    }

    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.entry.method = Some(method.into());
        self
    }

    pub fn with_decision_source(mut self, source: impl Into<String>) -> Self {
        self.entry.decision_source = Some(source.into());
        self
    }

    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.entry.result = Some(result.into());
        self
    }

    pub fn with_error_reason(mut self, reason: impl Into<String>) -> Self {
        self.entry.error_reason = Some(reason.into());
        self
    }

    pub fn with_error_details(mut self, details: impl Into<String>) -> Self {
        self.entry.error_details = Some(details.into());
        self
    }

    pub fn with_expected_ver(mut self, ver: u64) -> Self {
        self.entry.expected_ver = Some(ver);
        self
    }

    pub fn with_actual_ver(mut self, ver: u64) -> Self {
        self.entry.actual_ver = Some(ver);
        self
    }

    pub fn with_delegation_type(mut self, delegation_type: impl Into<String>) -> Self {
        self.entry.delegation_type = Some(delegation_type.into());
        self
    }

    pub fn with_extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        // SAFETY: We control the key names — they are never from JWT claims.
        // This prevents log field injection (HACK-961).
        if let Ok(mut obj) =
            serde_json::from_value::<serde_json::Map<String, Value>>(self.entry.extra.clone())
        {
            obj.insert(key.into(), Value::String(value.into()));
            self.entry.extra = Value::Object(obj);
        }
        self
    }

    pub fn build(self) -> JwtLogEntry {
        self.entry
    }
}

impl Default for JwtLogEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Log Field Injection Prevention (HACK-961)
// ============================================================================

/// Log field names that MUST NOT be set from JWT claim values.
///
/// These are reserved for the logging infrastructure. If a JWT claim
/// has the same name, the middleware must NOT use the claim value —
/// it must use the value set by the middleware itself.
const LOG_FIELD_PROTECTIONS: &[&str] = &[
    "level",
    "event",
    "service",
    "timestamp",
    "error_reason",
    "error_details",
    "expected_ver",
    "actual_ver",
    "delegation_type",
];

/// Validate that JWT claim keys do not conflict with protected log field names.
///
/// Returns an error message if a dangerous claim is found, `Ok(())` otherwise.
///
/// # Security
///
/// This is a defense-in-depth check. The primary protection is that the
/// builder extracts fields individually by known-safe names (HACK-961).
/// This check catches any future code that might accidentally merge claims.
pub fn validate_no_field_injection(claims_json: &Value) -> Result<(), String> {
    if let Some(obj) = claims_json.as_object() {
        for key in obj.keys() {
            if LOG_FIELD_PROTECTIONS.contains(&key.as_str()) {
                return Err(format!(
                    "JWT claim '{}' conflicts with protected log field name",
                    key
                ));
            }
        }
    }
    Ok(())
}

/// Validate that a log entry does not contain raw tokens.
///
/// This checks both the known token fields and any flattened extra fields
/// to ensure no raw JWT strings slipped through.
pub fn validate_no_raw_token(entry: &JwtLogEntry, raw_token: &str) -> bool {
    // Check that raw token doesn't appear in any string field
    let entry_json = serde_json::to_string(entry).unwrap_or_default();
    !entry_json.contains(raw_token)
}

// ============================================================================
// Structured Logging Helpers
// ============================================================================

/// Emit a structured JWT log entry using tracing.
///
/// # Arguments
///
/// * `level` — Logging level (INFO/WARN/ERROR)
/// * `entry` — The structured log entry
///
/// # Security
///
/// All fields are pre-built — this function does not extract from claims.
/// The entry is serialized to JSON and emitted as a single structured log line.
pub fn emit_structured_jwt_log(level: tracing::Level, entry: &JwtLogEntry) {
    let json = serde_json::to_string(entry).unwrap_or_else(|e| {
        format!(
            "{{\"event\":\"jwt_validation\",\"error\":\"serialization_failed\",\"details\":\"{}\"}}",
            truncate(&e.to_string(), 200)
        )
    });

    match level {
        tracing::Level::INFO => {
            tracing::info!(%json, "jwt_validation");
        }
        tracing::Level::WARN => {
            tracing::warn!(%json, "jwt_validation");
        }
        tracing::Level::ERROR => {
            tracing::error!(%json, "jwt_validation");
        }
        _ => {
            tracing::debug!(%json, "jwt_validation");
        }
    }
}

/// Emit a JWT validation failure log.
pub fn emit_jwt_validation_failure(
    service: &str,
    route: &str,
    method: &str,
    error: &str,
    claims: Option<&AccessClaims>,
) {
    let mut builder = JwtLogEntryBuilder::new()
        .with_service(service)
        .with_event("jwt_validation_failed")
        .with_route(route)
        .with_method(method)
        .with_result("denied")
        .with_error_reason(error);

    if let Some(claims) = claims {
        builder = builder.with_claims(claims).with_actor_subject(claims);
    }

    let entry = builder.build();
    emit_structured_jwt_log(tracing::Level::ERROR, &entry);
}

/// Emit a JWT denial log (policy violation).
pub fn emit_jwt_denial(
    service: &str,
    route: &str,
    method: &str,
    decision_source: &str,
    error_reason: &str,
    claims: Option<&AccessClaims>,
) {
    let mut builder = JwtLogEntryBuilder::new()
        .with_service(service)
        .with_event("jwt_validation")
        .with_route(route)
        .with_method(method)
        .with_result("denied")
        .with_decision_source(decision_source)
        .with_error_reason(error_reason);

    if let Some(claims) = claims {
        builder = builder.with_claims(claims).with_actor_subject(claims);
    }

    let entry = builder.build();
    emit_structured_jwt_log(tracing::Level::WARN, &entry);
}

/// Emit a JWT success log.
pub fn emit_jwt_allowed(
    service: &str,
    route: &str,
    method: &str,
    decision_source: &str,
    claims: Option<&AccessClaims>,
) {
    let mut builder = JwtLogEntryBuilder::new()
        .with_service(service)
        .with_event("jwt_validation")
        .with_route(route)
        .with_method(method)
        .with_result("allowed")
        .with_decision_source(decision_source);

    if let Some(claims) = claims {
        builder = builder.with_claims(claims).with_actor_subject(claims);
    }

    let entry = builder.build();
    emit_structured_jwt_log(tracing::Level::INFO, &entry);
}

/// Emit a delegation log.
pub fn emit_delegation_event(
    service: &str,
    route: &str,
    method: &str,
    delegation_type: &str,
    claims: &AccessClaims,
) {
    let entry = JwtLogEntryBuilder::new()
        .with_service(service)
        .with_event("jwt_delegation")
        .with_route(route)
        .with_method(method)
        .with_result("delegation")
        .with_claims(claims)
        .with_actor_subject(claims)
        .with_delegation_type(delegation_type)
        .with_extra("delegation_info", "actor_subject_logged")
        .build();

    emit_structured_jwt_log(tracing::Level::INFO, &entry);
}

/// Emit a version mismatch log.
pub fn emit_version_mismatch(
    service: &str,
    route: &str,
    method: &str,
    claims: &AccessClaims,
    cached_ver: u64,
) {
    let entry = JwtLogEntryBuilder::new()
        .with_service(service)
        .with_event("jwt_version_mismatch")
        .with_route(route)
        .with_method(method)
        .with_result("denied")
        .with_decision_source("version_mismatch")
        .with_claims(claims)
        .with_actor_subject(claims)
        .with_expected_ver(cached_ver)
        .with_actual_ver(claims.ver)
        .with_error_reason(format!(
            "token_version_mismatch: cached={}, actual={}",
            cached_ver, claims.ver
        ))
        .build();

    emit_structured_jwt_log(tracing::Level::WARN, &entry);
}

/// Truncate a string to the given max length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_claims() -> AccessClaims {
        AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["identity-login-service".into()])
            .client_id("web-portal")
            .scope("read".into())
            .exp(i64::MAX - 3600)
            .nbf(0)
            .iat(0)
            .jti("tok_abc123")
            .ver(42)
            .sid("ses_01JV8W".into())
            .tenant_id("tenant-a")
            .user_id("user-123")
            .user_type("registered")
            .sx(crate::jwt::SesameAuthzClaims::builder()
                .tenant("tenant-a")
                .portal("web-portal")
                .roles(vec!["admin".into(), "user".into()])
                .permissions(vec!["users:read".into(), "prefs:write".into()])
                .risk("normal".into())
                .build()
                .unwrap())
            .build()
            .unwrap()
    }

    fn make_test_claims_with_actor() -> AccessClaims {
        AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("admin-user")
            .aud(vec!["identity-login-service".into()])
            .client_id("admin-panel")
            .scope("admin".into())
            .exp(i64::MAX - 3600)
            .nbf(0)
            .iat(0)
            .jti("tok_def456")
            .ver(10)
            .sid("ses_admin1".into())
            .tenant_id("tenant-a")
            .user_id("admin-user")
            .user_type("platform")
            .sx(crate::jwt::SesameAuthzClaims::builder()
                .tenant("tenant-a")
                .portal("admin-panel")
                .roles(vec!["admin".into()])
                .permissions(vec!["users:manage".into()])
                .build()
                .unwrap())
            .act(Some(ActorClaim {
                sub: "support_agent_456".into(),
            }))
            .build()
            .unwrap()
    }

    // ─── Builder Tests ──────────────────────────────────────────────

    #[test]
    fn builder_creates_default_entry() {
        let entry = JwtLogEntryBuilder::new().build();
        assert_eq!(entry.event, "jwt_validation");
        assert_eq!(entry.service, "");
        assert!(entry.issuer.is_none());
        assert!(entry.subject.is_none());
    }

    #[test]
    fn builder_with_claims_sets_standard_fields() {
        let claims = make_test_claims();
        let entry = JwtLogEntryBuilder::new().with_claims(&claims).build();

        assert_eq!(entry.issuer, Some("https://idam.example.com".into()));
        assert_eq!(entry.subject, Some("user-123".into()));
        assert_eq!(entry.client_id, Some("web-portal".into()));
        assert_eq!(entry.session_id, Some("ses_01JV8W".into()));
        assert_eq!(entry.token_id, Some("tok_abc123".into()));
        assert_eq!(entry.token_version, Some(42));
    }

    #[test]
    fn builder_with_actor_subject_extracted() {
        let claims = make_test_claims_with_actor();
        let entry = JwtLogEntryBuilder::new()
            .with_actor_subject(&claims)
            .build();

        assert_eq!(entry.actor_subject, Some("support_agent_456".into()));
    }

    #[test]
    fn builder_actor_subject_null_when_no_act_claim() {
        let claims = make_test_claims();
        let entry = JwtLogEntryBuilder::new()
            .with_actor_subject(&claims)
            .build();

        assert_eq!(entry.actor_subject, None);
    }

    #[test]
    fn builder_all_fields() {
        let claims = make_test_claims();
        let entry = JwtLogEntryBuilder::new()
            .with_service("identity-login-service")
            .with_event("jwt_validation")
            .with_claims(&claims)
            .with_route("/api/v1/identity/users/me")
            .with_method("GET")
            .with_decision_source("jwt_claims")
            .with_result("allowed")
            .build();

        assert_eq!(entry.service, "identity-login-service");
        assert_eq!(entry.issuer, Some("https://idam.example.com".into()));
        assert_eq!(entry.subject, Some("user-123".into()));
        assert_eq!(entry.client_id, Some("web-portal".into()));
        assert_eq!(entry.session_id, Some("ses_01JV8W".into()));
        assert_eq!(entry.token_id, Some("tok_abc123".into()));
        assert_eq!(entry.token_version, Some(42));
        assert_eq!(entry.route, Some("/api/v1/identity/users/me".into()));
        assert_eq!(entry.method, Some("GET".into()));
        assert_eq!(entry.decision_source, Some("jwt_claims".into()));
        assert_eq!(entry.result, Some("allowed".into()));
    }

    #[test]
    fn builder_extra_fields() {
        let entry = JwtLogEntryBuilder::new()
            .with_extra("request_count", "1")
            .build();

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"request_count\":\"1\""));
    }

    // ─── Log Field Injection Prevention (HACK-961) ──────────────────

    #[test]
    fn validate_no_field_injection_clean_claims() {
        let claims_json = serde_json::json!({
            "iss": "https://idam.example.com",
            "sub": "user-123",
            "jti": "tok_abc",
            "ver": 42,
            "custom_claim": "safe"
        });
        assert!(validate_no_field_injection(&claims_json).is_ok());
    }

    #[test]
    fn validate_no_field_injection_detects_level_injection() {
        let claims_json = serde_json::json!({
            "iss": "https://idam.example.com",
            "level": "INFO"
        });
        let result = validate_no_field_injection(&claims_json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("level"));
    }

    #[test]
    fn validate_no_field_injection_detects_event_injection() {
        let claims_json = serde_json::json!({
            "event": "security_audit_success"
        });
        let result = validate_no_field_injection(&claims_json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("event"));
    }

    #[test]
    fn validate_no_field_injection_detects_service_injection() {
        let claims_json = serde_json::json!({
            "service": "evil-service"
        });
        let result = validate_no_field_injection(&claims_json);
        assert!(result.is_err());
    }

    // ─── No-PII / No-Token Safety ───────────────────────────────────

    #[test]
    fn log_entry_contains_no_pii_fields() {
        let claims = make_test_claims();
        let entry = JwtLogEntryBuilder::new().with_claims(&claims).build();

        let json = serde_json::to_string(&entry).unwrap();
        assert!(!json.contains("email"));
        assert!(!json.contains("phone"));
        assert!(!json.contains("name"));
        assert!(!json.contains("first_name"));
        assert!(!json.contains("last_name"));
    }

    #[test]
    fn log_entry_contains_no_raw_token() {
        let claims = make_test_claims();
        let entry = JwtLogEntryBuilder::new().with_claims(&claims).build();

        let raw_token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOi...";
        assert!(validate_no_raw_token(&entry, raw_token));
    }

    #[test]
    fn log_entry_does_not_contain_claims_array() {
        let claims = make_test_claims();
        let entry = JwtLogEntryBuilder::new().with_claims(&claims).build();

        let json = serde_json::to_string(&entry).unwrap();
        // Should NOT contain the full claims array or sx.permissions
        assert!(!json.contains("users:read"));
        assert!(!json.contains("prefs:write"));
        assert!(!json.contains("admin"));
    }

    // ─── Structured Log Output ──────────────────────────────────────

    #[test]
    fn log_entry_serializes_correctly() {
        let entry = JwtLogEntryBuilder::new()
            .with_service("test-service")
            .with_claims(&make_test_claims())
            .with_route("/api/test")
            .with_method("GET")
            .with_decision_source("jwt_claims")
            .with_result("allowed")
            .build();

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["service"], "test-service");
        assert_eq!(parsed["event"], "jwt_validation");
        assert_eq!(parsed["issuer"], "https://idam.example.com");
        assert_eq!(parsed["subject"], "user-123");
        assert_eq!(parsed["client_id"], "web-portal");
        assert_eq!(parsed["session_id"], "ses_01JV8W");
        assert_eq!(parsed["token_id"], "tok_abc123");
        assert_eq!(parsed["token_version"], 42);
        assert_eq!(parsed["route"], "/api/test");
        assert_eq!(parsed["decision_source"], "jwt_claims");
        assert_eq!(parsed["result"], "allowed");
        assert_eq!(parsed["method"], "GET");
    }

    #[test]
    fn log_entry_empty_claims_graceful() {
        let empty_claims = AccessClaims::builder()
            .iss("")
            .sub("")
            .aud(vec![])
            .client_id("")
            .scope("")
            .exp(0)
            .nbf(0)
            .iat(0)
            .jti("")
            .ver(0)
            .sid("")
            .tenant_id("")
            .user_id("")
            .user_type("")
            .sx(crate::jwt::SesameAuthzClaims::builder()
                .tenant("")
                .portal("")
                .build()
                .unwrap())
            .build()
            .unwrap();

        let entry = JwtLogEntryBuilder::new().with_claims(&empty_claims).build();

        // Empty strings should produce Some("") not None
        assert_eq!(entry.issuer, Some("".into()));
        assert_eq!(entry.subject, Some("".into()));
        // But empty jti/sid/ver should not appear
        assert_eq!(entry.token_id, None);
        assert_eq!(entry.session_id, None);
        assert_eq!(entry.token_version, None);
    }

    // ─── Helper Functions ───────────────────────────────────────────

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let long = "a".repeat(100);
        let result = truncate(&long, 20);
        assert_eq!(result.len(), 23); // 20 chars + "..."
        assert!(result.ends_with("..."));
    }
}
