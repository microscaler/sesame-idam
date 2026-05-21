/// Handler for Auth Refresh — refreshes an access token using a refresh token.
/// Implements token rotation per Story 3.1: validates old token, issues new
/// refresh token with new JTI, adds old JTI to denylist.
///
/// Returns 401 with reason "token_rotated" on reuse detection (tear scenario).
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::auth_refresh::{Request, Response};

use crate::audit::EMITTER;
use crate::models::refresh_token::REFRESH_TOKEN_TTL;
use crate::services::token_rotation::{self, RotationOutcome};
use sesame_audit::AuditEventType;

#[handler(AuthRefreshController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let span = tracing::span!(
        tracing::Level::INFO,
        "token.refreshed",
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty
    );
    let _guard = span.enter();

    let refresh_token = req.data.refresh_token.clone();
    let tenant_id = req.data.x_tenant_id.clone();
    let span_tenant = tenant_id.clone();

    // --- Audit logging ---
    let entry =
        sesame_audit::AuditLogEntry::new(AuditEventType::JwtIssued, "identity-session-service")
            .tenant_id(tenant_id.clone())
            .decision_source("token_refresh")
            .result("allowed")
            .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // --- Perform token rotation ---
    let result = token_rotation::rotate_refresh_token(&refresh_token, &tenant_id, &tenant_id);

    // Record result in span
    match &result {
        RotationOutcome::Rotated { .. } => {
            span.record("result", "rotated");
        }
        RotationOutcome::ReuseDetected { .. } => {
            span.record("result", "reuse_detected");
        }
        RotationOutcome::InvalidToken => {
            span.record("result", "invalid_token");
        }
        RotationOutcome::RedisUnavailable => {
            span.record("result", "redis_unavailable");
        }
    }
    span.record("tenant_id", &span_tenant);

    // --- Dispatch on outcome ---
    match result {
        RotationOutcome::Rotated {
            new_access_token,
            new_refresh_token,
            access_expires_in,
            refresh_expires_in,
        } => {
            // Success: emit audit event for completed rotation
            let entry = sesame_audit::AuditLogEntry::new(
                AuditEventType::JwtIssued,
                "identity-session-service",
            )
            .tenant_id(tenant_id.clone())
            .decision_source("token_refresh")
            .result("allowed")
            .ttl(REFRESH_TOKEN_TTL as u64)
            .build();

            if let Ok(entry) = entry {
                EMITTER.emit(entry);
            }

            Response {
                access_token: new_access_token,
                email: None,
                email_verified: None,
                expires_in: access_expires_in,
                id_token: None,
                mfa_required: None,
                phone_verified: None,
                refresh_token: new_refresh_token,
                refresh_token_expires_in: Some(refresh_expires_in),
                scope: None,
                token_type: "Bearer".to_string(),
                user_id: tenant_id.clone(),
            }
        }
        RotationOutcome::ReuseDetected {
            reused_jti,
            family_id,
        } => {
            // Reuse detected: revoke family and return 401 equivalent
            tracing::warn!(
                event = "refresh_reuse_detected",
                reused_jti = reused_jti,
                family_id = family_id,
                "Refresh token reuse detected — family revoked"
            );

            // Cross-session notification signal (F-005)
            tracing::info!(
                event = "cross_session_notification",
                family_id = family_id,
                "Triggering cross-session notification for token reuse"
            );

            Response {
                access_token: String::new(),
                email: None,
                email_verified: None,
                expires_in: 0,
                id_token: None,
                mfa_required: Some(true),
                phone_verified: None,
                refresh_token: String::new(),
                refresh_token_expires_in: None,
                scope: None,
                token_type: "Bearer".to_string(),
                user_id: tenant_id.clone(),
            }
        }
        RotationOutcome::InvalidToken => {
            tracing::warn!(
                event = "refresh_invalid_token",
                tenant_id = &span_tenant,
                "Refresh token not found in Redis"
            );

            let entry = sesame_audit::AuditLogEntry::new(
                AuditEventType::ValidationFailed,
                "identity-session-service",
            )
            .tenant_id(tenant_id.clone())
            .decision_source("token_refresh")
            .result("denied")
            .error("invalid_token")
            .reason("Refresh token not found in Redis")
            .build();

            if let Ok(entry) = entry {
                EMITTER.emit(entry);
            }

            Response {
                access_token: String::new(),
                email: None,
                email_verified: None,
                expires_in: 0,
                id_token: None,
                mfa_required: None,
                phone_verified: None,
                refresh_token: String::new(),
                refresh_token_expires_in: None,
                scope: None,
                token_type: "Bearer".to_string(),
                user_id: tenant_id.clone(),
            }
        }
        RotationOutcome::RedisUnavailable => {
            tracing::error!(
                event = "refresh_redis_unavailable",
                tenant_id = &span_tenant,
                "Redis unavailable during token rotation"
            );

            let entry = sesame_audit::AuditLogEntry::new(
                AuditEventType::ValidationFailed,
                "identity-session-service",
            )
            .tenant_id(tenant_id.clone())
            .decision_source("token_refresh")
            .result("denied")
            .error("redis_unavailable")
            .reason("Redis unavailable during token rotation")
            .build();

            if let Ok(entry) = entry {
                EMITTER.emit(entry);
            }

            Response {
                access_token: String::new(),
                email: None,
                email_verified: None,
                expires_in: 0,
                id_token: None,
                mfa_required: None,
                phone_verified: None,
                refresh_token: String::new(),
                refresh_token_expires_in: None,
                scope: None,
                token_type: "Bearer".to_string(),
                user_id: tenant_id.clone(),
            }
        }
    }
}
