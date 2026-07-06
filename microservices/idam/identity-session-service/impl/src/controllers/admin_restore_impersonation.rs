use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_common::audit::AuditEventType;
use sesame_idam_identity_session_service_gen::handlers::admin_restore_impersonation::{
    Request, Response,
};

#[handler(AdminRestoreImpersonationController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;

    let tenant_id = req.data.x_tenant_id.clone();
    let admin_user_id = req.data.admin_user_id.clone();

    let entry = sesame_common::audit::AuditLogEntry::new(
        AuditEventType::Delegation,
        "identity-session-service",
    )
    .user_id(admin_user_id.clone())
    .tenant_id(tenant_id.clone())
    .decision_source("admin_restore_impersonation")
    .result("allowed")
    .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        access_token: "restored-jwt".to_string(),
        impersonated_user_id: admin_user_id.clone(),
        original_user_id: admin_user_id,
        refresh_token: "restored-refresh".to_string(),
    }
}
