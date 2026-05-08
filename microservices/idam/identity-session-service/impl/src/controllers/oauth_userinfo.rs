// Implementation stub for handler 'oauth_userinfo'
// OIDC User Info endpoint
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::oauth_userinfo::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(OauthUserinfoController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Validate Bearer token from Authorization header
    // TODO: Extract user claims from token
    // TODO: Return user profile matching OIDC standard claims
    
    Response {
        sub: None,
        name: None,
        email: None,
        email_verified: None,
        preferred_username: None,
        picture: None,
        updated_at: None,
    }
}
