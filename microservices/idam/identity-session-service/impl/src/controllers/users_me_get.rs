/// Handler for Users Me Get — retrieves the authenticated user's profile.
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_common::audit::AuditEventType;
use sesame_idam_identity_session_service_gen::handlers::users_me_get::{Request, Response};

#[handler(UsersMeGetController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;

    let tenant_id = _req.data.x_tenant_id.clone();

    let entry =
        sesame_common::audit::AuditLogEntry::new(AuditEventType::JwtValidated, "identity-session-service")
            .tenant_id(tenant_id.clone())
            .decision_source("users_me_get")
            .result("allowed")
            .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        email: None,
        email_verified: None,
        first_name: None,
        last_name: None,
        name: None,
        org_id: None,
        org_name: None,
        phone_number: None,
        phone_verified: None,
        picture_url: None,
        preferred_username: None,
        properties: None,
        sub: None,
        updated_at: None,
        user_id: None,
        user_permissions: None,
        user_role: None,
        username: None,
    }
}
