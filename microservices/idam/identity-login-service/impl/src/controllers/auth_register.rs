//! `POST /auth/register` — create a user with email + password.
//!
//! Hashes the password with argon2id, inserts the user (tenant-scoped,
//! `UNIQUE(tenant_id, email)`), and issues a real token pair. Returns:
//! - 201 `TokenResponse` on success
//! - 400 for weak passwords or duplicate email
//! - 500 on infrastructure failure

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::auth_register::{Request, Response};

use crate::audit::EMITTER;
use crate::services::password;
use crate::services::token_issuer;
use crate::services::user_service::UserService;
use sesame_common::audit::{AuditEventType, AuditLogEntry};

/// Default portal/client for direct browser registrations.
const DEFAULT_PORTAL: &str = "frontend";

#[handler(AuthRegisterController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let email = req.data.email.trim().to_lowercase();

    if let Err(reason) = password::validate_password_strength(&req.data.password) {
        return HttpJson::new(
            400,
            serde_json::json!({
                "error": "weak_password",
                "error_description": reason
            }),
        );
    }

    let exec = sesame_idam_database::db();

    // Pre-check duplicate email (the DB unique constraint is the failsafe).
    match UserService::find_by_tenant_and_email(&tenant_id, &email, exec) {
        Ok(Some(_)) => {
            return HttpJson::new(
                400,
                serde_json::json!({
                    "error": "email_in_use",
                    "error_description": "An account with this email already exists"
                }),
            );
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!(error = %e, "auth_register: duplicate check failed");
            return internal_error();
        }
    }

    let password_hash = match password::hash_password(&req.data.password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!(error = %e, "auth_register: hashing failed");
            return internal_error();
        }
    };

    let user_id = match UserService::create_user(
        &tenant_id,
        &email,
        &password_hash,
        req.data.phone.clone(),
        exec,
    ) {
        Ok(id) => id,
        Err(e) => {
            // Unique-constraint race: two concurrent registrations.
            let msg = e.to_string();
            if msg.contains("unique") || msg.contains("duplicate") {
                return HttpJson::new(
                    400,
                    serde_json::json!({
                        "error": "email_in_use",
                        "error_description": "An account with this email already exists"
                    }),
                );
            }
            tracing::error!(error = %e, "auth_register: user insert failed");
            return internal_error();
        }
    };

    let user_id_str = user_id.to_string();
    let tokens = match token_issuer::issue_tokens(
        &user_id_str,
        &tenant_id,
        DEFAULT_PORTAL,
        vec![],
        "customer",
        None,
    ) {
        Ok(tokens) => tokens,
        Err(e) => {
            tracing::error!(error = %e, "auth_register: token issuance failed");
            return internal_error();
        }
    };

    // Audit: user created + tokens issued
    match AuditLogEntry::new(AuditEventType::JwtIssued, "identity-login-service")
        .tenant_id(tenant_id.clone())
        .user_id(user_id.to_string())
        .decision_source("registration")
        .result("allowed")
        .build()
    {
        Ok(entry) => EMITTER.emit(entry),
        Err(e) => tracing::warn!(error = %e, "auth_register: audit entry build failed"),
    }

    let body = Response {
        access_token: tokens.access_token,
        entitlements_hash: None,
        entitlements_ref: None,
        expires_in: i32::try_from(tokens.expires_in).unwrap_or(300),
        id_token: None,
        mfa_required: Some(false),
        permissions: None,
        refresh_token: tokens.refresh_token,
        refresh_token_expires_in: Some(
            i32::try_from(tokens.refresh_expires_in).unwrap_or(i32::MAX),
        ),
        roles: Some(vec![]),
        scope: Some(tokens.scope),
        token_type: "Bearer".to_string(),
        token_version: i32::try_from(tokens.token_version).ok(),
        user_id: user_id_str,
    };

    match serde_json::to_value(&body) {
        Ok(json) => HttpJson::new(201, json),
        Err(e) => {
            tracing::error!(error = %e, "auth_register: response serialization failed");
            internal_error()
        }
    }
}

fn internal_error() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "An unexpected error occurred"
        }),
    )
}
