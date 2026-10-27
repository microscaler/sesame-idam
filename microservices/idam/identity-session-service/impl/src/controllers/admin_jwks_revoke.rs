/// Admin handler: POST /admin/jwks/revoke
/// Revoke a signing key by `kid`: removes it from JWKS and drops the private key (HACK-101).
///
/// A platform administrator only (`sesame_common::platform_admin`): the signing keys belong to the whole deployment,
/// not to one tenant, and a valid token alone was enough until 27 Oct 2026. Every attempt is audited. With the shared
/// keyset (ADR-006 file mode) the key manager refuses: the kid is removed from the keyset Secret instead.
use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use serde_json::{json, Value};
use sesame_common::audit::{AuditEventType, AuditLogEntry};
use sesame_idam_identity_session_service_gen::handlers::admin_jwks_revoke::Request;

use crate::audit::EMITTER;
use crate::key_manager::{KeyError, KEY_MANAGER};

fn answer(kid: &str, success: bool, message: impl Into<String>) -> HttpJson<Value> {
    HttpJson::new(
        200,
        json!({ "kid": kid, "message": message.into(), "success": success }),
    )
}

fn audit(user: Option<&str>, tenant: Option<&str>, result: &str, reason: &str) {
    let mut entry =
        AuditLogEntry::new(AuditEventType::ValidationFailed, "identity-session-service")
            .decision_source("platform_admin")
            .result(result.to_string())
            .reason(reason.to_string());
    if let Some(u) = user {
        entry = entry.user_id(u.to_string());
    }
    if let Some(t) = tenant {
        entry = entry.tenant_id(t.to_string());
    }
    match entry.build() {
        Ok(e) => EMITTER.emit(e),
        Err(e) => tracing::warn!(error = %e, "admin_jwks_revoke: audit entry build failed"),
    }
}

/// Revoke a JWKS key by its `kid`.
///
/// 401/403 unless the caller is a platform administrator; otherwise success, `key_not_found`, or the key manager's
/// refusal (file mode) as `success: false`.
#[handler(AdminRevokeKeyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<Value> {
    let kid = req.data.kid.clone();
    let claims = &req.jwt_claims;
    let sub = claims
        .as_ref()
        .and_then(|c| c.get("sub"))
        .and_then(Value::as_str);
    let admin = match sesame_common::platform_admin::platform_admin(claims) {
        Ok(a) => a,
        Err(refused) => {
            tracing::warn!(kid = %kid, status = refused.status, "admin: key revocation refused (not a platform administrator)");
            audit(
                sub,
                None,
                "denied",
                "admin_jwks_revoke: not a platform administrator",
            );
            return refused;
        }
    };
    let user = admin.user_id.to_string();
    // Not entered (may_tracing ADR-0001): a may coroutine can resume on another thread after a yield.
    let _span = tracing::span!(tracing::Level::INFO, "key.revoke.admin", kid = %kid, admin = %user);

    let result = match KEY_MANAGER.write() {
        Ok(mut guard) => guard.revoke_key(&kid),
        Err(poison) => {
            tracing::error!(error = ?poison, "KEY_MANAGER write lock poisoned");
            Err(KeyError::RevocationFailed("lock poisoned".to_string()))
        }
    };

    match result {
        Ok(()) => {
            tracing::info!(kid = %kid, admin = %user, "admin: key revoked");
            audit(
                Some(&user),
                Some(&admin.tenant),
                "allowed",
                "admin_jwks_revoke: key revoked",
            );
            answer(&kid, true, "Key revoked successfully")
        }
        Err(KeyError::KeyNotFound(_)) => {
            tracing::warn!(kid = %kid, admin = %user, "admin: revocation requested for unknown key");
            audit(
                Some(&user),
                Some(&admin.tenant),
                "allowed",
                "admin_jwks_revoke: key not found",
            );
            answer(&kid, false, "Key not found")
        }
        Err(KeyError::RevocationFailed(msg)) => {
            tracing::warn!(kid = %kid, admin = %user, error = %msg, "admin: revocation refused");
            audit(
                Some(&user),
                Some(&admin.tenant),
                "allowed",
                "admin_jwks_revoke: refused by the key manager",
            );
            answer(&kid, false, format!("Key revocation failed: {msg}"))
        }
        Err(e) => {
            tracing::error!(kid = %kid, admin = %user, error = ?e, "admin: revocation failed unexpectedly");
            answer(&kid, false, format!("Revocation failed: {e}"))
        }
    }
}
