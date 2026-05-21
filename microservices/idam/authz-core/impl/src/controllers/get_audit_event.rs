use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::get_audit_event::{Request, Response};

/// Handler for Get Audit Event
#[handler(GetAuditEventController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    Response {
        id: req.data.id,
        event_type: "".to_string(),
        event_action: "".to_string(),
        actor: "".to_string(),
        ip_address: "".to_string(),
        hmac_signature: None,
        timestamp: "".to_string(),
        metadata: None,
        org_id: None,
        session_id: None,
        severity: None,
        target_id: None,
        target_type: None,
        tenant_id: req.data.x_tenant_id,
        user_agent: None,
        user_id: None,
    }
}
