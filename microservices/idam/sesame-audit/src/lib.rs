//! # sesame-audit
//!
//! Shared audit event emission interface for Sesame-IDAM microservices.
//!
//! This crate provides:
//! - `AuditEventType` — enum of event categories
//! - `AuditActor` — enum of actor types
//! - `AuditSeverity` — enum of severity levels
//! - `AuditEvent` — the core event struct
//! - `AuditEmitter` — thread-safe event emitter with HMAC signing
//! - `AuditLogger` — console logger (default impl)
//!
//! Usage:
//! ```rust
//! use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity, AuditEmitter};
//! use uuid::Uuid;
//!
//! let emitter = AuditEmitter::new(Some(b"my-secret-key"));
//! let tenant_id = Uuid::new_v4();
//! let mut event = AuditEvent::new(
//!     AuditEventType::Authentication,
//!     "login_success",
//!     tenant_id,
//!     AuditActor::User,
//!     "192.168.1.1",
//! );
//! emitter.emit(&mut event);
//! ```

pub use chrono::{DateTime, Utc};
pub use uuid::Uuid;

// ─── Enums ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    #[default]
    Authentication,
    Authorization,
    UserManagement,
    SessionManagement,
    Organization,
    ApiKey,
    System,
    Compliance,
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditEventType::Authentication => write!(f, "authentication"),
            AuditEventType::Authorization => write!(f, "authorization"),
            AuditEventType::UserManagement => write!(f, "user_management"),
            AuditEventType::SessionManagement => write!(f, "session_management"),
            AuditEventType::Organization => write!(f, "organization"),
            AuditEventType::ApiKey => write!(f, "api_key"),
            AuditEventType::System => write!(f, "system"),
            AuditEventType::Compliance => write!(f, "compliance"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuditActor {
    #[default]
    User,
    System,
    Admin,
    ServiceAccount,
    ApiKey,
}

impl std::fmt::Display for AuditActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditActor::User => write!(f, "user"),
            AuditActor::System => write!(f, "system"),
            AuditActor::Admin => write!(f, "admin"),
            AuditActor::ServiceAccount => write!(f, "service_account"),
            AuditActor::ApiKey => write!(f, "api_key"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for AuditSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditSeverity::Info => write!(f, "info"),
            AuditSeverity::Warning => write!(f, "warning"),
            AuditSeverity::Error => write!(f, "error"),
            AuditSeverity::Critical => write!(f, "critical"),
        }
    }
}

// ─── Event Struct ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AuditEvent {
    /// Unique event identifier (generated on emit)
    pub id: Uuid,
    /// Event category
    pub event_type: AuditEventType,
    /// Specific action (e.g., "`login_success`", "`token_rotate`", "`user_delete`")
    pub event_action: String,
    /// Tenant scope
    pub tenant_id: Uuid,
    /// Organization scope (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<Uuid>,
    /// Associated user ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    /// Actor type
    pub actor: AuditActor,
    /// Target entity ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<Uuid>,
    /// Target entity type (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    /// Severity level (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<AuditSeverity>,
    /// Event-specific structured details (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Client IP address
    pub ip_address: String,
    /// Client user agent (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// Associated session ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<Uuid>,
    /// HMAC for tamper-evident logging (set by emitter)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hmac_signature: Option<String>,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
}

impl AuditEvent {
    pub fn new(
        event_type: AuditEventType,
        event_action: impl Into<String>,
        tenant_id: Uuid,
        actor: AuditActor,
        ip_address: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            event_type,
            event_action: event_action.into(),
            tenant_id,
            org_id: None,
            user_id: None,
            actor,
            target_id: None,
            target_type: None,
            severity: None,
            metadata: None,
            ip_address: ip_address.into(),
            user_agent: None,
            session_id: None,
            hmac_signature: None,
            timestamp: Utc::now(),
        }
    }
}

// ─── Emitter Trait & Default Logger ──────────────────────────────────────────

pub trait AuditSink: Send + Sync {
    /// Emit an audit event (synchronous — callers must not block)
    fn emit(&self, event: &AuditEvent);
}

pub struct AuditLogger;

impl AuditSink for AuditLogger {
    fn emit(&self, event: &AuditEvent) {
        // Use tracing so callers can wire up their own sinks (slog, slog, etc.)
        tracing::info!(
            audit.id = %event.id,
            audit.event_type = %event.event_type,
            audit.event_action = %event.event_action,
            audit.tenant_id = %event.tenant_id,
            audit.actor = %event.actor,
            audit.severity = ?event.severity,
            audit.target_type = ?event.target_type,
            audit.ip_address = %event.ip_address,
            audit.timestamp = %event.timestamp,
            "audit_event"
        );

        // Also log the full event as JSON for structured logging consumers
        if let Ok(json) = serde_json::to_string(event) {
            tracing::debug!(json = json, "audit_event_detail");
        }
    }
}

/// Thread-safe audit event emitter with optional HMAC signing.
pub struct AuditEmitter {
    sink: arc_swap::ArcSwap<Box<dyn AuditSink>>,
    hmac_key: Option<Vec<u8>>,
}

impl Default for AuditEmitter {
    fn default() -> Self {
        Self::new(None)
    }
}

impl AuditEmitter {
    #[must_use]
    pub fn new(hmac_key: Option<&[u8]>) -> Self {
        Self {
            sink: arc_swap::ArcSwap::from_pointee(Box::new(AuditLogger)),
            hmac_key: hmac_key.map(<[u8]>::to_vec),
        }
    }

    /// Replace the active sink (e.g., swap in a file or DB sink for production)
    pub fn set_sink(&self, sink: Box<dyn AuditSink>) {
        use std::sync::Arc;
        self.sink.store(Arc::new(sink));
    }

    /// Emit an audit event
    pub fn emit(&self, event: &mut AuditEvent) {
        // Sign if HMAC key is configured
        if let Some(key) = &self.hmac_key {
            let message = format!(
                "{}:{}:{}:{}:{}",
                event.id, event.event_type, event.event_action, event.tenant_id, event.timestamp
            );
            let mac = Self::compute_hmac(key, &message);
            event.hmac_signature = Some(mac);
        }

        let sink = self.sink.load();
        sink.emit(event);
    }

    fn compute_hmac(key: &[u8], message: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let mut mac = Hmac::<Sha256>::new_from_slice(key)
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
}

/// Helper types for common audit actions
pub mod events {
    use super::{AuditEmitter, AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    /// Emit a login event
    pub fn login_success(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        user_id: Uuid,
        ip_address: impl Into<String>,
        session_id: Option<Uuid>,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::Authentication,
            "login_success",
            tenant_id,
            AuditActor::User,
            ip_address,
        );
        event.user_id = Some(user_id);
        event.session_id = session_id;
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }

    /// Emit a login failure event
    pub fn login_failure(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        user_id: Option<Uuid>,
        ip_address: impl Into<String>,
        reason: Option<&str>,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::Authentication,
            "login_failure",
            tenant_id,
            AuditActor::User,
            ip_address,
        );
        event.user_id = user_id;
        event.severity = Some(AuditSeverity::Warning);
        if let Some(reason) = reason {
            event.metadata = Some(serde_json::json!({ "reason": reason }));
        }
        emitter.emit(&mut event);
    }

    /// Emit a logout event
    pub fn logout(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        user_id: Uuid,
        ip_address: impl Into<String>,
        session_id: Option<Uuid>,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::SessionManagement,
            "logout",
            tenant_id,
            AuditActor::User,
            ip_address,
        );
        event.user_id = Some(user_id);
        event.session_id = session_id;
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }

    /// Emit a role assignment event
    pub fn role_assigned(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        actor_id: Uuid,
        role_name: &str,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::Authorization,
            "role_assigned",
            tenant_id,
            AuditActor::Admin,
            "internal".to_string(),
        );
        event.org_id = Some(org_id);
        event.user_id = Some(user_id);
        event.target_id = Some(actor_id);
        event.metadata = Some(serde_json::json!({ "role": role_name }));
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }

    /// Emit a role revocation event
    pub fn role_revoked(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        actor_id: Uuid,
        role_name: &str,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::Authorization,
            "role_revoked",
            tenant_id,
            AuditActor::Admin,
            "internal".to_string(),
        );
        event.org_id = Some(org_id);
        event.user_id = Some(user_id);
        event.target_id = Some(actor_id);
        event.metadata = Some(serde_json::json!({ "role": role_name }));
        event.severity = Some(AuditSeverity::Warning);
        emitter.emit(&mut event);
    }

    /// Emit a user creation event
    pub fn user_created(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        user_id: Uuid,
        actor_id: Uuid,
        email: &str,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::UserManagement,
            "user_created",
            tenant_id,
            AuditActor::Admin,
            "internal".to_string(),
        );
        event.user_id = Some(user_id);
        event.target_id = Some(actor_id);
        event.metadata = Some(serde_json::json!({ "email": email }));
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }

    /// Emit a user deletion event
    pub fn user_deleted(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        user_id: Uuid,
        actor_id: Uuid,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::UserManagement,
            "user_deleted",
            tenant_id,
            AuditActor::Admin,
            "internal".to_string(),
        );
        event.user_id = Some(user_id);
        event.target_id = Some(actor_id);
        event.severity = Some(AuditSeverity::Critical);
        emitter.emit(&mut event);
    }

    /// Emit a password change event
    pub fn password_changed(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        user_id: Uuid,
        ip_address: impl Into<String>,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::UserManagement,
            "password_changed",
            tenant_id,
            AuditActor::User,
            ip_address,
        );
        event.user_id = Some(user_id);
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }

    /// Emit an MFA enrollment event
    pub fn mfa_enrolled(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        user_id: Uuid,
        ip_address: impl Into<String>,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::UserManagement,
            "mfa_enrolled",
            tenant_id,
            AuditActor::User,
            ip_address,
        );
        event.user_id = Some(user_id);
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }

    /// Emit an API key creation event
    pub fn api_key_created(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        api_key_id: Uuid,
        user_id: Option<Uuid>,
        key_name: &str,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::ApiKey,
            "api_key_created",
            tenant_id,
            AuditActor::ApiKey,
            "internal".to_string(),
        );
        event.user_id = user_id;
        event.target_id = Some(api_key_id);
        event.metadata = Some(serde_json::json!({ "key_name": key_name }));
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }

    /// Emit an API key revocation event
    pub fn api_key_revoked(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        api_key_id: Uuid,
        user_id: Option<Uuid>,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::ApiKey,
            "api_key_revoked",
            tenant_id,
            AuditActor::ApiKey,
            "internal".to_string(),
        );
        event.user_id = user_id;
        event.target_id = Some(api_key_id);
        event.severity = Some(AuditSeverity::Warning);
        emitter.emit(&mut event);
    }

    /// Emit an org member addition event
    pub fn org_member_added(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        actor_id: Uuid,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::Organization,
            "org_member_added",
            tenant_id,
            AuditActor::Admin,
            "internal".to_string(),
        );
        event.org_id = Some(org_id);
        event.user_id = Some(user_id);
        event.target_id = Some(actor_id);
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }

    /// Emit an org member removal event
    pub fn org_member_removed(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        actor_id: Uuid,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::Organization,
            "org_member_removed",
            tenant_id,
            AuditActor::Admin,
            "internal".to_string(),
        );
        event.org_id = Some(org_id);
        event.user_id = Some(user_id);
        event.target_id = Some(actor_id);
        event.severity = Some(AuditSeverity::Warning);
        emitter.emit(&mut event);
    }

    /// Emit an SSO configuration change event
    pub fn sso_configured(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        org_id: Uuid,
        actor_id: Uuid,
        provider: &str,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::Organization,
            "sso_configured",
            tenant_id,
            AuditActor::Admin,
            "internal".to_string(),
        );
        event.org_id = Some(org_id);
        event.target_id = Some(actor_id);
        event.metadata = Some(serde_json::json!({ "provider": provider }));
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }

    /// Emit a token refresh event
    pub fn token_refreshed(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        user_id: Uuid,
        session_id: Uuid,
        ip_address: impl Into<String>,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::SessionManagement,
            "token_refreshed",
            tenant_id,
            AuditActor::User,
            ip_address,
        );
        event.user_id = Some(user_id);
        event.session_id = Some(session_id);
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }

    /// Emit a token revocation event
    pub fn token_revoked(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        user_id: Uuid,
        session_id: Uuid,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::SessionManagement,
            "token_revoked",
            tenant_id,
            AuditActor::Admin,
            "internal".to_string(),
        );
        event.user_id = Some(user_id);
        event.session_id = Some(session_id);
        event.severity = Some(AuditSeverity::Warning);
        emitter.emit(&mut event);
    }

    /// Emit a principal permission change event
    pub fn permission_changed(
        emitter: &AuditEmitter,
        tenant_id: Uuid,
        user_id: Uuid,
        actor_id: Uuid,
        action: &str,
        resource: Option<&str>,
    ) {
        let mut event = AuditEvent::new(
            AuditEventType::Authorization,
            action,
            tenant_id,
            AuditActor::Admin,
            "internal".to_string(),
        );
        event.user_id = Some(user_id);
        event.target_id = Some(actor_id);
        if let Some(resource) = resource {
            event.metadata = Some(serde_json::json!({ "resource": resource }));
        }
        event.severity = Some(AuditSeverity::Info);
        emitter.emit(&mut event);
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{AuditEmitter, AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    #[test]
    fn test_event_creation() {
        let event = AuditEvent::new(
            AuditEventType::Authentication,
            "login_success",
            Uuid::nil(),
            AuditActor::User,
            "127.0.0.1",
        );
        assert_eq!(event.event_type, AuditEventType::Authentication);
        assert_eq!(event.event_action, "login_success");
        assert_eq!(event.actor, AuditActor::User);
        assert_eq!(event.ip_address, "127.0.0.1");
    }

    #[test]
    fn test_hmac_signing() {
        let key = b"test-secret-key";
        let emitter = AuditEmitter::new(Some(key));

        let mut event = AuditEvent::new(
            AuditEventType::Authentication,
            "login_success",
            Uuid::nil(),
            AuditActor::User,
            "127.0.0.1",
        );
        emitter.emit(&mut event);

        assert!(event.hmac_signature.is_some());
        let sig = event.hmac_signature.as_ref().unwrap();
        // HMAC-SHA256 produces 64 hex chars
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn test_hmac_consistency() {
        let key = b"test-secret-key";
        let emitter = AuditEmitter::new(Some(key));

        let mut event1 = AuditEvent::new(
            AuditEventType::Authentication,
            "login_success",
            Uuid::nil(),
            AuditActor::User,
            "127.0.0.1",
        );
        emitter.emit(&mut event1);

        let mut event2 = event1.clone();
        emitter.emit(&mut event2);

        // Same inputs → same signature
        assert_eq!(event1.hmac_signature, event2.hmac_signature);
    }

    #[test]
    fn test_event_serialization() {
        let mut event = AuditEvent::new(
            AuditEventType::Authentication,
            "login_success",
            Uuid::new_v4(),
            AuditActor::User,
            "127.0.0.1",
        );
        event.user_id = Some(Uuid::new_v4());
        event.severity = Some(AuditSeverity::Info);

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_action, "login_success");
    }
}
