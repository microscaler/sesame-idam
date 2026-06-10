use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::validate_personal_api_key::{Request, Response};

/// Handler for Validate Personal Api Key.
#[handler(ValidatePersonalApiKeyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLevel, AuditLogEntry};
    use uuid::Uuid;

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "api-keys")
        .tenant_id(req.inner.tenant_id.clone())
        .metadata(serde_json::json!({
            "valid": req.inner.valid,
        }))
        .build();

    let entry = entry.and_then(|e| {
        let level = if req.inner.valid.unwrap_or(false) {
            AuditLevel::Info
        } else {
            AuditLevel::Warn
        };
        Ok(e.user_id(
            req.inner.user_id
                .parse::<Uuid>()
                .ok()
                .map(|u| u.to_string())
                .unwrap_or_default(),
        )
        .level(level)
        .build()?)
    });

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        valid: req.inner.valid.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
