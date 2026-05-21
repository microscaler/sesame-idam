use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::set_principal_attribute::{Request, Response};
use sesame_token_versioning::BumpReason;

/// Handler for Set Principal Attribute - sets a metadata attribute on a principal.
///
/// Emits an audit event and publishes a version bump push invalidation event
/// via Redis pub/sub (Story 5.4).
#[handler(SetPrincipalAttributeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEventType, AuditLogEntry};

    let mut metadata = serde_json::Map::new();
    metadata.insert("key".to_string(), serde_json::json!(&req.data.key));
    metadata.insert("value_set".to_string(), serde_json::json!(!req.data.value.is_empty()));

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "attribute_updated")
        .tenant_id(&req.data.tenant_id)
        .user_id(&req.data.user_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // Story 5.4: Publish push invalidation event for principal attribute change
    let tenant_id = req.data.tenant_id.clone();
    if let Some(publisher) = &*crate::audit::PUBLISHER {
        publisher.publish_tenant(
            &tenant_id,
            0, // version is managed by VersionStore
            BumpReason::PrincipalAttributeModified,
        );
    }

    // In a production implementation, this would:
    // 1. Store the attribute in the principal's metadata table
    // 2. Invalidate cached effective permissions
    // 3. Optionally notify dependent services via webhook

    Response {
        error: String::new(),
        error_description: None,
    }
}
