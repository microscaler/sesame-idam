use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::delete_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Delete Org.
#[handler(DeleteOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Span: org.deleted
    let _span = tracing::span!(
        tracing::Level::INFO,
        "org.deleted",
        tenant_id = tracing::field::Empty,
        result = tracing::field::Empty
    );
    // Not entered (may_tracing ADR-0001): a may coroutine can resume on another thread after a yield, and a span
    // entered on one thread and exited on another leaks its context there for good (2026-10-23 memory incident).
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLevel, AuditLogEntry};

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "org-mgmt")
        .tenant_id(req.inner.tenant_id.clone())
        .build();

    let entry = entry.and_then(|e| Ok(e.level(AuditLevel::Error).build()?));

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
