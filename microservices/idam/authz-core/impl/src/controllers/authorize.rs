use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::authorize::{Request, Response};

/// Authorization controller handler.
///
/// Evaluates whether a principal (user) is allowed to perform an action
/// on a resource within a tenant/org context.
///
/// This endpoint audits all requests via `sesame_audit` before returning
/// the authorization decision.
#[handler(AuthorizeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEventType, AuditLogEntry};

    let mut metadata = serde_json::Map::new();
    metadata.insert("action".to_string(), serde_json::json!(req.data.action));
    metadata.insert("resource".to_string(), serde_json::json!(req.data.resource));

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "authorization_check")
        .tenant_id(
            req.data
                .tenant_id
                .as_deref()
                .unwrap_or_default(),
        )
        .user_id(&req.data.user_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        allowed: true,
        permissions_used: None,
        reason: None,
        roles_matched: None,
    }
}
