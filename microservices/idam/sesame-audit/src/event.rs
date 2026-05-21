//! Audit event types, severity, and the core `AuditLogEntry` struct.
//!
//! This module defines:
//! - [`AuditEventType`] — the 8+ defined event categories from the story
//! - [`AuditLevel`] — logging levels mapped to logging semantics
//! - [`AuditLogEntry`] — the structured JSON log entry with all required fields

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::AuditActor;

// ─── Event Types ─────────────────────────────────────────────────────────────

/// All defined audit event types. No values outside this set are allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    #[default]
    JwtIssued,
    JwtValidated,
    ValidationFailed,
    TokenRevoked,
    FamilyRevoked,
    Delegation,
    VersionBump,
    VersionMismatch,
    TokenBindingMismatch,
    SessionManagement,
    UserManagement,
    System,
}

impl AuditEventType {
    /// Returns the string representation used in JSON logs.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventType::JwtIssued => "jwt_issued",
            AuditEventType::JwtValidated => "jwt_validated",
            AuditEventType::ValidationFailed => "validation_failed",
            AuditEventType::TokenRevoked => "token_revoked",
            AuditEventType::FamilyRevoked => "family_revoked",
            AuditEventType::Delegation => "delegation",
            AuditEventType::VersionBump => "version_bump",
            AuditEventType::VersionMismatch => "version_mismatch",
            AuditEventType::TokenBindingMismatch => "token_binding_mismatch",
            AuditEventType::SessionManagement => "session_management",
            AuditEventType::UserManagement => "user_management",
            AuditEventType::System => "system",
        }
    }

    /// Returns the default logging level for this event type.
    #[must_use]
    pub fn default_level(&self) -> AuditLevel {
        match self {
            AuditEventType::JwtIssued => AuditLevel::Info,
            AuditEventType::JwtValidated => AuditLevel::Debug,
            AuditEventType::ValidationFailed => AuditLevel::Warn,
            AuditEventType::TokenRevoked => AuditLevel::Warn,
            AuditEventType::FamilyRevoked => AuditLevel::Warn,
            AuditEventType::Delegation => AuditLevel::Info,
            AuditEventType::VersionBump => AuditLevel::Info,
            AuditEventType::VersionMismatch => AuditLevel::Warn,
            AuditEventType::TokenBindingMismatch => AuditLevel::Error,
            AuditEventType::SessionManagement => AuditLevel::Info,
            AuditEventType::UserManagement => AuditLevel::Info,
            AuditEventType::System => AuditLevel::Info,
        }
    }

    /// Check if this is a security event (WARN or ERROR level).
    #[must_use]
    pub fn is_security_event(&self) -> bool {
        matches!(
            self,
            AuditEventType::ValidationFailed
                | AuditEventType::TokenRevoked
                | AuditEventType::FamilyRevoked
                | AuditEventType::VersionMismatch
                | AuditEventType::TokenBindingMismatch
        )
    }
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Returns the set of allowed event type strings.
#[must_use]
pub fn allowed_event_types() -> &'static [&'static str] {
    &[
        "jwt_issued",
        "jwt_validated",
        "validation_failed",
        "token_revoked",
        "family_revoked",
        "delegation",
        "version_bump",
        "version_mismatch",
        "token_binding_mismatch",
        "session_management",
        "user_management",
        "system",
    ]
}

/// Check if a string is a valid event type.
#[must_use]
pub fn is_valid_event_type(event: &str) -> bool {
    allowed_event_types().contains(&event)
}

// ─── Logging Levels ──────────────────────────────────────────────────────────

/// Logging levels for audit events, mapped to the story's logging requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditLevel {
    /// High-volume normal operations. Rate-limited.
    Debug,
    /// Normal operational events.
    Info,
    /// Security-relevant events (potential issue).
    Warn,
    /// Active attack indicators. Always synchronous.
    Error,
    /// Critical security events requiring immediate attention.
    Critical,
}

impl Default for AuditLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl AuditLevel {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditLevel::Debug => "debug",
            AuditLevel::Info => "info",
            AuditLevel::Warn => "warn",
            AuditLevel::Error => "error",
            AuditLevel::Critical => "critical",
        }
    }

    /// Returns true for security events (WARN, ERROR) that should be logged synchronously.
    #[must_use]
    pub fn is_security(&self) -> bool {
        matches!(self, AuditLevel::Warn | AuditLevel::Error)
    }

    /// Returns true for normal operational events (DEBUG, INFO) that can be async.
    #[must_use]
    pub fn is_normal(&self) -> bool {
        matches!(self, AuditLevel::Debug | AuditLevel::Info)
    }
}

// ─── Audit Log Entry ─────────────────────────────────────────────────────────

/// Maximum length for error/reason fields to prevent log bloat.
const MAX_FIELD_LENGTH: usize = 1024;

/// The structured JSON audit log entry.
///
/// Every security event MUST include all core fields. Optional fields are
/// populated when available but never leak PII.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditLogEntry {
    /// Event type — one of the 9 defined types.
    pub event: String,

    /// ISO 8601 UTC timestamp.
    pub timestamp: DateTime<Utc>,

    /// Service name emitting the event.
    pub service: String,

    /// Tenant context (string from claims, not a UUID).
    pub tenant_id: Option<String>,

    /// Subject user ID (string).
    pub user_id: Option<String>,

    /// Actor ID (for delegation/impersonation events).
    pub actor_id: Option<String>,

    /// Requested scopes (space-separated or comma-separated).
    pub scopes: String,

    /// How authorization was decided.
    pub decision_source: String,

    /// allowed / denied / revoked.
    pub result: String,

    /// Event-specific optional fields serialized as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// IP address from X-Forwarded-For or remote_addr.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,

    /// User-Agent header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// Unique request ID for end-to-end correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// JWT-specific: token version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_version: Option<u64>,

    /// JWT-specific: TTL in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u64>,

    /// JWT-specific: signing algorithm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,

    /// Error code for failed validations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Human-readable reason for failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Delegation-specific: type of delegation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegation_type: Option<String>,

    /// Delegation-specific: actor roles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_roles: Option<Vec<String>>,

    /// Delegation-specific: whether act claim is present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_claim_present: Option<bool>,

    /// Version bump: old version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_ver: Option<u64>,

    /// Version bump: new version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_ver: Option<u64>,

    /// Version bump: reason for bump.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_reason: Option<String>,

    /// Token binding mismatch: expected binding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_binding: Option<String>,

    /// Token binding mismatch: actual binding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_binding: Option<String>,

    /// HMAC signature for tamper evidence (set by emitter).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hmac_signature: Option<String>,

    /// Internal: logging level (not logged to output, used for routing).
    #[serde(skip_deserializing)]
    pub level: AuditLevel,

    /// Internal: internal event type (not logged to output, used for validation).
    #[serde(skip_deserializing)]
    pub event_type: AuditEventType,

    /// Internal: unique event ID (not logged to output).
    #[serde(skip_deserializing)]
    #[serde(serialize_with = "uuid_to_string", deserialize_with = "string_to_uuid")]
    pub event_id: Uuid,
}


fn uuid_to_string<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&uuid.to_string())
}

fn string_to_uuid<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<Uuid>().map_err(serde::de::Error::custom)
}
impl AuditLogEntry {
    /// Create a new audit log entry with a generated UUID and current timestamp.
    #[must_use]
    pub fn new(event_type: AuditEventType, service: impl Into<String>) -> AuditLogEntryBuilder {
        AuditLogEntryBuilder {
            entry: Self {
                event: event_type.as_str().to_string(),
                timestamp: Utc::now(),
                service: service.into(),
                tenant_id: None,
                user_id: None,
                actor_id: None,
                scopes: String::new(),
                decision_source: String::new(),
                result: String::new(),
                metadata: None,
                ip_address: None,
                user_agent: None,
                request_id: None,
                token_version: None,
                ttl: None,
                algorithm: None,
                error: None,
                reason: None,
                delegation_type: None,
                actor_roles: None,
                act_claim_present: None,
                old_ver: None,
                new_ver: None,
                version_reason: None,
                expected_binding: None,
                actual_binding: None,
                hmac_signature: None,
                level: event_type.default_level(),
                event_type,
                event_id: Uuid::now_v7(),
            },
        }
    }

    /// Validate the entry before writing: no raw JWT tokens, no PII fields.
    ///
    /// SECURITY: This check must happen before the entry is written to any log.
    /// HACK-832: Raw token strings must never appear in audit logs.
    /// HACK-837: Event type must be in the allowed set.
    /// HACK-835: All fields are JSON-escaped by serde_json, so injection is impossible
    ///           through normal field population. This function checks for red flags.
    pub fn validate(&self) -> Result<(), String> {
        // HACK-837: Validate event type
        if !is_valid_event_type(&self.event) {
            return Err(format!("Invalid event type: '{}'", self.event));
        }

        // HACK-832: Check for raw JWT token content (eyJ... pattern)
        // We check the serialized JSON for base64url tokens
        if let Ok(json) = serde_json::to_string(self) {
            // JWT tokens start with "eyJ" (base64url for '{"')
            // Check that no field value looks like a raw JWT
            if json.contains("eyJ") && json.contains("base64url") == false {
                // Heuristic: if the JSON contains "eyJ" in a value context, it might be a raw token
                // Only flag if it appears outside the event name
                let suspicious = json
                    .split('"')
                    .any(|part| part.len() > 50 && part.starts_with("eyJ"));
                if suspicious {
                    return Err("Raw JWT token detected in audit log entry — entry dropped".to_string());
                }
            }
        }

        // Truncate fields that could be too long (HACK-833 mitigation)
        if let Some(ref reason) = self.reason {
            if reason.len() > MAX_FIELD_LENGTH {
                return Err(format!(
                    "Reason field exceeds {} chars: truncated",
                    MAX_FIELD_LENGTH
                ));
            }
        }

        Ok(())
    }

    /// Create a new audit log entry with custom event name and actor.
    /// Matches the legacy 5-parameter API used by auth controllers.
    pub fn new_with_params(
        event_type: AuditEventType,
        event_name: impl Into<String>,
        user_id: impl Into<String>,
        actor: AuditActor,
        ip_address: impl Into<String>,
    ) -> Self {
        let actor_str = match actor {
            AuditActor::User => Some("user".to_string()),
            AuditActor::Admin => Some("admin".to_string()),
            AuditActor::System => Some("service_account".to_string()),
        };
        Self {
            event: event_name.into(),
            event_type,
            timestamp: Utc::now(),
            service: String::new(),
            tenant_id: None,
            user_id: Some(user_id.into()),
            actor_id: actor_str,
            scopes: String::new(),
            decision_source: String::new(),
            result: String::new(),
            metadata: None,
            ip_address: Some(ip_address.into()),
            user_agent: None,
            request_id: None,
            token_version: None,
            ttl: None,
            algorithm: None,
            error: None,
            reason: None,
            delegation_type: None,
            actor_roles: None,
            act_claim_present: None,
            old_ver: None,
            new_ver: None,
            version_reason: None,
            expected_binding: None,
            actual_binding: None,
            hmac_signature: None,
            level: event_type.default_level(),
            event_id: Uuid::now_v7(),
        }
    }

    /// Sanitize the entry by truncating long fields to prevent log bloat.
    pub fn sanitize(&mut self) {
        // Truncate string fields
        if let Some(ref mut reason) = self.reason {
            if reason.len() > MAX_FIELD_LENGTH {
                reason.truncate(MAX_FIELD_LENGTH);
            }
        }
        if let Some(ref mut error) = self.error {
            if error.len() > MAX_FIELD_LENGTH {
                error.truncate(MAX_FIELD_LENGTH);
            }
        }
        if let Some(ref mut ip) = self.ip_address {
            if ip.len() > MAX_FIELD_LENGTH {
                ip.truncate(MAX_FIELD_LENGTH);
            }
        }
        if let Some(ref mut ua) = self.user_agent {
            if ua.len() > MAX_FIELD_LENGTH {
                ua.truncate(MAX_FIELD_LENGTH);
            }
        }
        // Truncate metadata string values
        if let Some(ref mut meta) = self.metadata {
            sanitize_value(meta, MAX_FIELD_LENGTH);
        }
    }

    /// Serialize the entry to a JSON string.
    ///
    /// This uses serde_json which handles all JSON escaping automatically,
    /// preventing log injection attacks (HACK-835).
    #[must_use]
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize with a custom serializer that excludes internal fields.
    #[must_use]
    pub fn to_log_json(&self) -> Result<String, serde_json::Error> {
        // Create a log-friendly version that only includes fields meant for output
        let log_entry = AuditLogOutput {
            event: self.event.clone(),
            timestamp: self.timestamp,
            service: self.service.clone(),
            tenant_id: self.tenant_id.clone(),
            user_id: self.user_id.clone(),
            actor_id: self.actor_id.clone(),
            scopes: self.scopes.clone(),
            decision_source: self.decision_source.clone(),
            result: self.result.clone(),
            metadata: self.metadata.clone(),
            ip_address: self.ip_address.clone(),
            user_agent: self.user_agent.clone(),
            request_id: self.request_id.clone(),
            token_version: self.token_version,
            ttl: self.ttl,
            algorithm: self.algorithm.clone(),
            error: self.error.clone(),
            reason: self.reason.clone(),
            delegation_type: self.delegation_type.clone(),
            actor_roles: self.actor_roles.clone(),
            act_claim_present: self.act_claim_present,
            old_ver: self.old_ver,
            new_ver: self.new_ver,
            version_reason: self.version_reason.clone(),
            expected_binding: self.expected_binding.clone(),
            actual_binding: self.actual_binding.clone(),
        };
        serde_json::to_string(&log_entry)
    }

    #[must_use]
    pub fn tenant_id(mut self, id: impl Into<String>) -> Self {
        self.tenant_id = Some(id.into());
        self
    }

    #[must_use]
    pub fn user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }

    #[must_use]
    pub fn actor_id(mut self, id: impl Into<String>) -> Self {
        self.actor_id = Some(id.into());
        self
    }

    #[must_use]
    pub fn scopes(mut self, scopes: impl Into<String>) -> Self {
        self.scopes = scopes.into();
        self
    }

    #[must_use]
    pub fn decision_source(mut self, source: impl Into<String>) -> Self {
        self.decision_source = source.into();
        self
    }

    #[must_use]
    pub fn result(mut self, result: impl Into<String>) -> Self {
        self.result = result.into();
        self
    }

    #[must_use]
    pub fn ip_address(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    #[must_use]
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    #[must_use]
    pub fn request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    #[must_use]
    pub fn token_version(mut self, ver: u64) -> Self {
        self.token_version = Some(ver);
        self
    }

    #[must_use]
    pub fn ttl(mut self, seconds: u64) -> Self {
        self.ttl = Some(seconds);
        self
    }

    #[must_use]
    pub fn algorithm(mut self, algo: impl Into<String>) -> Self {
        self.algorithm = Some(algo.into());
        self
    }

    #[must_use]
    pub fn error(mut self, err: impl Into<String>) -> Self {
        self.error = Some(err.into());
        self
    }

    #[must_use]
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    #[must_use]
    pub fn metadata(mut self, meta: serde_json::Value) -> Self {
        self.metadata = Some(meta);
        self
    }

    #[must_use]
    pub fn delegation_type(mut self, dtype: impl Into<String>) -> Self {
        self.delegation_type = Some(dtype.into());
        self
    }

    #[must_use]
    pub fn actor_roles(mut self, roles: Vec<String>) -> Self {
        self.actor_roles = Some(roles);
        self
    }

    #[must_use]
    pub fn act_claim_present(mut self, present: bool) -> Self {
        self.act_claim_present = Some(present);
        self
    }

    #[must_use]
    pub fn old_ver(mut self, ver: u64) -> Self {
        self.old_ver = Some(ver);
        self
    }

    #[must_use]
    pub fn new_ver(mut self, ver: u64) -> Self {
        self.new_ver = Some(ver);
        self
    }

    #[must_use]
    pub fn version_reason(mut self, reason: impl Into<String>) -> Self {
        self.version_reason = Some(reason.into());
        self
    }

    #[must_use]
    pub fn expected_binding(mut self, binding: impl Into<String>) -> Self {
        self.expected_binding = Some(binding.into());
        self
    }

    #[must_use]
    pub fn actual_binding(mut self, binding: impl Into<String>) -> Self {
        self.actual_binding = Some(binding.into());
        self
    }

    #[must_use]
    pub fn level(mut self, level: AuditLevel) -> Self {
        self.level = level;
        self
    }
}

/// Minimal output structure — only fields meant for log consumers.
/// Internal fields (level, event_type, event_id, hmac_signature) excluded.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditLogOutput {
    pub event: String,
    pub timestamp: DateTime<Utc>,
    pub service: String,
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub actor_id: Option<String>,
    pub scopes: String,
    pub decision_source: String,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_version: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_claim_present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_ver: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_ver: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_binding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_binding: Option<String>,
}

/// Recursively sanitize a JSON value — truncate string values that exceed limit.
fn sanitize_value(value: &mut serde_json::Value, limit: usize) {
    match value {
        serde_json::Value::String(s) => {
            if s.len() > limit {
                *s = s[..limit].to_string();
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                sanitize_value(item, limit);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                sanitize_value(item, limit);
            }
        }
        _ => {}
    }
}

// ─── Builder Pattern ─────────────────────────────────────────────────────────

/// Builder for constructing `AuditLogEntry` instances.
///
/// Provides a fluent API for populating all fields.
pub struct AuditLogEntryBuilder {
    entry: AuditLogEntry,
}

impl AuditLogEntryBuilder {
    pub fn new(event_type: AuditEventType, service: impl Into<String>) -> Self {
        Self {
            entry: AuditLogEntry {
                event: event_type.as_str().to_string(),
                timestamp: Utc::now(),
                service: service.into(),
                tenant_id: None,
                user_id: None,
                actor_id: None,
                scopes: String::new(),
                decision_source: String::new(),
                result: String::new(),
                metadata: None,
                ip_address: None,
                user_agent: None,
                request_id: None,
                token_version: None,
                ttl: None,
                algorithm: None,
                error: None,
                reason: None,
                delegation_type: None,
                actor_roles: None,
                act_claim_present: None,
                old_ver: None,
                new_ver: None,
                version_reason: None,
                expected_binding: None,
                actual_binding: None,
                hmac_signature: None,
                level: event_type.default_level(),
                event_type,
                event_id: Uuid::now_v7(),
            },
        }
    }

    #[must_use]
    pub fn tenant_id(mut self, id: impl Into<String>) -> Self {
        self.entry.tenant_id = Some(id.into());
        self
    }

    #[must_use]
    pub fn user_id(mut self, id: impl Into<String>) -> Self {
        self.entry.user_id = Some(id.into());
        self
    }

    #[must_use]
    pub fn actor_id(mut self, id: impl Into<String>) -> Self {
        self.entry.actor_id = Some(id.into());
        self
    }

    #[must_use]
    pub fn scopes(mut self, scopes: impl Into<String>) -> Self {
        self.entry.scopes = scopes.into();
        self
    }

    #[must_use]
    pub fn decision_source(mut self, source: impl Into<String>) -> Self {
        self.entry.decision_source = source.into();
        self
    }

    #[must_use]
    pub fn result(mut self, result: impl Into<String>) -> Self {
        self.entry.result = result.into();
        self
    }

    #[must_use]
    pub fn ip_address(mut self, ip: impl Into<String>) -> Self {
        self.entry.ip_address = Some(ip.into());
        self
    }

    #[must_use]
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.entry.user_agent = Some(ua.into());
        self
    }

    #[must_use]
    pub fn request_id(mut self, id: impl Into<String>) -> Self {
        self.entry.request_id = Some(id.into());
        self
    }

    #[must_use]
    pub fn token_version(mut self, ver: u64) -> Self {
        self.entry.token_version = Some(ver);
        self
    }

    #[must_use]
    pub fn ttl(mut self, seconds: u64) -> Self {
        self.entry.ttl = Some(seconds);
        self
    }

    #[must_use]
    pub fn algorithm(mut self, algo: impl Into<String>) -> Self {
        self.entry.algorithm = Some(algo.into());
        self
    }

    #[must_use]
    pub fn error(mut self, err: impl Into<String>) -> Self {
        self.entry.error = Some(err.into());
        self
    }

    #[must_use]
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.entry.reason = Some(reason.into());
        self
    }

    #[must_use]
    pub fn metadata(mut self, meta: serde_json::Value) -> Self {
        self.entry.metadata = Some(meta);
        self
    }

    #[must_use]
    pub fn delegation_type(mut self, dtype: impl Into<String>) -> Self {
        self.entry.delegation_type = Some(dtype.into());
        self
    }

    #[must_use]
    pub fn actor_roles(mut self, roles: Vec<String>) -> Self {
        self.entry.actor_roles = Some(roles);
        self
    }

    #[must_use]
    pub fn act_claim_present(mut self, present: bool) -> Self {
        self.entry.act_claim_present = Some(present);
        self
    }

    #[must_use]
    pub fn old_ver(mut self, ver: u64) -> Self {
        self.entry.old_ver = Some(ver);
        self
    }

    #[must_use]
    pub fn new_ver(mut self, ver: u64) -> Self {
        self.entry.new_ver = Some(ver);
        self
    }

    #[must_use]
    pub fn version_reason(mut self, reason: impl Into<String>) -> Self {
        self.entry.version_reason = Some(reason.into());
        self
    }

    #[must_use]
    pub fn expected_binding(mut self, binding: impl Into<String>) -> Self {
        self.entry.expected_binding = Some(binding.into());
        self
    }

    #[must_use]
    pub fn actual_binding(mut self, binding: impl Into<String>) -> Self {
        self.entry.actual_binding = Some(binding.into());
        self
    }

    /// Set the logging level explicitly (overrides the default from event_type).
    #[must_use]
    pub fn level(mut self, level: AuditLevel) -> Self {
        self.entry.level = level;
        self
    }

    /// Build the entry, running validation and sanitization.
    pub fn build(mut self) -> Result<AuditLogEntry, String> {
        self.entry.sanitize();
        self.entry.validate()?;
        Ok(self.entry)
    }
}
