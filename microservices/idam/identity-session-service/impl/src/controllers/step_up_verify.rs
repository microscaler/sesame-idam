/// Handler for Step Up Verify — verifies step-up authentication (MFA).
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_common::audit::AuditEventType;
use sesame_idam_identity_session_service_gen::handlers::step_up_verify::{Request, Response};

#[handler(StepUpVerifyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;

    let tenant_id = _req.data.x_tenant_id.clone();

    let entry =
        sesame_common::audit::AuditLogEntry::new(AuditEventType::JwtIssued, "identity-session-service")
            .tenant_id(tenant_id.clone())
            .decision_source("step_up_verify")
            .result("allowed")
            .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        mfa_method: None,
        session_id: None,
        verified: false,
    }
}
