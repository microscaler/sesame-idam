// Implementation stub for handler 'admin_issue_token'
// Admin issues access token directly (bypass login)
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::admin_issue_token::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(AdminIssueTokenController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let user_id = req.inner.user_id;
    let application_id = req.inner.application_id;
    
    // TODO: Verify request is from admin context
    // TODO: Fetch user from DB
    // TODO: Sign JWT with user claims + application scope
    // TODO: Return tokens
    
    Response {
        access_token: "direct-issued-jwt".to_string(),
        expires_in: 3600,
        refresh_token: "direct-issued-refresh".to_string(),
        refresh_token_expires_in: Some(86400),
        token_type: "Bearer".to_string(),
        user_id: user_id,
        email: None,
        email_verified: None,
        mfa_required: None,
        phone_verified: None,
    }
}
