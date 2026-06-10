use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::fetch_archived_api_keys::{Request, Response};

/// Handler for Fetch Archived Api Keys.
#[handler(FetchArchivedApiKeysController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};
    use uuid::Uuid;

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "api-keys")
        .tenant_id(req.inner.tenant_id.clone())
        .build();

    let entry = entry.and_then(|e| {
        Ok(e.user_id(
            req.inner.user_id
                .parse::<Uuid>()
                .ok()
                .map(|u| u.to_string())
                .unwrap_or_default(),
        )
        .build()?)
    });

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
        keys: Vec::new(),
    }
}
