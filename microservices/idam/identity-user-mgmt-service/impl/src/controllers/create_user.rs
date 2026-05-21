use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::create_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Create User.
#[handler(CreateUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Span: user.created
    let span = tracing::span!(
        tracing::Level::INFO,
        "user.created",
        tenant_id = tracing::field::Empty,
        result = tracing::field::Empty
    );
    let _guard = span.enter();
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let user_id = Uuid::new_v7();
    
    let mut event = AuditEvent::new(
        AuditEventType::UserManagement,
        "user_created",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = Some(user_id);
    event.metadata = serde_json::json!({
        "email": req.inner.email,
        "username": req.inner.username,
    }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: INSERT INTO users WITH tenant_id, email, username, password_hash (bcrypt)
    // TODO: Send email confirmation if not already verified
    
    Response {
        id: user_id.to_string(),
        email: req.inner.email.clone(),
        username: req.inner.username.clone(),
        first_name: req.inner.first_name.clone(),
        last_name: req.inner.last_name.clone(),
        phone: req.inner.phone.clone(),
        email_verified: false,
        phone_verified: false,
        auth_methods: vec![],
        mfa_enabled: false,
        tenant_id: req.inner.tenant_id,
        org_id: req.inner.org_id,
    }
}
