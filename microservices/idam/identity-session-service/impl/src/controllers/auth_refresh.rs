// BRRTRouter: user-owned

/// Handler for Auth Refresh — refreshes an access token using a refresh token.
/// Implements token rotation per Story 3.1: validates old token, issues new
/// refresh token with new JTI, adds old JTI to denylist.
use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::auth_refresh::{Request, Response};

use crate::audit::EMITTER;
use crate::models::refresh_token::REFRESH_TOKEN_TTL;
use crate::services::token_rotation::{self, RotationOutcome};
use sesame_common::audit::AuditEventType;

#[handler(AuthRefreshController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let span = tracing::span!(
        tracing::Level::INFO,
        "token.refreshed",
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
        result = tracing::field::Empty
    );
    let _guard = span.enter();

    let refresh_token = req.data.refresh_token.clone();
    let tenant_id = req.data.x_tenant_id.clone();
    let span_tenant = tenant_id.clone();

    let result = token_rotation::rotate_refresh_token(&refresh_token, &tenant_id);

    match &result {
        RotationOutcome::Rotated { user_id, .. } => {
            span.record("result", "rotated");
            span.record("user_id", user_id.as_str());
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

    match result {
        RotationOutcome::Rotated {
            new_access_token,
            new_refresh_token,
            access_expires_in,
            refresh_expires_in,
            user_id,
            scope,
        } => {
            let entry = sesame_common::audit::AuditLogEntry::new(
                AuditEventType::JwtIssued,
                "identity-session-service",
            )
            .tenant_id(tenant_id.clone())
            .user_id(user_id.clone())
            .decision_source("token_refresh")
            .result("allowed")
            .ttl(u64::from(REFRESH_TOKEN_TTL))
            .build();

            if let Ok(entry) = entry {
                EMITTER.emit(entry);
            }

            let body = Response {
                access_token: new_access_token,
                email: None,
                email_verified: None,
                expires_in: access_expires_in,
                id_token: None,
                mfa_required: None,
                phone_verified: None,
                refresh_token: new_refresh_token,
                refresh_token_expires_in: Some(refresh_expires_in),
                scope: Some(scope),
                token_type: "Bearer".to_string(),
                user_id,
            };
            match serde_json::to_value(body) {
                Ok(json) => HttpJson::ok(json),
                Err(e) => {
                    tracing::error!(error = %e, "auth_refresh: failed to serialize token response");
                    refresh_internal_error()
                }
            }
        }
        RotationOutcome::ReuseDetected {
            reused_jti,
            family_id,
        } => {
            tracing::warn!(
                event = "refresh_reuse_detected",
                reused_jti = reused_jti,
                family_id = family_id,
                "Refresh token reuse detected — family revoked"
            );

            tracing::info!(
                event = "cross_session_notification",
                family_id = family_id,
                "Triggering cross-session notification for token reuse"
            );

            refresh_unauthorized("invalid_grant", "Refresh token reuse detected")
        }
        RotationOutcome::InvalidToken => {
            tracing::warn!(
                event = "refresh_invalid_token",
                tenant_id = &span_tenant,
                "Refresh token not found in Redis"
            );

            let entry = sesame_common::audit::AuditLogEntry::new(
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

            refresh_unauthorized("invalid_grant", "Invalid or expired refresh token")
        }
        RotationOutcome::RedisUnavailable => {
            tracing::error!(
                event = "refresh_redis_unavailable",
                tenant_id = &span_tenant,
                "Redis unavailable during token rotation"
            );

            let entry = sesame_common::audit::AuditLogEntry::new(
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

            refresh_internal_error()
        }
    }
}

fn refresh_unauthorized(error: &str, error_description: &str) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        401,
        serde_json::json!({
            "error": error,
            "error_description": error_description,
        }),
    )
}

fn refresh_internal_error() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "An unexpected error occurred",
        }),
    )
}
